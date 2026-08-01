//! Engine Contract types (ADR-0001): the only thing both the core and a
//! separation engine know. JSON request on the engine's stdin, JSONL events
//! on its stdout, human logging on stderr.

use serde::{Deserialize, Serialize};

/// Request written to the engine's stdin as a single JSON document.
#[derive(Debug, Clone, Serialize)]
pub struct EngineRequest {
    /// Absolute path to the input audio file.
    pub audio_path: String,
    /// Model id as known to the engine (e.g. "htdemucs_6s").
    pub model_id: String,
    /// Directory the engine writes stem files into.
    pub output_dir: String,
    /// Directory holding model weights.
    pub model_dir: String,
    /// "auto" | "cpu" | "cuda"
    pub device: String,
}

/// One JSONL event on the engine's stdout, tagged by "event".
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EngineEvent {
    /// A processing stage began. Percent is best-effort and may never come.
    Stage {
        stage: String,
        #[serde(default)]
        percent: Option<f64>,
        #[serde(default)]
        message: Option<String>,
    },
    /// A stem file reached its final name inside output_dir.
    Stem { name: String, path: String },
    /// Terminal success. The engine exits 0 after this.
    Done {
        model_id: String,
        engine_version: String,
        device: String,
        timings: serde_json::Value,
    },
    /// Terminal failure. The engine exits nonzero after this.
    Error { message: String },
}
