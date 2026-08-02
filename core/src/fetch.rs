//! Core-owned weight fetch (ADR-0001, #11). The core downloads pinned weights
//! into the platformdirs cache, streams byte-level progress, verifies each
//! file against its manifest SHA-256, and relays license status at fetch time.
//! The engine only ever receives local file paths and never touches the
//! network. The network is a trait boundary so tests fake it and CI never
//! downloads a model.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::registry::{License, ModelEntry, WeightFile};

/// Progress relayed to the interface as weights are made available. The
/// license event fires once per model, before any bytes move (relayed, not
/// certified).
#[derive(Debug, Clone, PartialEq)]
pub enum FetchEvent {
    /// The model's license status, relayed up front.
    License { model_id: String, license: License },
    /// A file was already cached and hash-verified; no download happened.
    Cached { file_name: String },
    /// A download began. `total_bytes` is `None` when the server omits length.
    DownloadStarted {
        file_name: String,
        total_bytes: Option<u64>,
    },
    /// Cumulative byte-level progress during a download.
    DownloadProgress {
        file_name: String,
        downloaded: u64,
        total_bytes: Option<u64>,
    },
    /// A download completed and passed hash verification.
    DownloadFinished { file_name: String },
}

/// An open download: the total size (if the server reports it) and a reader
/// over the response body.
pub struct Download {
    pub total_bytes: Option<u64>,
    pub reader: Box<dyn Read>,
}

/// The network boundary. Real runs use [`HttpFetcher`]; tests inject a fake
/// that serves bytes from memory so no model is ever downloaded in CI.
pub trait Fetcher {
    /// Open a byte stream for `url`.
    fn get(&self, url: &str) -> Result<Download>;
}

/// Ensure every weight file for `entry` is present and hash-verified in
/// `cache_dir`, downloading the missing ones. Returns the local file paths in
/// manifest order — the paths the engine is handed. Emits the license relay
/// once, then a per-file cache/download/progress trace.
/// Make every listed model's weights present in `cache_dir`, fetching the
/// missing ones — the auto-fetch `separate` runs before a job (story 8). A
/// model whose files all exist is trusted by presence alone: re-hashing
/// multi-GB weights on every run costs seconds for corruption that
/// `models fetch` can always re-verify explicitly. Missing models take the
/// full [`ensure_model`] fetch-and-verify path, license relay included.
pub fn ensure_weights(
    entries: &[&ModelEntry],
    cache_dir: &Path,
    fetcher: &dyn Fetcher,
    mut on_event: impl FnMut(FetchEvent),
) -> Result<()> {
    for entry in entries {
        let warm = entry
            .files
            .iter()
            .all(|f| cache_dir.join(f.file_name).is_file());
        if !warm {
            ensure_model(entry, cache_dir, fetcher, &mut on_event)?;
        }
    }
    Ok(())
}

pub fn ensure_model(
    entry: &ModelEntry,
    cache_dir: &Path,
    fetcher: &dyn Fetcher,
    mut on_event: impl FnMut(FetchEvent),
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating model cache {}", cache_dir.display()))?;

    on_event(FetchEvent::License {
        model_id: entry.id.to_string(),
        license: entry.license,
    });

    let mut paths = Vec::with_capacity(entry.files.len());
    for file in entry.files {
        let dest = cache_dir.join(file.file_name);
        // A cached file counts only if it still matches its pinned digest; a
        // truncated or tampered file re-downloads instead of being trusted.
        if dest.is_file() && crate::job::sha256_file(&dest)? == file.sha256 {
            on_event(FetchEvent::Cached {
                file_name: file.file_name.to_string(),
            });
            paths.push(dest);
            continue;
        }
        download_verified(file, &dest, fetcher, &mut on_event)?;
        paths.push(dest);
    }
    Ok(paths)
}

/// Download one file to a `.partial` sibling, verify its hash, then rename it
/// into place. Nothing untrusted ever lands under the final name: a mismatch
/// removes the partial and fails, so a later `ensure_model` re-downloads
/// rather than trusting junk.
fn download_verified(
    file: &WeightFile,
    dest: &Path,
    fetcher: &dyn Fetcher,
    on_event: &mut impl FnMut(FetchEvent),
) -> Result<()> {
    // Fail closed: refuse to save weights we cannot verify rather than pin an
    // unchecked download.
    if file.sha256.is_empty() {
        bail!(
            "no pinned SHA-256 for {}: refusing to fetch unverified weights",
            file.file_name
        );
    }

    let download = fetcher
        .get(file.url)
        .with_context(|| format!("fetching {}", file.url))?;
    let total_bytes = download.total_bytes;
    let mut reader = download.reader;

    on_event(FetchEvent::DownloadStarted {
        file_name: file.file_name.to_string(),
        total_bytes,
    });

    let partial = dest.with_file_name(format!("{}.partial", file.file_name));
    let mut out =
        fs::File::create(&partial).with_context(|| format!("creating {}", partial.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader.read(&mut buf).context("reading download stream")?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])
            .with_context(|| format!("writing {}", partial.display()))?;
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        on_event(FetchEvent::DownloadProgress {
            file_name: file.file_name.to_string(),
            downloaded,
            total_bytes,
        });
    }
    out.flush().context("flushing download")?;
    drop(out);

    let got = format!("{:x}", hasher.finalize());
    if got != file.sha256 {
        let _ = fs::remove_file(&partial);
        bail!(
            "hash mismatch for {}: expected {}, got {got}",
            file.file_name,
            file.sha256
        );
    }
    fs::rename(&partial, dest).with_context(|| format!("finalizing {}", dest.display()))?;
    on_event(FetchEvent::DownloadFinished {
        file_name: file.file_name.to_string(),
    });
    Ok(())
}

/// The real network fetcher over HTTPS.
pub struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn get(&self, url: &str) -> Result<Download> {
        let response = ureq::get(url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let total_bytes = response
            .headers()
            .get(ureq::http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        let reader = response.into_body().into_reader();
        Ok(Download {
            total_bytes,
            reader: Box::new(reader),
        })
    }
}
