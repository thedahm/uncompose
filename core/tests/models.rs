//! Model fetch/cache machinery tested at the faked download boundary. No
//! network and no real weights: a fake `Downloader` speaks bytes, and test
//! `ModelEntry` values stand in for the pinned manifest.

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use uncompose_core::models::{self, Downloader, FetchOutcome};
use uncompose_core::registry::{HardwareTier, ModelEntry};

/// sha256("fake weights payload")
const PAYLOAD: &[u8] = b"fake weights payload";
const PAYLOAD_SHA: &str = "3ae2ba7990c41b8238ccd7b4606645bb441c81a3da306f9c8bc0892ad440ad2e";

/// A downloader that emits fixed bytes and counts how often it is called.
struct FakeDownloader {
    bytes: Vec<u8>,
    calls: AtomicUsize,
}

impl FakeDownloader {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            calls: AtomicUsize::new(0),
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Downloader for FakeDownloader {
    fn download(&self, _url: &str, sink: &mut dyn Write) -> anyhow::Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Deliver in two chunks so byte progress is exercised as a stream.
        let mid = self.bytes.len() / 2;
        sink.write_all(&self.bytes[..mid])?;
        sink.write_all(&self.bytes[mid..])?;
        Ok(())
    }
}

struct FailingDownloader;
impl Downloader for FailingDownloader {
    fn download(&self, _url: &str, _sink: &mut dyn Write) -> anyhow::Result<()> {
        anyhow::bail!("network is down")
    }
}

fn pinned_entry() -> ModelEntry {
    ModelEntry {
        id: "test_model",
        filename: "test_model.bin",
        url: Some("https://example.invalid/test_model.bin"),
        sha256: Some(PAYLOAD_SHA),
        license: "test",
        hardware_tier: HardwareTier::RunsEverywhere,
    }
}

#[test]
fn fetch_downloads_verifies_and_reports_byte_progress() {
    let dir = tempfile::tempdir().unwrap();
    let entry = pinned_entry();
    let dl = FakeDownloader::new(PAYLOAD);

    let mut last_progress = 0u64;
    let outcome = models::fetch(dir.path(), &entry, &dl, |b| last_progress = b).unwrap();

    assert_eq!(
        outcome,
        FetchOutcome::Fetched {
            bytes: PAYLOAD.len() as u64,
            verified: true
        }
    );
    assert_eq!(last_progress, PAYLOAD.len() as u64, "byte progress relayed");
    let cached = models::cache_path(dir.path(), &entry);
    assert_eq!(std::fs::read(&cached).unwrap(), PAYLOAD);
    // No partial left behind.
    assert!(!cached.with_extension("bin.partial").exists());
}

#[test]
fn fetch_is_idempotent_and_skips_the_download_when_cached() {
    let dir = tempfile::tempdir().unwrap();
    let entry = pinned_entry();
    let dl = FakeDownloader::new(PAYLOAD);

    models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap();
    let second = models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap();

    assert_eq!(second, FetchOutcome::AlreadyCached);
    assert_eq!(dl.calls(), 1, "cached file must not re-download");
}

#[test]
fn fetch_rejects_a_hash_mismatch_and_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let entry = pinned_entry();
    let dl = FakeDownloader::new(b"different bytes here");

    let err = models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap_err();
    assert!(err.to_string().contains("hash mismatch"), "got: {err:#}");
    let cached = models::cache_path(dir.path(), &entry);
    assert!(!cached.exists(), "corrupt download must not be cached");
    assert!(
        !cached.with_extension("bin.partial").exists(),
        "partial cleaned"
    );
}

#[test]
fn fetch_cleans_up_when_the_download_fails() {
    let dir = tempfile::tempdir().unwrap();
    let entry = pinned_entry();

    let err = models::fetch(dir.path(), &entry, &FailingDownloader, |_| {}).unwrap_err();
    assert!(
        format!("{err:#}").contains("network is down"),
        "got: {err:#}"
    );
    let cached = models::cache_path(dir.path(), &entry);
    assert!(
        !cached.with_extension("bin.partial").exists(),
        "partial cleaned"
    );
}

#[test]
fn fetch_without_a_pin_reports_not_yet_fetchable() {
    let dir = tempfile::tempdir().unwrap();
    let mut entry = pinned_entry();
    entry.url = None;
    let dl = FakeDownloader::new(PAYLOAD);

    let err = models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap_err();
    assert!(err.to_string().contains("no download pin"), "got: {err:#}");
    assert_eq!(dl.calls(), 0);
}

#[test]
fn fetch_of_an_unpinned_hash_is_relayed_as_unverified() {
    let dir = tempfile::tempdir().unwrap();
    let mut entry = pinned_entry();
    entry.sha256 = None;
    let dl = FakeDownloader::new(PAYLOAD);

    let outcome = models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap();
    assert_eq!(
        outcome,
        FetchOutcome::Fetched {
            bytes: PAYLOAD.len() as u64,
            verified: false
        }
    );
}

#[test]
fn remove_deletes_a_cached_file_and_is_a_no_op_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let entry = pinned_entry();
    let dl = FakeDownloader::new(PAYLOAD);

    assert!(
        !models::remove(dir.path(), &entry).unwrap(),
        "nothing to remove"
    );
    models::fetch(dir.path(), &entry, &dl, |_| {}).unwrap();
    assert!(models::is_cached(dir.path(), &entry));
    assert!(models::remove(dir.path(), &entry).unwrap(), "removed");
    assert!(!models::is_cached(dir.path(), &entry));
}
