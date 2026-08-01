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
// unverified weights. Pinned from the real artifacts on the machine of record
// at M1 acceptance (#33); CI verifies the machinery against the fake fetcher
// instead. File names match what the engine asks audio-separator to load, so
// a fetched cache is a warm cache. The demucs checkpoint's own name embeds the
// first 8 hex of its sha256 (`-34c22ccb`), a built-in cross-check.
const HTDEMUCS_6S_FILES: &[WeightFile] = &[
    WeightFile {
        file_name: "5c90dfd2-34c22ccb.th",
        url: "https://dl.fbaipublicfiles.com/demucs/hybrid_transformer/5c90dfd2-34c22ccb.th",
        sha256: "34c22ccb381c6f9fdbf324f04e1e2fe21aaaf293f5ded163a162697ff9a02ddd",
    },
    WeightFile {
        file_name: "htdemucs_6s.yaml",
        url: "https://github.com/TRvlvr/model_repo/releases/download/all_public_uvr_models/htdemucs_6s.yaml",
        sha256: "207405151270af8fd81c2373c25d27950916682ac91dca7884a11ce13dad6f58",
    },
];

// Kim Jensen's Mel-Band RoFormer vocal model, the #14 A/B winner, from
// audio-separator's own model mirror (the UVR public repo does not carry it).
const KIM_ROFORMER_FILES: &[WeightFile] = &[
    WeightFile {
        file_name: "vocals_mel_band_roformer.ckpt",
        url: "https://github.com/nomadkaraoke/python-audio-separator/releases/download/model-configs/vocals_mel_band_roformer.ckpt",
        sha256: "87201f4d31afb5bc79993230fc49446918425574db48c01c405e44f365c7559e",
    },
    WeightFile {
        file_name: "vocals_mel_band_roformer.yaml",
        url: "https://github.com/nomadkaraoke/python-audio-separator/releases/download/model-configs/vocals_mel_band_roformer.yaml",
        sha256: "b958b29c8f7195f0d86bee6759a33980db675c4ecaf2fcaa80fa125828e6cd38",
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
