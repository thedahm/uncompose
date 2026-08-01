//! Core-owned presets (user story 27): fixed model mappings that never
//! silently substitute by hardware, so the same preset means the same result
//! on every machine. The printed surface (#31) reads its header — preset
//! name, stem names, model id, relayed license — straight from here.
//!
//! License status is *relayed, not certified*. The authoritative source is
//! the weight manifest (#30); until it lands, these strings mirror the
//! reference transcripts and are the single place the CLI header consults.

/// A separation preset: a stable name, the stems it yields, and the model
/// that (in v0.1) produces them.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    /// Stable, user-facing name (e.g. "6-stem"). Never changes with hardware.
    pub id: &'static str,
    /// Stem file basenames this preset writes, in display order.
    pub stems: &'static [&'static str],
    /// Model id as known to the engine.
    pub model_id: &'static str,
    /// Relayed (not certified) weight license status, e.g. "research-only".
    pub license: &'static str,
    /// Whether the preset needs a CUDA GPU (no correct CPU fallback).
    pub gpu_required: bool,
}

/// The preset a bare `separate` uses when `--preset` is omitted.
pub const DEFAULT: &str = "6-stem";

const PRESETS: &[Preset] = &[
    Preset {
        id: "6-stem",
        stems: &["vocals", "drums", "bass", "guitar", "keys", "other"],
        model_id: "htdemucs_6s",
        license: "research-only",
        gpu_required: false,
    },
    Preset {
        id: "2-stem",
        stems: &["vocals", "instrumental"],
        model_id: "melband-roformer-kim",
        license: "MIT",
        gpu_required: true,
    },
];

/// Look up a preset by its stable name. `None` for an unknown name.
pub fn lookup(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_is_the_six_stem_preset() {
        let preset = lookup(DEFAULT).expect("default preset must exist");
        assert_eq!(preset.id, "6-stem");
        assert_eq!(preset.model_id, "htdemucs_6s");
        assert_eq!(
            preset.stems,
            &["vocals", "drums", "bass", "guitar", "keys", "other"]
        );
        assert!(!preset.gpu_required);
    }

    #[test]
    fn two_stem_preset_is_gpu_required() {
        let preset = lookup("2-stem").expect("2-stem preset must exist");
        assert_eq!(preset.stems, &["vocals", "instrumental"]);
        assert!(preset.gpu_required);
    }

    #[test]
    fn unknown_preset_is_none() {
        assert!(lookup("nonesuch").is_none());
    }
}
