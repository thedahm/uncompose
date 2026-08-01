//! The pinned model manifest: the core's static, in-package registry of the
//! weights it knows how to fetch. Each entry pins a model id, its downloadable
//! files (URL + SHA-256), its license status (relayed, not certified), and its
//! Hardware Tier. Pinning in-package is what makes a preset mean the same
//! weights on every machine and never silently substitute by hardware
//! (ADR-0001, #11, #27).

/// Whether a model needs a GPU or runs on any hardware. A first-class Engine
/// Contract field, surfaced before a run so the user is never surprised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareTier {
    /// Correct on CPU, just slow: the 6-stem preset's models.
    RunsEverywhere,
    /// Needs CUDA: the 2-stem Mel-Band RoFormer preset.
    GpuRequired,
}

impl HardwareTier {
    /// The user-facing tier label, surfaced before a run.
    pub fn label(self) -> &'static str {
        match self {
            HardwareTier::RunsEverywhere => "runs everywhere",
            HardwareTier::GpuRequired => "GPU required",
        }
    }
}

/// A weight file's license, relayed to the user and never certified by
/// Uncompose. `label` is the status shown ("MIT", "research-only"); `open` is
/// false for research-only / unlicensed checkpoints so callers can flag them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct License {
    pub label: &'static str,
    pub open: bool,
}

/// One downloadable weight file, pinned by URL and expected SHA-256. The
/// digest is lowercase hex and is the integrity gate every download passes.
#[derive(Debug, Clone, Copy)]
pub struct WeightFile {
    /// Name the file is cached under and handed to the engine.
    pub file_name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

/// A model in the manifest: an id the engine understands, its relayed license
/// and Hardware Tier, and the one-or-more files that make it up.
#[derive(Debug, Clone, Copy)]
pub struct ModelEntry {
    pub id: &'static str,
    pub license: License,
    pub hardware_tier: HardwareTier,
    pub files: &'static [WeightFile],
}

const MIT: License = License {
    label: "MIT",
    open: true,
};

// Demucs weights ship as "research purposes only", unlicensed (#3). We relay
// that, we do not certify it.
const RESEARCH_ONLY: License = License {
    label: "research-only",
    open: false,
};

// The `sha256` digests are the integrity gate: a download that does not match
// is rejected, and an unset (empty) digest fails closed rather than saving
// unverified weights. They are left unset here and pinned from the real
// artifacts on the machine of record during M1 acceptance, the only place real
// downloads happen; CI verifies the machinery against the fake fetcher instead.
const HTDEMUCS_6S_FILES: &[WeightFile] = &[WeightFile {
    file_name: "955717e8-8726e21a.th",
    url: "https://dl.fbaipublicfiles.com/demucs/hybrid_transformer/955717e8-8726e21a.th",
    sha256: "",
}];

const KIM_ROFORMER_FILES: &[WeightFile] = &[
    WeightFile {
        file_name: "model_bs_roformer_ep_317_sdr_12.9755.ckpt",
        url: "https://huggingface.co/unwa/kim-mel-band-roformer/resolve/main/model_bs_roformer_ep_317_sdr_12.9755.ckpt",
        sha256: "",
    },
    WeightFile {
        file_name: "model_bs_roformer_ep_317_sdr_12.9755.yaml",
        url: "https://huggingface.co/unwa/kim-mel-band-roformer/resolve/main/model_bs_roformer_ep_317_sdr_12.9755.yaml",
        sha256: "",
    },
];

/// The pinned manifest. Fixed model->files mappings, owned by the core.
pub const MANIFEST: &[ModelEntry] = &[
    ModelEntry {
        id: "htdemucs_6s",
        license: RESEARCH_ONLY,
        hardware_tier: HardwareTier::RunsEverywhere,
        files: HTDEMUCS_6S_FILES,
    },
    ModelEntry {
        id: "mel_band_roformer_kim",
        license: MIT,
        hardware_tier: HardwareTier::GpuRequired,
        files: KIM_ROFORMER_FILES,
    },
];

/// Look a model up by its manifest id.
pub fn find(model_id: &str) -> Option<&'static ModelEntry> {
    MANIFEST.iter().find(|m| m.id == model_id)
}

/// Resolve a target — a bare model id or a preset name — to manifest
/// entries. Preset targets resolve to their pipeline's models in order.
pub fn resolve(target: &str) -> Option<Vec<&'static ModelEntry>> {
    if let Some(entry) = find(target) {
        return Some(vec![entry]);
    }
    let preset = crate::preset::by_name(target)?;
    preset.steps.iter().map(|s| find(s.model.id)).collect()
}
