//! Contract tests for `run_job` against the fake engine: the core's side of
//! the Engine Contract seam (spawn, stream, failure handling, folder shape).

mod support;

use std::path::Path;

use uncompose_core::{run_job, JobConfig, JobEvent};

const STEMS: [&str; 6] = ["vocals", "drums", "bass", "guitar", "keys", "other"];

fn config(dir: &Path, input_name: &str) -> JobConfig {
    let input = dir.join(input_name);
    std::fs::write(&input, b"not really audio").expect("writing input");
    JobConfig {
        input,
        model_id: "htdemucs_6s".into(),
        device: "cpu".into(),
        model_dir: dir.join("models"),
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
fn success_produces_stems_and_job_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config(dir.path(), "song.wav");
    let (outcome, events) = run(&config);
    let outcome = outcome.expect("job should succeed");

    assert_eq!(outcome.job_folder, dir.path().join("song.stems"));
    assert_eq!(outcome.stems, STEMS);
    for stem in STEMS {
        assert!(outcome.job_folder.join(format!("{stem}.wav")).is_file());
        // Partials are promoted to final names, never left behind on success.
        assert!(!outcome
            .job_folder
            .join(format!("{stem}.wav.partial"))
            .exists());
    }
    assert!(outcome.job_folder.join("engine.log").is_file());

    // Stage events stream through before stems.
    assert!(matches!(&events[0], JobEvent::Stage { stage, .. } if stage == "model_load"));
    let stem_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Stem { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stem_events, STEMS);

    // job.json is the completion marker and reproducibility record.
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(outcome.job_folder.join("job.json")).expect("reading job.json"),
    )
    .expect("job.json is JSON");
    assert_eq!(record["model_id"], "htdemucs_6s");
    assert_eq!(record["outcome"], "success");
    assert_eq!(record["device"], "cpu");
    assert_eq!(record["engine_version"], "fake-0.0");
    assert_eq!(
        record["input_path"].as_str(),
        Some(config.input.to_string_lossy().as_ref())
    );
    assert_eq!(
        record["input_sha256"].as_str().map(str::len),
        Some(64),
        "input hash recorded"
    );
    assert_eq!(record["stems"].as_array().map(Vec::len), Some(6));
}

#[test]
fn engine_stderr_lands_in_engine_log_not_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "song.wav"));
    let log = std::fs::read_to_string(outcome.expect("success").job_folder.join("engine.log"))
        .expect("reading engine.log");
    assert!(log.contains("fake-engine: starting"));
}

#[test]
fn repeated_runs_suffix_the_job_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = config(dir.path(), "song.wav");
    let first = run(&config).0.expect("first run");
    let second = run(&config).0.expect("second run");
    assert_eq!(first.job_folder, dir.path().join("song.stems"));
    assert_eq!(second.job_folder, dir.path().join("song.stems-2"));
    assert!(first.job_folder.join("job.json").is_file(), "never reused");
}

#[test]
fn output_override_places_stems_at_the_given_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = config(dir.path(), "song.wav");
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
    let mut config = config(dir.path(), "song.wav");
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
    let (outcome, _) = run(&config(dir.path(), "error.wav"));
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
    let (outcome, _) = run(&config(dir.path(), "crash.wav"));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("engine exited with"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("crash.stems"));
}

#[test]
fn malformed_event_stream_fails_the_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "malformed.wav"));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("malformed engine event"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("malformed.stems"));
}

#[test]
fn clean_exit_without_done_event_fails_the_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (outcome, _) = run(&config(dir.path(), "no-done.wav"));
    let err = format!("{:#}", outcome.expect_err("job should fail"));
    assert!(err.contains("without a done event"), "got: {err}");
    assert_diagnosable_failure(&dir.path().join("no-done.stems"));
}

#[test]
fn missing_input_fails_before_creating_a_job_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = JobConfig {
        input: dir.path().join("nope.wav"),
        model_id: "htdemucs_6s".into(),
        device: "cpu".into(),
        model_dir: dir.path().join("models"),
        engine_python: support::fake_engine(),
        output: None,
    };
    let err = format!("{:#}", run_job(&config, |_| ()).expect_err("should fail"));
    assert!(err.contains("input not found"), "got: {err}");
    assert!(!dir.path().join("nope.stems").exists());
}

/// A failed job leaves a diagnosable folder: engine.log present, no job.json
/// masquerading as completion.
fn assert_diagnosable_failure(job_folder: &Path) {
    assert!(job_folder.is_dir(), "job folder left behind");
    assert!(job_folder.join("engine.log").is_file());
    assert!(!job_folder.join("job.json").exists());
}
