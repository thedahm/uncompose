//! Fake separation engine for contract tests: speaks the JSONL Engine
//! Contract (ADR-0001) without any ML stack. The core spawns it exactly like
//! the real shim (`<interpreter> -m uncompose_engine`); argv is ignored so
//! this binary can stand in as the interpreter itself.
//!
//! The behavior mode comes from the input file's basename, not an env var,
//! so parallel tests can't race on process-global state:
//!
//! - `error.*`     emit an error event, exit 1
//! - `crash.*`     die mid-run with no error event, exit 1
//! - `malformed.*` emit a line that is not a contract event, exit 0
//! - `no-done.*`   exit 0 without a done event
//! - `hang.*`      emit a stage event, then sleep (bounded, so an orphan
//!   left behind by a kill test still exits on its own)
//! - anything else: full success, six tiny stem files

use std::io::{Read, Write};
use std::path::Path;

const STEMS: [&str; 6] = ["vocals", "drums", "bass", "guitar", "keys", "other"];

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("reading request from stdin");
    let request: serde_json::Value =
        serde_json::from_str(&input).expect("request must be one JSON document");
    let audio_path = request["audio_path"].as_str().expect("audio_path");
    let output_dir = request["output_dir"].as_str().expect("output_dir");

    // Stderr is the engine's log channel; the core must route it to
    // engine.log and surface it in failure messages.
    eprintln!("fake-engine: starting on {audio_path}");

    let mode = Path::new(audio_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    emit(serde_json::json!({
        "event": "stage", "stage": "model_load", "message": request["model_id"]
    }));

    match mode.as_str() {
        "error" => {
            eprintln!("fake-engine: something broke in the model");
            emit(serde_json::json!({
                "event": "error", "message": "fake engine failed on purpose"
            }));
            1
        }
        "crash" => {
            eprintln!("fake-engine: dying without an error event");
            1
        }
        // Exits 0: the malformed line must fail the job on its own, without
        // an exit status to hide behind.
        "malformed" => {
            println!("this is not a contract event");
            0
        }
        "no-done" => 0,
        "hang" => {
            // Stage a partial before hanging so a cancel test can prove the
            // core cleans it up. It is never promoted (no stem event).
            let partial = Path::new(output_dir).join("vocals.wav.partial");
            std::fs::write(&partial, b"RIFF").expect("writing partial stem");
            emit(serde_json::json!({ "event": "stage", "stage": "separate" }));
            // Bounded so a kill test that orphans us doesn't leak forever.
            std::thread::sleep(std::time::Duration::from_secs(10));
            eprintln!("fake-engine: hang timed out");
            1
        }
        _ => {
            emit(serde_json::json!({
                "event": "stage", "stage": "separate", "percent": 50.0
            }));
            for name in STEMS {
                // Stems stream as `<stem>.wav.partial`; the core promotes each
                // to its final name when it sees the stem event.
                let path = Path::new(output_dir).join(format!("{name}.wav.partial"));
                std::fs::write(&path, b"RIFF").expect("writing stem file");
                emit(serde_json::json!({
                    "event": "stem", "name": name, "path": path
                }));
            }
            emit(serde_json::json!({
                "event": "done",
                "model_id": request["model_id"],
                "engine_version": "fake-0.0",
                "device": "cpu",
                "timings": { "model_load_secs": 0.0, "separate_secs": 0.0 }
            }));
            0
        }
    }
}

fn emit(event: serde_json::Value) {
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{event}").expect("writing event");
    stdout.flush().expect("flushing event");
}
