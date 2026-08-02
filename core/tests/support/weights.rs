//! Warm-cache seeding for CLI suites that run `separate`: the auto-fetch
//! trusts presence, so stub files keep every test off the network.

use std::path::Path;

/// Seed every manifest weight file (as stubs) into `<cache>/uncompose/models`
/// so a `separate` under test finds a warm cache: the auto-fetch trusts
/// presence and never touches the network in CI.
pub fn seed_weights(cache: &Path) {
    let models = cache.join("uncompose/models");
    std::fs::create_dir_all(&models).expect("creating model cache");
    for entry in uncompose_core::registry::MANIFEST {
        for file in entry.files {
            std::fs::write(models.join(file.file_name), b"stub weights").expect("seeding weights");
        }
    }
}
