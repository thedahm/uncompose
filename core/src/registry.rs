//! The pinned model registry: Uncompose's static manifest of every model it
//! knows, plus the core-owned preset -> model mappings (ADR-0001, ADR-0003).
//!
//! Presets are fixed and never silently substitute by hardware, so the same
//! preset means the same result on every machine. License status is *relayed,
//! not certified*, and Hardware Tier is a first-class field surfaced before a
//! run and in `models list`.
//!
//! Download pins (`url`, `sha256`) are `Option` on purpose: the fetch
//! machinery is fully built and tested, but the verified pins for the real
//! weights are captured on the machine of record (real-model runs are manual
//! acceptance, not CI). An entry without a `url` reports as not-yet-fetchable
//! rather than pretending to download.

/// Whether a model runs on any machine or requires a GPU. Surfaced before a
/// run and in `models list` so the user is never surprised by a slow CPU path
/// or an impossible one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareTier {
    /// Runs everywhere (GPU fast, CPU slow-but-correct).
    RunsEverywhere,
    /// Requires a GPU; there is no viable CPU fallback.
    GpuRequired,
}

impl HardwareTier {
    pub fn label(self) -> &'static str {
        match self {
            HardwareTier::RunsEverywhere => "runs everywhere",
            HardwareTier::GpuRequired => "GPU required",
        }
    }
}

/// One model in the manifest. `filename` is the file the engine loads out of
/// the model cache directory; `url`/`sha256` pin the download for the
/// core-owned fetch (bypassing audio-separator's own downloader).
#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: &'static str,
    pub filename: &'static str,
    pub url: Option<&'static str>,
    /// Hex SHA-256 the download is verified against. `None` means no pin is
    /// captured yet; such a download is relayed as unverified rather than
    /// silently trusted.
    pub sha256: Option<&'static str>,
    /// License status, relayed not certified.
    pub license: &'static str,
    pub hardware_tier: HardwareTier,
}

/// The full manifest. Adding a model later is an entry here, not an
/// integration (ADR-0001).
pub const REGISTRY: &[ModelEntry] = &[
    ModelEntry {
        id: "htdemucs_6s",
        filename: "htdemucs_6s.yaml",
        url: None,
        sha256: None,
        license: "Demucs weights: research/non-commercial (relayed, not certified)",
        hardware_tier: HardwareTier::RunsEverywhere,
    },
    ModelEntry {
        id: "mel_band_roformer_kim",
        filename: "mel_band_roformer_kim.ckpt",
        url: None,
        sha256: None,
        license: "MIT (relayed, not certified)",
        hardware_tier: HardwareTier::GpuRequired,
    },
];

/// A named preset and the ordered model ids its pipeline needs. The `6-stem`
/// default is a Kim Mel-Band RoFormer vocal pre-pass then htdemucs_6s; the
/// `2-stem` preset is RoFormer alone (GPU-required).
struct Preset {
    name: &'static str,
    model_ids: &'static [&'static str],
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "6-stem",
        model_ids: &["mel_band_roformer_kim", "htdemucs_6s"],
    },
    Preset {
        name: "2-stem",
        model_ids: &["mel_band_roformer_kim"],
    },
];

/// Look up a model by its exact id.
pub fn find(id: &str) -> Option<&'static ModelEntry> {
    REGISTRY.iter().find(|m| m.id == id)
}

/// The ordered model ids a preset needs, or `None` if the name is not a preset.
pub fn preset_model_ids(name: &str) -> Option<&'static [&'static str]> {
    PRESETS.iter().find(|p| p.name == name).map(|p| p.model_ids)
}

/// Resolve a `models fetch` target (a model id or a preset name) to the
/// concrete set of entries it covers, in order and de-duplicated. `None` if
/// the target matches neither a model nor a preset.
pub fn resolve(target: &str) -> Option<Vec<&'static ModelEntry>> {
    if let Some(entry) = find(target) {
        return Some(vec![entry]);
    }
    let ids = preset_model_ids(target)?;
    let mut out: Vec<&'static ModelEntry> = Vec::new();
    for id in ids {
        if let Some(entry) = find(id) {
            if !out.iter().any(|e| e.id == entry.id) {
                out.push(entry);
            }
        }
    }
    Some(out)
}
