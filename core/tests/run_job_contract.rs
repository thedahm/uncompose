//! Contract tests for `run_job` against the fake engine: the core's side of
//! the Engine Contract seam (spawn, stream, pipeline composition, failure
//! handling, folder shape). The fake engine emits model-appropriate stems,
//! so a preset's pipeline runs for real — the core just can't tell it apart
//! from the shim.

mod support;

use std::path::Path;

use uncompose_core::preset::{Preset, SIX_STEM, TWO_STEM};
use uncompose_core::{run_job, JobConfig, JobEvent};

const SIX_STEMS: [&str; 6] = ["vocals", "drums", "bass", "guitar", "keys", "other"];

fn config(dir: &Path, input_name: &str, preset: &'static Preset) -> JobConfig {
    let input = dir.join(input_name);
    std::fs::write(&input, b"not really audio").expect("writing input");
    JobConfig {
        input,
        preset,
        parameters: serde_json::json!({ "shifts": 1 }),
        device: "cpu".into(),
        model_dir: dir.join("models"),
        state_dir: dir.join("state"),
        engine_python: support::fake_engine(),
        output: None,
    }
}

fn run(config: &JobConfig) -> (anyhow::Result<uncompose_core::JobOutcome>, Vec<JobEvent>) {
    let mut events = Vec::new();
    let outcome = run_job(config, |e| events.push(e));
    (outcome, events)
}

#[test]
fn six_stem_composes_two_engine_calls_into_six_stems() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config(dir.path(), "song.wav", &SIX_STEM);
    let (outcome, events) = run(&config);
    let outcome = outcome.expect("job should succeed");

    assert_eq!(outcome.job_folder, dir.path().join("song.stems"));
    assert_eq!(outcome.stems, SIX_STEMS);
    for stem in SIX_STEMS {
        assert!(
            outcome.job_folder.join(format!("{stem}.wav")).is_file(),
            "{stem}.wav present"
        );
        // Partials are promoted to final names, never left behind on success.
        assert!(!outcome
            .job_folder
            .join(format!("{stem}.wav.partial"))
            .exists());
    }

    // The Started event fires first, carrying the resolved job folder, then
    // stage events stream through before stems.
    assert!(
        matches!(&events[0], JobEvent::Started { job_folder } if job_folder == &outcome.job_folder),
        "first event announces the job folder"
    );
    assert!(matches!(&events[1], JobEvent::Stage { stage, .. } if stage == "model_load"));

    // Two engine calls: the RoFormer pre-pass on the song, then htdemucs on
    // the instrumental it produced.
    let log = std::fs::read_to_string(outcome.job_folder.join("engine.log")).expect("engine.log");
    assert_eq!(
        log.matches("fake-engine: starting").count(),
        2,
        "two engine calls: {log}"
    );
    assert!(
        log.contains("instrumental.wav"),
        "step 2 ran on the RoFormer instrumental: {log}"
    );

    // Only the final stem set streams to the UI; htdemucs' own vocals and the
    // instrumental never surface as Stem events.
    let stem_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Stem { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        stem_events,
        ["vocals", "drums", "bass", "guitar", "keys", "other"]
    );

    // job.json is the completion marker and reproducibility record.
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(outcome.job_folder.join("job.json")).expect("reading job.json"),
    )
    .expect("job.json is JSON");
    assert_eq!(record["preset"], "6-stem");
    assert_eq!(
        record["models"]
            .as_array()
            .map(|m| m.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()),
        Some(vec!["mel_band_roformer_kim", "htdemucs_6s"])
    );
    assert_eq!(record["parameters"]["shifts"], 1);
    assert_eq!(record["outcome"], "success");
    assert_eq!(record["device"], "cpu");
    assert_eq!(record["engine_version"], "fake-0.0");
    assert_eq!(record["stems"].as_array().map(Vec::len), Some(6));
    assert_eq!(
        record["input_sha256"].as_str().map(str::len),
        Some(64),
        "input hash recorded"
    );
}

#[test]
fn six_stem_deletes_intermediates_and_discards_on_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "song.wav", &SIX_STEM));
    let folder = outcome.expect("success").job_folder;

    // The instrumental intermediate and htdemucs' discarded vocals lived in
    // scratch dirs that are gone on success; only the six promised stems and
    // the records remain.
    assert!(!folder.join(".stage-1").exists(), "scratch removed");
    assert!(!folder.join(".stage-2").exists(), "scratch removed");
    assert!(
        !folder.join("instrumental.wav").exists(),
        "intermediate is never a promised output"
    );
    let stems: Vec<_> = std::fs::read_dir(&folder)
        .expect("reading job folder")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".wav"))
        .collect();
    assert_eq!(stems.len(), 6, "exactly the six promised stems: {stems:?}");
}

#[test]
fn two_stem_runs_roformer_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, events) = run(&config(dir.path(), "song.wav", &TWO_STEM));
    let outcome = outcome.expect("job should succeed");

    assert_eq!(outcome.stems, ["vocals", "instrumental"]);
    assert!(outcome.job_folder.join("vocals.wav").is_file());
    assert!(outcome.job_folder.join("instrumental.wav").is_file());

    // Stage events stream before stems.
    // Started fires first; stage events stream through before stems.
    assert!(matches!(&events[1], JobEvent::Stage { stage, .. } if stage == "model_load"));

    let log = std::fs::read_to_string(outcome.job_folder.join("engine.log")).expect("engine.log");
    assert_eq!(
        log.matches("fake-engine: starting").count(),
        1,
        "one engine call: {log}"
    );

    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(outcome.job_folder.join("job.json")).expect("reading job.json"),
    )
    .expect("job.json is JSON");
    assert_eq!(record["preset"], "2-stem");
    assert_eq!(
        record["models"].as_array().map(Vec::len),
        Some(1),
        "single-model pipeline"
    );
}

#[test]
fn success_points_the_last_job_pointer_at_the_folder() {
    use uncompose_core::state;
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config(dir.path(), "song.wav", &TWO_STEM);
    let outcome = run(&config).0.expect("job should succeed");

    let last = state::read_last_job(&config.state_dir)
        .expect("reading pointer")
        .expect("pointer written after a successful run");
    assert_eq!(last.job_folder, outcome.job_folder);
    assert_eq!(last.input_path, config.input.canonicalize().unwrap());
}

#[test]
fn failure_leaves_the_previous_last_job_pointer_intact() {
    use uncompose_core::state;
    let dir = tempfile::tempdir().expect("tempdir");
    // A successful run sets the pointer.
    let good = config(dir.path(), "song.wav", &TWO_STEM);
    let good_outcome = run(&good).0.expect("first run should succeed");
    // A later failing run must not clobber it: play/open still find the good job.
    let mut bad = config(dir.path(), "error.wav", &TWO_STEM);
    bad.state_dir = good.state_dir.clone();
    run(&bad).0.expect_err("second run should fail");

    let last = state::read_last_job(&good.state_dir)
        .expect("reading pointer")
        .expect("pointer still present");
    assert_eq!(last.job_folder, good_outcome.job_folder);
}

#[test]
fn engine_stderr_lands_in_engine_log_not_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "song.wav", &TWO_STEM));
    let log = std::fs::read_to_string(outcome.expect("success").job_folder.join("engine.log"))
        .expect("reading engine.log");
    assert!(log.contains("fake-engine: starting"));
}

#[test]
fn repeated_runs_suffix_the_job_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config(dir.path(), "song.wav", &TWO_STEM);
    let first = run(&config).0.expect("first run");
    let second = run(&config).0.expect("second run");
    assert_eq!(first.job_folder, dir.path().join("song.stems"));
    assert_eq!(second.job_folder, dir.path().join("song.stems-2"));
    assert!(first.job_folder.join("job.json").is_file(), "never reused");
}

#[test]
fn output_override_places_stems_at_the_given_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = config(dir.path(), "song.wav", &TWO_STEM);
    let dest = dir.path().join("elsewhere").join("session-stems");
    config.output = Some(dest.clone());

    let outcome = run(&config).0.expect("job should succeed");
    assert_eq!(outcome.job_folder, dest);
    assert!(dest.join("vocals.wav").is_file());
    assert!(dest.join("job.json").is_file());
    assert!(!dir.path().join("song.stems").exists());
}

#[test]
fn output_override_is_collision_suffixed_never_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = config(dir.path(), "song.wav", &TWO_STEM);
    let dest = dir.path().join("stems");
    config.output = Some(dest.clone());

    let first = run(&config).0.expect("first run");
    let second = run(&config).0.expect("second run");
    assert_eq!(first.job_folder, dest);
    assert_eq!(second.job_folder, dir.path().join("stems-2"));
    assert!(first.job_folder.join("job.json").is_file(), "never reused");
}

#[test]
fn engine_error_event_fails_with_message_and_log_tail() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "error.wav", &TWO_STEM));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("fake engine failed on purpose"), "got: {err}");
    assert!(
        err.contains("something broke in the model"),
        "stderr tail surfaced: {err}"
    );
    assert_diagnosable_failure(&dir.path().join("error.stems"));
}

#[test]
fn engine_crash_without_error_event_reports_exit_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "crash.wav", &TWO_STEM));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("engine exited with"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("crash.stems"));
}

#[test]
fn malformed_event_stream_fails_the_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "malformed.wav", &TWO_STEM));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("malformed engine event"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("malformed.stems"));
}

#[test]
fn clean_exit_without_done_event_fails_the_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "no-done.wav", &TWO_STEM));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("without a done event"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("no-done.stems"));
}

#[test]
fn missing_input_fails_before_creating_a_job_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = JobConfig {
        input: dir.path().join("nope.wav"),
        preset: &TWO_STEM,
        parameters: serde_json::json!({}),
        device: "cpu".into(),
        model_dir: dir.path().join("models"),
        state_dir: dir.path().join("state"),
        engine_python: support::fake_engine(),
        output: None,
    };
    let err = format!("{:#}", run_job(&config, |_| ()).expect_err("should fail"));
    assert!(err.contains("input not found"), "got: {err}");
    assert!(!dir.path().join("nope.stems").exists());
}

#[test]
fn an_unknown_device_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = config(dir.path(), "song.wav", &TWO_STEM);
    config.device = "gpu".into();
    let err = format!("{:#}", run_job(&config, |_| ()).expect_err("should fail"));
    assert!(err.contains("unknown device 'gpu'"), "got: {err}");
}

/// A failed job leaves a diagnosable folder: engine.log present, no job.json
/// masquerading as completion.
fn assert_diagnosable_failure(job_folder: &Path) {
    assert!(job_folder.is_dir(), "job folder left behind");
    assert!(job_folder.join("engine.log").is_file());
    assert!(!job_folder.join("job.json").exists());
}
