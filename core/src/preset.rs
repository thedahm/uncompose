//! Core-owned static Preset/Model registry and pipeline shapes (ADR-0001,
//! CONTEXT.md). A Preset is a fixed recipe the user picks; it maps to one or
//! more engine calls the core orchestrates. Presets never silently
//! substitute models based on hardware — the mapping here is the whole
//! contract. Weight-fetch details (URLs, hashes) belong to the download
//! manifest (#30); a `Model` here carries only what a run reasons about.

/// A model's declared hardware requirement, surfaced before a run rather
/// than discovered during one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareTier {
    RunsEverywhere,
    GpuRequired,
}

impl HardwareTier {
    /// Human label for the pre-run header and `models list`.
    pub fn label(self) -> &'static str {
        match self {
            HardwareTier::RunsEverywhere => "runs everywhere",
            HardwareTier::GpuRequired => "GPU required",
        }
    }
}

/// A specific separation checkpoint an engine can run, carrying its own
/// license status (relayed, not certified) and hardware tier.
#[derive(Debug, Clone, Copy)]
pub struct Model {
    /// Id as known to the engine and passed over the contract.
    pub id: &'static str,
    pub hardware_tier: HardwareTier,
    /// License status, relayed not certified.
    pub license: &'static str,
}

/// Where a pipeline step gets its audio input.
#[derive(Debug, Clone, Copy)]
pub enum StepInput {
    /// The original song the user passed.
    Song,
    /// A named intermediate produced by an earlier step.
    Intermediate(&'static str),
}

/// What the core does with one stem the engine emits during a step.
#[derive(Debug, Clone, Copy)]
pub enum Route {
    /// Keep as a final stem under this preset-level name (`keys`, not
    /// `piano` — the name is stable even as the model changes).
    Stem(&'static str),
    /// Keep as a named intermediate that feeds a later step. Lives in the
    /// job folder while the pipeline runs; deleted on success.
    Intermediate(&'static str),
    /// Discard (e.g. htdemucs_6s' own vocals in the 6-stem pipeline, where
    /// the RoFormer pre-pass owns `vocals`).
    Discard,
}

/// One emitted stem's fate, mapping the engine's stem name to a [`Route`].
#[derive(Debug, Clone, Copy)]
pub struct Output {
    /// The stem name the engine emits for this model.
    pub engine_name: &'static str,
    pub route: Route,
}

/// One engine call in a preset's pipeline.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub model: Model,
    pub input: StepInput,
    pub outputs: &'static [Output],
}

/// A named, fixed recipe: the pipeline of engine calls plus the final stem
/// set. Owned by the core; the same preset means the same result on every
/// machine.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    /// Surfaced before a run. Declared per preset rather than derived from
    /// its models: the 6-stem default is CPU-viable (just slow) even though
    /// its RoFormer pre-pass is happiest on a GPU.
    pub hardware_tier: HardwareTier,
    /// Final stem names, in presentation order.
    pub stems: &'static [&'static str],
    pub steps: &'static [Step],
}

/// htdemucs_6s: the whole 6-stem set in one pass. Weights are unlicensed
/// ("research purposes only", per ADR-0001).
pub const HTDEMUCS_6S: Model = Model {
    id: "htdemucs_6s",
    hardware_tier: HardwareTier::RunsEverywhere,
    license: "research-only (weights unlicensed)",
};

/// Kim Mel-Band RoFormer: best-in-class vocals, MIT weights, 2-stem only.
pub const MEL_BAND_ROFORMER_KIM: Model = Model {
    id: "mel_band_roformer_kim",
    hardware_tier: HardwareTier::GpuRequired,
    license: "MIT",
};

/// The default preset. A RoFormer vocal pre-pass (its vocals become the
/// final `vocals.wav`) feeds its instrumental to htdemucs_6s for the
/// remaining five stems; htdemucs' own vocals are discarded and its `piano`
/// stem is surfaced as `keys`.
pub const SIX_STEM: Preset = Preset {
    name: "6-stem",
    hardware_tier: HardwareTier::RunsEverywhere,
    stems: &["vocals", "drums", "bass", "guitar", "keys", "other"],
    steps: &[
        Step {
            model: MEL_BAND_ROFORMER_KIM,
            input: StepInput::Song,
            outputs: &[
                Output {
                    engine_name: "vocals",
                    route: Route::Stem("vocals"),
                },
                Output {
                    engine_name: "instrumental",
                    route: Route::Intermediate("instrumental"),
                },
            ],
        },
        Step {
            model: HTDEMUCS_6S,
            input: StepInput::Intermediate("instrumental"),
            outputs: &[
                Output {
                    engine_name: "vocals",
                    route: Route::Discard,
                },
                Output {
                    engine_name: "drums",
                    route: Route::Stem("drums"),
                },
                Output {
                    engine_name: "bass",
                    route: Route::Stem("bass"),
                },
                Output {
                    engine_name: "guitar",
                    route: Route::Stem("guitar"),
                },
                Output {
                    engine_name: "keys",
                    route: Route::Stem("keys"),
                },
                Output {
                    engine_name: "other",
                    route: Route::Stem("other"),
                },
            ],
        },
    ],
};

/// A quick vocals/instrumental split: RoFormer alone. GPU-required.
pub const TWO_STEM: Preset = Preset {
    name: "2-stem",
    hardware_tier: HardwareTier::GpuRequired,
    stems: &["vocals", "instrumental"],
    steps: &[Step {
        model: MEL_BAND_ROFORMER_KIM,
        input: StepInput::Song,
        outputs: &[
            Output {
                engine_name: "vocals",
                route: Route::Stem("vocals"),
            },
            Output {
                engine_name: "instrumental",
                route: Route::Stem("instrumental"),
            },
        ],
    }],
};

/// Every preset the core knows, in menu order.
pub const PRESETS: &[&Preset] = &[&SIX_STEM, &TWO_STEM];

/// Look up a preset by its user-facing name.
pub fn by_name(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().copied().find(|p| p.name == name)
}
