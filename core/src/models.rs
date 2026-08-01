//! Core-owned model fetch/cache machinery behind a substitutable download
//! boundary. The core resolves cache paths, downloads, verifies against the
//! pinned hash, and relays byte progress; the engine only ever receives local
//! file paths and never touches the network (ADR-0001).
//!
//! `Downloader` is the one seam tests substitute (a fake speaking bytes), so
//! the whole thing exercises GPU-free and network-free. `CurlDownloader` is
//! the production implementation: a shell-out consistent with how the rest of
//! Uncompose reaches external tools, streaming the body so byte progress is
//! real and no TLS stack is linked into the wheel.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::registry::{ModelEntry, REGISTRY};

/// Where a model's file lives inside the cache directory.
pub fn cache_path(model_dir: &Path, entry: &ModelEntry) -> PathBuf {
    model_dir.join(entry.filename)
}

/// Whether the model's file is present in the cache. Presence is the marker;
/// integrity is checked at fetch time against the pinned hash.
pub fn is_cached(model_dir: &Path, entry: &ModelEntry) -> bool {
    cache_path(model_dir, entry).is_file()
}

/// A registry entry paired with its cache state, for `models list`.
pub struct ModelStatus {
    pub entry: &'static ModelEntry,
    pub cached: bool,
}

/// Every known model with its cache state.
pub fn list(model_dir: &Path) -> Vec<ModelStatus> {
    REGISTRY
        .iter()
        .map(|entry| ModelStatus {
            entry,
            cached: is_cached(model_dir, entry),
        })
        .collect()
}

/// What a fetch did.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchOutcome {
    /// The file was already present (and passed hash verification if pinned).
    AlreadyCached,
    /// Freshly downloaded. `verified` is false when the entry has no pinned
    /// hash, so the caller can relay that the weights are unverified.
    Fetched { bytes: u64, verified: bool },
}

/// The download boundary. An implementation streams `url` to `sink`, which the
/// core writes to disk while counting bytes. Errors on any non-success.
pub trait Downloader {
    fn download(&self, url: &str, sink: &mut dyn Write) -> Result<()>;
}

/// Fetch one model into the cache, verifying against its pinned hash and
/// relaying cumulative byte progress. Idempotent: a cached, verified file is
/// left untouched. The download lands in a `.partial` sibling and is renamed
/// into place only after verification, so an interrupted fetch never looks
/// complete.
pub fn fetch(
    model_dir: &Path,
    entry: &ModelEntry,
    downloader: &dyn Downloader,
    mut on_progress: impl FnMut(u64),
) -> Result<FetchOutcome> {
    let dest = cache_path(model_dir, entry);
    if dest.is_file() {
        if let Some(expected) = entry.sha256 {
            let got = sha256_file(&dest)?;
            if got.eq_ignore_ascii_case(expected) {
                return Ok(FetchOutcome::AlreadyCached);
            }
            // A cached file that fails its pin is corrupt; re-fetch over it.
        } else {
            return Ok(FetchOutcome::AlreadyCached);
        }
    }

    let url = entry.url.with_context(|| {
        format!(
            "no download pin for '{}' yet; its weights are fetched by the engine \
             on first use until a verified pin lands",
            entry.id
        )
    })?;

    std::fs::create_dir_all(model_dir).context("creating model cache dir")?;
    let partial = dest.with_extension(format!(
        "{}.partial",
        dest.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));

    let mut sink = CountingHasher::new(
        std::fs::File::create(&partial).context("creating partial download file")?,
    );
    let result = downloader
        .download(url, &mut sink)
        .with_context(|| format!("downloading '{}' from {url}", entry.id));
    let (mut file, hasher, bytes) = sink.into_parts();
    file.flush().ok();
    drop(file);

    if let Err(e) = result {
        std::fs::remove_file(&partial).ok();
        return Err(e);
    }

    on_progress(bytes);

    let mut verified = false;
    if let Some(expected) = entry.sha256 {
        let got = format!("{:x}", hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            std::fs::remove_file(&partial).ok();
            bail!(
                "hash mismatch for '{}': expected {expected}, got {got}",
                entry.id
            );
        }
        verified = true;
    }

    std::fs::rename(&partial, &dest)
        .with_context(|| format!("finalizing '{}' into cache", entry.id))?;
    Ok(FetchOutcome::Fetched { bytes, verified })
}

/// Remove a model's cached file. Returns whether anything was removed.
pub fn remove(model_dir: &Path, entry: &ModelEntry) -> Result<bool> {
    let path = cache_path(model_dir, entry);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

/// A `Write` that both persists bytes and hashes them, counting the total.
struct CountingHasher<W: Write> {
    inner: W,
    hasher: Sha256,
    bytes: u64,
}

impl<W: Write> CountingHasher<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    fn into_parts(self) -> (W, Sha256, u64) {
        (self.inner, self.hasher, self.bytes)
    }
}

impl<W: Write> Write for CountingHasher<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        self.bytes += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path).context("opening cached file for hashing")?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Production downloader: stream the body via `curl`, count bytes as they
/// arrive. curl's stderr is captured and surfaced on failure; a missing curl
/// is reported as a clear, actionable message.
pub struct CurlDownloader;

impl Downloader for CurlDownloader {
    fn download(&self, url: &str, sink: &mut dyn Write) -> Result<()> {
        let mut child = Command::new("curl")
            .args(["--fail", "--location", "--silent", "--show-error", url])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!("curl not found: install curl to download model weights")
                } else {
                    anyhow::Error::new(e).context("spawning curl")
                }
            })?;

        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut buf = [0u8; 65536];
        loop {
            let n = stdout.read(&mut buf).context("reading download stream")?;
            if n == 0 {
                break;
            }
            sink.write_all(&buf[..n]).context("writing download")?;
        }

        let output = child.wait_with_output().context("waiting for curl")?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("curl failed ({}): {}", output.status, err.trim());
        }
        Ok(())
    }
}
