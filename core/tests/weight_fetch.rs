//! Contract tests for core-owned weight fetch (#30) against a faked network
//! and a tempdir cache: the download/verify/cache seam ADR-0001 owns. No real
//! model is ever downloaded here — the [`Fetcher`] boundary serves bytes from
//! memory.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;

use uncompose_core::fetch::{ensure_model, Download, FetchEvent, Fetcher};
use uncompose_core::registry::{self, HardwareTier, License, ModelEntry, WeightFile};

// SHA-256 of the exact byte payloads the fake server returns below.
const HASH_A: &str = "a4237e0c3b4831509aae7dc67a8001067433aff3ef5534cf71d4a5ec0cc661a4";
const HASH_SECOND: &str = "7c017f836887f3642bd516f67465eb9089669dcdb15ff95a1002c6322156f02c";
const BYTES_A: &[u8] = b"uncompose-fake-weight-a";
const BYTES_SECOND: &[u8] = b"second-file-bytes";

const MIT: License = License {
    label: "MIT",
    open: true,
};

/// A network fetcher backed by an in-memory url->bytes map. It records every
/// URL it is asked for so tests can prove a cache hit skipped the network, and
/// can be told to omit Content-Length for the length-unknown path.
struct FakeFetcher {
    bodies: HashMap<&'static str, &'static [u8]>,
    report_length: bool,
    requested: RefCell<Vec<String>>,
}

impl FakeFetcher {
    fn new(bodies: &[(&'static str, &'static [u8])]) -> Self {
        Self {
            bodies: bodies.iter().copied().collect(),
            report_length: true,
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl Fetcher for FakeFetcher {
    fn get(&self, url: &str) -> anyhow::Result<Download> {
        self.requested.borrow_mut().push(url.to_string());
        let bytes = *self
            .bodies
            .get(url)
            .unwrap_or_else(|| panic!("fake fetcher has no body for {url}"));
        Ok(Download {
            total_bytes: self.report_length.then_some(bytes.len() as u64),
            reader: Box::new(Cursor::new(bytes)),
        })
    }
}

/// A fetcher that fails the test if the network is touched at all.
struct ForbiddenFetcher;
impl Fetcher for ForbiddenFetcher {
    fn get(&self, url: &str) -> anyhow::Result<Download> {
        panic!("network touched for {url} on what should be a cache hit");
    }
}

const URL_A: &str = "https://example.invalid/a.ckpt";
const URL_B: &str = "https://example.invalid/b.yaml";

fn one_file_entry() -> ModelEntry {
    ModelEntry {
        id: "fake_model",
        license: MIT,
        hardware_tier: HardwareTier::RunsEverywhere,
        files: &[WeightFile {
            file_name: "a.ckpt",
            url: URL_A,
            sha256: HASH_A,
        }],
    }
}

fn collect<F: FnMut(&mut Vec<FetchEvent>)>(mut f: F) -> Vec<FetchEvent> {
    let mut events = Vec::new();
    f(&mut events);
    events
}

#[test]
fn fresh_download_relays_license_streams_progress_and_verifies_hash() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = one_file_entry();
    let fetcher = FakeFetcher::new(&[(URL_A, BYTES_A)]);

    let events = collect(|ev| {
        let paths = ensure_model(&entry, dir.path(), &fetcher, |e| ev.push(e))
            .expect("fetch should succeed");
        assert_eq!(paths, vec![dir.path().join("a.ckpt")]);
    });

    // License is relayed first, before any bytes move.
    assert_eq!(
        events[0],
        FetchEvent::License {
            model_id: "fake_model".into(),
            license: MIT,
        }
    );
    assert!(
        matches!(&events[1], FetchEvent::DownloadStarted { file_name, total_bytes }
            if file_name == "a.ckpt" && *total_bytes == Some(BYTES_A.len() as u64))
    );

    // Byte-level progress: cumulative and ending at the full size.
    let last_progress = events
        .iter()
        .filter_map(|e| match e {
            FetchEvent::DownloadProgress { downloaded, .. } => Some(*downloaded),
            _ => None,
        })
        .next_back()
        .expect("at least one progress event");
    assert_eq!(last_progress, BYTES_A.len() as u64);

    assert_eq!(
        events.last(),
        Some(&FetchEvent::DownloadFinished {
            file_name: "a.ckpt".into()
        })
    );

    // The verified bytes actually landed under the final name, no .partial.
    let dest = dir.path().join("a.ckpt");
    assert_eq!(std::fs::read(&dest).expect("weight file"), BYTES_A);
    assert!(!dir.path().join("a.ckpt.partial").exists());
}

#[test]
fn second_run_hits_the_cache_and_never_touches_the_network() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = one_file_entry();

    ensure_model(
        &entry,
        dir.path(),
        &FakeFetcher::new(&[(URL_A, BYTES_A)]),
        |_| {},
    )
    .expect("first fetch");

    let events = collect(|ev| {
        let paths = ensure_model(&entry, dir.path(), &ForbiddenFetcher, |e| ev.push(e))
            .expect("cached fetch");
        assert_eq!(paths, vec![dir.path().join("a.ckpt")]);
    });

    assert!(events.contains(&FetchEvent::Cached {
        file_name: "a.ckpt".into()
    }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, FetchEvent::DownloadStarted { .. })),
        "cache hit must not download"
    );
}

#[test]
fn corrupt_cached_file_is_redownloaded_not_trusted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = one_file_entry();
    // A file under the final name whose bytes do not match the pinned digest.
    std::fs::write(dir.path().join("a.ckpt"), b"tampered").expect("seed cache");

    let fetcher = FakeFetcher::new(&[(URL_A, BYTES_A)]);
    ensure_model(&entry, dir.path(), &fetcher, |_| {}).expect("re-fetch");

    assert_eq!(std::fs::read(dir.path().join("a.ckpt")).unwrap(), BYTES_A);
    assert_eq!(fetcher.requested.borrow().as_slice(), [URL_A.to_string()]);
}

#[test]
fn hash_mismatch_fails_and_leaves_no_file_behind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = one_file_entry();
    // Server returns bytes that do not hash to the pinned digest.
    let fetcher = FakeFetcher::new(&[(URL_A, b"wrong bytes")]);

    let err = format!(
        "{:#}",
        ensure_model(&entry, dir.path(), &fetcher, |_| {}).expect_err("should reject")
    );
    assert!(err.contains("hash mismatch"), "got: {err}");
    assert!(
        !dir.path().join("a.ckpt").exists(),
        "no unverified file kept"
    );
    assert!(
        !dir.path().join("a.ckpt.partial").exists(),
        "no partial kept"
    );
}

#[test]
fn unpinned_digest_refuses_to_fetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = ModelEntry {
        files: &[WeightFile {
            file_name: "a.ckpt",
            url: URL_A,
            sha256: "",
        }],
        ..one_file_entry()
    };
    let err = format!(
        "{:#}",
        ensure_model(&entry, dir.path(), &ForbiddenFetcher, |_| {}).expect_err("fail closed")
    );
    assert!(err.contains("no pinned SHA-256"), "got: {err}");
}

#[test]
fn multi_file_model_fetches_every_file_in_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = ModelEntry {
        id: "two_file_model",
        license: MIT,
        hardware_tier: HardwareTier::GpuRequired,
        files: &[
            WeightFile {
                file_name: "a.ckpt",
                url: URL_A,
                sha256: HASH_A,
            },
            WeightFile {
                file_name: "b.yaml",
                url: URL_B,
                sha256: HASH_SECOND,
            },
        ],
    };
    let fetcher = FakeFetcher::new(&[(URL_A, BYTES_A), (URL_B, BYTES_SECOND)]);
    let paths = ensure_model(&entry, dir.path(), &fetcher, |_| {}).expect("fetch both");

    assert_eq!(
        paths,
        vec![dir.path().join("a.ckpt"), dir.path().join("b.yaml")]
    );
    assert_eq!(std::fs::read(&paths[0]).unwrap(), BYTES_A);
    assert_eq!(std::fs::read(&paths[1]).unwrap(), BYTES_SECOND);
    assert_eq!(
        fetcher.requested.borrow().as_slice(),
        [URL_A.to_string(), URL_B.to_string()]
    );
}

#[test]
fn missing_content_length_still_downloads_and_verifies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = one_file_entry();
    let mut fetcher = FakeFetcher::new(&[(URL_A, BYTES_A)]);
    fetcher.report_length = false;

    let events = collect(|ev| {
        ensure_model(&entry, dir.path(), &fetcher, |e| ev.push(e)).expect("length-unknown fetch");
    });

    assert!(matches!(
        &events[1],
        FetchEvent::DownloadStarted {
            total_bytes: None,
            ..
        }
    ));
    assert_eq!(std::fs::read(dir.path().join("a.ckpt")).unwrap(), BYTES_A);
}

#[test]
fn manifest_pins_the_two_v01_models_with_relayed_license_and_tier() {
    // The 6-stem workhorse: runs everywhere, weights relayed research-only.
    let demucs = registry::find("htdemucs_6s").expect("htdemucs_6s in manifest");
    assert_eq!(demucs.hardware_tier, HardwareTier::RunsEverywhere);
    assert_eq!(demucs.license.label, "research-only");
    assert!(!demucs.license.open, "relayed as not open, not certified");
    assert!(!demucs.files.is_empty());

    // The 2-stem vocal model: GPU-required, MIT weights.
    let roformer = registry::find("kim_mel_band_roformer").expect("roformer in manifest");
    assert_eq!(roformer.hardware_tier, HardwareTier::GpuRequired);
    assert_eq!(roformer.license.label, "MIT");
    assert!(roformer.license.open);

    // Every pinned file has a URL; digests are pinned on the machine of record.
    for entry in registry::MANIFEST {
        for file in entry.files {
            assert!(
                file.url.starts_with("https://"),
                "{} not https",
                file.file_name
            );
        }
    }

    assert!(registry::find("no_such_model").is_none());
}
