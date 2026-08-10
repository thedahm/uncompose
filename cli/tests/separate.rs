//! Contract tests for the CLI binary end to end against the fake engine:
//! printed surface, exit codes, and kill behavior. Same seam as the core
//! tests, observed one level higher.

// Same helper as the core's contract tests; included by path so the two
// suites can't drift apart.
#[path = "../../core/tests/support/mod.rs"]
mod support;

#[path = "../../core/tests/support/weights.rs"]
mod weights;

// Only `write_executable` is needed here; the provisioning suite uses the rest.
#[path = "../../core/tests/support/fake_uv.rs"]
#[allow(dead_code)]
mod fake_uv;

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// The common case: write `<dir>/<input_name>` and return a `separate`
/// command for it, wired via [`separate_cmd`] with a stub-ffmpeg-only PATH.
fn uncompose(dir: &Path, input_name: &str) -> Command {
    let input = dir.join(input_name);
    write_input(&input);
    let mut cmd = separate_cmd(dir, &bin_dir_with_ffmpeg(dir));
    cmd.arg(input);
    cmd
}

/// A `separate` command wired to the fake engine and the given hermetic `bin`
/// dir as the whole PATH (where a test places its stub `ffmpeg` and, for the
/// `--project` tests, a stub `uncompose-project`), so nothing depends on the
/// host machine. The model cache and last-job pointer stay inside the test's
/// tempdir, pre-seeded so the weight auto-fetch sees a warm cache. Callers
/// append the input positional and flags.
fn separate_cmd(dir: &Path, bin: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.arg("separate")
        .args(["--device", "cpu"])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        .env("PATH", bin)
        .env("XDG_CACHE_HOME", seeded_cache(dir))
        .env("XDG_STATE_HOME", dir.join("state"));
    cmd
}

fn write_input(path: &Path) {
    std::fs::write(path, b"not really audio").expect("writing input");
}

/// The test's XDG cache dir, with every manifest weight pre-seeded.
fn seeded_cache(dir: &Path) -> std::path::PathBuf {
    let cache = dir.join("cache");
    weights::seed_weights(&cache);
    cache
}

/// Create `<dir>/bin/ffmpeg` (executable) and return the `bin` directory, so a
/// test can point PATH at a stub ffmpeg without touching the host.
fn bin_dir_with_ffmpeg(dir: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("creating bin dir");
    let ffmpeg = bin.join("ffmpeg");
    std::fs::write(&ffmpeg, b"#!/bin/sh\n").expect("writing stub ffmpeg");
    let mut perms = std::fs::metadata(&ffmpeg).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&ffmpeg, perms).expect("chmod");
    bin
}

#[test]
fn separate_prints_header_progress_and_hints() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = uncompose(dir.path(), "song.wav")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Pre-run header: input / preset / model+license per step / device / output.
    let folder = dir.path().join("song.stems");
    assert!(stdout.contains("input"), "header input, got:\n{stdout}");
    assert!(stdout.contains("song.wav"), "input path, got:\n{stdout}");
    assert!(
        stdout.contains("preset   6-stem"),
        "preset line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("(vocals, drums, bass, guitar, keys, other)"),
        "preset stems, got:\n{stdout}"
    );
    assert!(
        stdout.contains("model") && stdout.contains("htdemucs_6s"),
        "model line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("weights: research-only"),
        "license relay, got:\n{stdout}"
    );
    assert!(
        stdout.contains("device   cpu"),
        "device line, got:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("output   {}", folder.display())),
        "output line, got:\n{stdout}"
    );

    // Per-stage lines collapse to one line with elapsed time (M:SS).
    assert!(
        has_collapsed_stage(&stdout, "model_load"),
        "model_load did not collapse with an elapsed time, got:\n{stdout}"
    );
    assert!(
        has_collapsed_stage(&stdout, "separate"),
        "separate did not collapse with an elapsed time, got:\n{stdout}"
    );
    // Stems accrue onto a single write line.
    assert!(
        stdout.contains("write") && stdout.contains("vocals") && stdout.contains("keys"),
        "write line, got:\n{stdout}"
    );

    // Always-on post-run hints.
    assert!(
        stdout.contains("✓ ") && stdout.contains("(6 stems)"),
        "success line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("play a stem:") && stdout.contains("uncompose play vocals"),
        "play hint, got:\n{stdout}"
    );
    assert!(
        stdout.contains("open folder:") && stdout.contains("uncompose open"),
        "open hint, got:\n{stdout}"
    );

    assert!(folder.join("job.json").is_file());
    // The last-job pointer records the completed job so play/open work next.
    assert!(
        dir.path().join("state/uncompose/last-job.json").is_file(),
        "last-job pointer written"
    );
}

/// A finished stage shows one line carrying the stage name and an `M:SS`
/// elapsed time; timing is nondeterministic so we only check the shape.
fn has_collapsed_stage(stdout: &str, stage: &str) -> bool {
    stdout.lines().any(|line| {
        let line = line.trim();
        line.starts_with(stage)
            && line
                .rsplit(char::is_whitespace)
                .next()
                .map(is_mmss)
                .unwrap_or(false)
    })
}

fn is_mmss(token: &str) -> bool {
    match token.split_once(':') {
        Some((m, s)) => {
            !m.is_empty()
                && m.chars().all(|c| c.is_ascii_digit())
                && s.len() == 2
                && s.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

#[test]
fn engine_failure_exits_nonzero_with_reason_and_no_job_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = uncompose(dir.path(), "error.wav")
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("fake engine failed on purpose"),
        "got:\n{stderr}"
    );
    assert!(stderr.contains("engine.log tail"), "got:\n{stderr}");
    let folder = dir.path().join("error.stems");
    assert!(folder.join("engine.log").is_file(), "diagnosable folder");
    assert!(!folder.join("job.json").exists(), "must not look complete");
}

#[test]
fn missing_input_fails_with_a_clear_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    let output = cmd
        .args(["separate", dir.path().join("nope.wav").to_str().unwrap()])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        .env("PATH", bin_dir_with_ffmpeg(dir.path()))
        .env("XDG_CACHE_HOME", dir.path().join("cache"))
        .output()
        .expect("running CLI");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("input not found"));
}

#[test]
fn missing_ffmpeg_stops_up_front_with_an_install_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("song.wav");
    std::fs::write(&input, b"not really audio").expect("writing input");
    // An empty bin dir on PATH: no ffmpeg to be found.
    let empty_bin = dir.path().join("empty-bin");
    std::fs::create_dir_all(&empty_bin).expect("creating empty bin dir");

    let output = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .args(["separate", input.to_str().expect("utf8 path")])
        .args(["--device", "cpu"])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        .env("PATH", &empty_bin)
        .env("XDG_CACHE_HOME", dir.path().join("cache"))
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("ffmpeg not found"),
        "clear message, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sudo apt install ffmpeg"),
        "install hint, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "no stack trace, got:\n{stderr}"
    );
    // Up front: it bails before creating a job folder or spawning the engine.
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder should be created"
    );
}

#[test]
fn output_flag_writes_stems_to_the_given_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("my-stems");
    let output = uncompose(dir.path(), "song.wav")
        .args(["--output", dest.to_str().expect("utf8 path")])
        .output()
        .expect("running CLI");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dest.join("vocals.wav").is_file());
    assert!(dest.join("job.json").is_file());
    assert!(
        !dir.path().join("song.stems").exists(),
        "-o replaced the default location"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("✓ {}", dest.display())) && stdout.contains("(6 stems)"),
        "success line names the override, got:\n{stdout}"
    );
}

#[test]
fn sigint_mid_run_kills_the_job_without_a_job_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A real Ctrl+C is delivered to the foreground process group, CLI and
    // engine both; give the CLI its own group so the test can do the same
    // without the engine surviving as an orphan.
    let mut child = uncompose(dir.path(), "hang.wav")
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("spawning CLI");

    // Wait until the engine is provably mid-run before interrupting.
    let stdout = child.stdout.take().expect("piped stdout");
    let mut saw_stage = false;
    for line in BufReader::new(stdout).lines() {
        if line.expect("reading CLI stdout").trim() == "separate" {
            saw_stage = true;
            break;
        }
    }
    assert!(saw_stage, "CLI never reached the hanging stage");

    let kill = Command::new("kill")
        .args(["-INT", "--", &format!("-{}", child.id())])
        .status()
        .expect("sending SIGINT");
    assert!(kill.success());

    let status = child.wait().expect("waiting for CLI");
    assert!(!status.success());
    let folder = dir.path().join("hang.stems");
    assert!(folder.is_dir(), "job folder left as the artifact");
    assert!(!folder.join("job.json").exists(), "must not look complete");
    // Ctrl+C leaves no half-written junk: every staged partial is removed.
    let partials: Vec<_> = walk_partials(&folder);
    assert!(partials.is_empty(), "partials left behind: {partials:?}");
}

// ---------------------------------------------------------------------------
// `separate --project`: pre-flight and chained registration (M5 slice 3).
// ---------------------------------------------------------------------------

const PROJECT_SCHEMA: &str =
    "https://uncompose.org/schemas/project/v0/uncompose.project.schema.json";

/// Write `<root>/uncompose.project.json` carrying `schema` (only field the
/// pre-flight inspects; strict validation is uncompose-project's job).
fn write_manifest(root: &Path, schema: &str) {
    std::fs::create_dir_all(root).expect("creating project root");
    std::fs::write(
        root.join("uncompose.project.json"),
        format!(r#"{{"schema": "{schema}"}}"#),
    )
    .expect("writing manifest");
}

/// A stub `uncompose-project` on the synthetic PATH: records each argv token
/// (one per line) to `log`, optionally writes `stderr`, and exits `code` — the
/// same substitution-at-the-process-boundary pattern as the fake engine / uv.
fn stub_project(bin: &Path, log: &Path, code: i32, stderr: &str) {
    let err_line = if stderr.is_empty() {
        String::new()
    } else {
        format!("echo '{stderr}' >&2\n")
    };
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"{log}\"\n{err_line}exit {code}\n",
        log = log.display(),
    );
    fake_uv::write_executable(&bin.join("uncompose-project"), &script);
}

#[test]
fn project_missing_manifest_refuses_before_the_engine_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    stub_project(&bin, &dir.path().join("argv.log"), 0, "");
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", dir.path().to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("uncompose.project.json"),
        "names the expected manifest, got:\n{stderr}"
    );
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder: the engine never ran"
    );
}

#[test]
fn project_wrong_schema_url_refuses_before_the_engine_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    stub_project(&bin, &dir.path().join("argv.log"), 0, "");
    write_manifest(dir.path(), "https://example.com/not-a-project.json");
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", dir.path().to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("schema") && stderr.contains(PROJECT_SCHEMA),
        "names the expected schema, got:\n{stderr}"
    );
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder: the engine never ran"
    );
}

#[test]
fn project_missing_uncompose_project_refuses_with_install_hint() {
    let dir = tempfile::tempdir().expect("tempdir");
    // bin has ffmpeg but no uncompose-project.
    let bin = bin_dir_with_ffmpeg(dir.path());
    write_manifest(dir.path(), PROJECT_SCHEMA);
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", dir.path().to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("uncompose-project") && stderr.contains("uv tool install"),
        "install hint, got:\n{stderr}"
    );
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder: the engine never ran"
    );
}

#[test]
fn project_out_of_root_input_refuses_before_the_engine_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    stub_project(&bin, &dir.path().join("argv.log"), 0, "");
    // The manifest lives in a subdirectory; the input sits outside it.
    let root = dir.path().join("proj");
    write_manifest(&root, PROJECT_SCHEMA);
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", root.to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("outside the project root"),
        "explains why, got:\n{stderr}"
    );
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder: the engine never ran"
    );
}

#[test]
fn project_out_of_root_output_override_refuses_before_the_engine_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    stub_project(&bin, &dir.path().join("argv.log"), 0, "");
    let root = dir.path().join("proj");
    write_manifest(&root, PROJECT_SCHEMA);
    let input = root.join("song.wav");
    write_input(&input);
    // -o points outside the project root.
    let outside = dir.path().join("elsewhere");

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", root.to_str().unwrap()])
        .args(["--output", outside.to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("outside the project root"),
        "explains why, got:\n{stderr}"
    );
    assert!(!outside.exists(), "no job folder: the engine never ran");
}

#[test]
fn project_out_of_root_output_via_dotdot_refuses_before_the_engine_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    stub_project(&bin, &dir.path().join("argv.log"), 0, "");
    let root = dir.path().join("proj");
    write_manifest(&root, PROJECT_SCHEMA);
    let input = root.join("song.wav");
    write_input(&input);
    // -o escapes the root through a not-yet-existing folder: the path stays
    // under `proj` textually until the `..`s fold it out to `elsewhere`.
    let sneaky = root.join("missing/../../elsewhere");

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", root.to_str().unwrap()])
        .args(["--output", sneaky.to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("outside the project root"),
        "explains why, got:\n{stderr}"
    );
    assert!(
        !dir.path().join("elsewhere").exists(),
        "no job folder: the engine never ran"
    );
}

#[test]
fn project_chained_success_passes_the_pinned_argv_and_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    let argv_log = dir.path().join("argv.log");
    stub_project(&bin, &argv_log, 0, "");
    write_manifest(dir.path(), PROJECT_SCHEMA);
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", dir.path().to_str().unwrap()])
        .output()
        .expect("running CLI");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let logged = std::fs::read_to_string(&argv_log).expect("stub recorded no argv");
    let lines: Vec<&str> = logged.lines().collect();
    let root = dir.path().canonicalize().expect("canonical root");
    let job_json = dir
        .path()
        .join("song.stems/job.json")
        .canonicalize()
        .expect("canonical job.json");
    assert_eq!(
        lines,
        [
            "import",
            "--project",
            root.to_str().unwrap(),
            job_json.to_str().unwrap(),
        ],
        "exact pinned argv with absolute paths"
    );
}

#[test]
fn project_chained_failure_keeps_the_job_and_prints_recovery_last() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    let argv_log = dir.path().join("argv.log");
    stub_project(&bin, &argv_log, 3, "import blew up");
    write_manifest(dir.path(), PROJECT_SCHEMA);
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .args(["--project", dir.path().to_str().unwrap()])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The import's own exit code is carried, not flattened to a generic 1
    // (ADR-0006): the stub exits 3, so the CLI must exit 3.
    assert_eq!(
        output.status.code(),
        Some(3),
        "the import's own exit code is carried, got:\n{stderr}"
    );
    // The separation is intact: the job folder and job.json stay untouched.
    let job_json = dir.path().join("song.stems/job.json");
    assert!(job_json.is_file(), "job.json left intact");
    // The import's stderr is relayed.
    assert!(
        stderr.contains("import blew up"),
        "relayed import stderr, got:\n{stderr}"
    );
    // The recovery command is the exact human form, printed last.
    let last = stderr.lines().rfind(|l| !l.trim().is_empty());
    assert_eq!(
        last,
        Some(
            format!(
                "re-register with: uncompose project import {}",
                job_json.canonicalize().unwrap().display()
            )
            .as_str()
        ),
        "recovery command printed last, got:\n{stderr}"
    );
}

#[test]
fn separate_without_project_does_not_register() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = bin_dir_with_ffmpeg(dir.path());
    // A stub is present but must never be invoked without --project.
    let argv_log = dir.path().join("argv.log");
    stub_project(&bin, &argv_log, 0, "");
    let input = dir.path().join("song.wav");
    write_input(&input);

    let output = separate_cmd(dir.path(), &bin)
        .arg(&input)
        .output()
        .expect("running CLI");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        dir.path().join("song.stems/job.json").is_file(),
        "separated as usual"
    );
    assert!(
        !argv_log.exists(),
        "no registration attempted without --project"
    );
}

/// Collect `*.partial` files anywhere under the folder (stage scratch dirs
/// included) so the cancel test proves the whole tree is clean.
fn walk_partials(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk_partials(&path));
        } else if path.extension().is_some_and(|x| x == "partial") {
            found.push(path);
        }
    }
    found
}
