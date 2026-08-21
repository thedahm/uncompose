//! uncompose: thin clap CLI over uncompose-core's `run_job`. The interface
//! layer owns only the printed surface (#31): a pre-run header, per-stage
//! progress lines that update in place and collapse with their elapsed
//! time, and the always-on post-run hint lines. Exit code is 0 on success,
//! 130 on Ctrl+C, nonzero with an engine.log tail on stderr on failure.

use std::io::{ErrorKind, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command as Process;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

mod dispatch;
use uncompose_core::fetch::{self, ensure_model, FetchEvent, HttpFetcher};
use uncompose_core::preset::{self, Preset};
use uncompose_core::registry;
use uncompose_core::{
    default_model_dir, engine, ensure_ffmpeg, resolve_device, run_job, state, Cancelled, JobConfig,
    JobEvent,
};

#[derive(Parser)]
#[command(
    name = "uncompose",
    version,
    about = "Local-first music source separation"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Separate a song into stems
    Separate {
        /// Input audio file (WAV/MP3)
        song: PathBuf,
        /// Separation preset: 6-stem | 2-stem
        #[arg(long, default_value = "6-stem")]
        preset: String,
        /// Device: auto | cpu | cuda
        #[arg(long, default_value = "auto")]
        device: String,
        /// Output folder (default: `<song>.stems` next to the input)
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Register the finished job in the project at this directory
        /// (`<dir>/uncompose.project.json`). Pre-flighted before any work.
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Audition a stem of the last job with mpv (falling back to ffplay)
    Play {
        /// Stem name (e.g. `vocals`) or a path to an audio file
        stem: String,
    },
    /// Open the last job's folder in the file manager
    Open,
    /// Inspect and manage cached model weights
    Models {
        #[command(subcommand)]
        command: ModelsCommand,
    },
}

#[derive(Subcommand)]
enum ModelsCommand {
    /// List known models with license status, Hardware Tier, and cache state
    List,
    /// Pre-download a model or preset's weights into the cache
    Fetch {
        /// A model id (e.g. htdemucs_6s) or preset name (e.g. 6-stem)
        target: String,
    },
    /// Remove a model's cached weights to reclaim disk space
    Remove {
        /// A model id (omit with --all)
        #[arg(required_unless_present = "all")]
        id: Option<String>,
        /// Remove every cached model's weight files, not just one
        #[arg(long, conflicts_with = "id")]
        all: bool,
    },
}

fn main() -> Result<()> {
    // Git-style external-command dispatch happens before clap sees argv: an
    // eligible first token `exec()`s `uncompose-<token>` and never returns here.
    dispatch::maybe_dispatch();
    let cli = parse_with_external_commands_help();
    match cli.command {
        Command::Separate {
            song,
            preset,
            device,
            output,
            project,
        } => separate(song, preset, device, output, project),
        Command::Play { stem } => play(stem),
        Command::Open => open(),
        Command::Models { command } => match command {
            ModelsCommand::List => models_list(),
            ModelsCommand::Fetch { target } => models_fetch(&target),
            ModelsCommand::Remove { id, all } => models_remove(id.as_deref(), all),
        },
    }
}

/// `Cli::parse()`, with the root help extended by an
/// "External commands (installed):" section listing the executable
/// `uncompose-*` names found on `PATH` (ADR-0005). The section is a directory
/// scan — names only, nothing is executed — and is omitted entirely when no
/// extension is installed.
fn parse_with_external_commands_help() -> Cli {
    let mut cmd = Cli::command();
    let extensions = dispatch::installed_extensions();
    if !extensions.is_empty() {
        let mut section = String::from("External commands (installed):\n");
        for name in &extensions {
            section.push_str(&format!("  {name}\n"));
        }
        // after_help keeps clap's builtin Commands/Options blocks first; trim
        // the trailing newline so clap's own spacing stays intact.
        cmd = cmd.after_help(section.trim_end().to_string());
    }
    let matches = cmd.get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
}

fn models_list() -> Result<()> {
    let model_dir = default_model_dir();
    for entry in registry::MANIFEST {
        let cached = entry
            .files
            .iter()
            .all(|f| model_dir.join(f.file_name).is_file());
        let state = if cached { "cached" } else { "not cached" };
        println!(
            "{}  [{}]  {}  ({})",
            entry.id,
            state,
            entry.hardware_tier.label(),
            entry.license.label,
        );
    }
    Ok(())
}

fn models_fetch(target: &str) -> Result<()> {
    let entries =
        registry::resolve(target).ok_or_else(|| anyhow!("unknown model or preset: {target}"))?;
    let model_dir = default_model_dir();
    for entry in entries {
        // The manifest digests are pinned at M1 acceptance; until then a
        // fetch would fail closed inside ensure_model anyway — say why up
        // front instead of after a download.
        if entry.files.iter().any(|f| f.sha256.is_empty()) {
            bail!(
                "{}: no download pin yet (SHA-256 unpinned); fetch is enabled once \
                 the manifest digests are pinned",
                entry.id
            );
        }
        ensure_model(entry, &model_dir, &HttpFetcher, print_fetch_event)?;
    }
    Ok(())
}

/// One printed line per fetch event: the license relay, then a byte-progress
/// line per file. Shared by `models fetch` and the `separate` auto-fetch so
/// the two surfaces cannot drift.
fn print_fetch_event(event: FetchEvent) {
    match event {
        FetchEvent::License { model_id, license } => {
            println!("{model_id}: weights are {}", license.label);
        }
        FetchEvent::Cached { file_name } => println!("  {file_name}: already cached"),
        FetchEvent::DownloadStarted { file_name, .. } => {
            println!("  {file_name}: downloading");
        }
        FetchEvent::DownloadProgress {
            file_name,
            downloaded,
            total_bytes,
        } => {
            // \x1b[K erases the rest of the line: the finished line is
            // shorter than the byte counter it overwrites.
            match total_bytes {
                Some(total) => print!("\r  {file_name}: {downloaded}/{total} bytes\x1b[K"),
                None => print!("\r  {file_name}: {downloaded} bytes\x1b[K"),
            }
            std::io::stdout().flush().ok();
        }
        FetchEvent::DownloadFinished { file_name } => {
            println!("\r  {file_name}: fetched and verified\x1b[K");
        }
    }
}

fn models_remove(id: Option<&str>, all: bool) -> Result<()> {
    let model_dir = default_model_dir();
    if all {
        // One stuck file must not strand the rest of the sweep: `--all` is the
        // whole cache in one pass, and a user told nothing about the models it
        // never reached cannot act on them. Failing at the end keeps the exit
        // code honest.
        let mut failed = 0;
        for entry in registry::MANIFEST {
            if let Err(err) = remove_cached_entry(entry, &model_dir) {
                eprintln!("{}: {err:#}", entry.id);
                failed += 1;
            }
        }
        if failed > 0 {
            bail!(
                "{failed} of {} models could not be removed",
                registry::MANIFEST.len()
            );
        }
        return Ok(());
    }
    let id = id.expect("clap requires id when --all is absent");
    let entry = registry::find(id).ok_or_else(|| anyhow!("unknown model: {id}"))?;
    remove_cached_entry(entry, &model_dir)
}

/// One removal and printed line per manifest entry. Shared by `models remove
/// <id>` and `models remove --all` so the two surfaces cannot drift.
fn remove_cached_entry(entry: &registry::ModelEntry, model_dir: &Path) -> Result<()> {
    let mut removed = false;
    for file in entry.files {
        let path = model_dir.join(file.file_name);
        if path.is_file() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            removed = true;
        }
    }
    if removed {
        println!("{}: removed", entry.id);
    } else {
        println!("{}: not cached", entry.id);
    }
    Ok(())
}

fn separate(
    song: PathBuf,
    preset_name: String,
    device: String,
    output: Option<PathBuf>,
    project: Option<PathBuf>,
) -> Result<()> {
    // Survive our own Ctrl+C so we can clean up before exiting. The engine
    // shares our process group and dies on the same SIGINT; the core then
    // sees it was cancelled and removes any partial stems.
    install_sigint_handler();

    // In project mode, catch every foreseeable failure before any engine or
    // provisioning work: a doomed run must fail in milliseconds, not after
    // minutes of inference (#67 res. 4). Returns the canonical project root
    // for the chained registration on the success path.
    let project_root = match project {
        Some(dir) => Some(preflight_project(&dir, &song, output.as_deref())?),
        None => None,
    };

    // ffmpeg is a checked system dependency: fail up front with an install
    // message rather than a cryptic engine stack trace once the run starts.
    ensure_ffmpeg()?;

    let preset = preset::by_name(&preset_name)
        .ok_or_else(|| anyhow!("unknown preset '{preset_name}': try 6-stem or 2-stem"))?;
    // Resolve the device up front so the pre-run header shows where the run
    // will actually happen, not the literal `auto`.
    let device = resolve_device(&device)?;

    // Check the input before any slow first-run work: a typo'd path must
    // not cost a multi-GB weight download first. run_job re-checks when it
    // canonicalizes; this is the fail-fast copy of the same message.
    if !song.is_file() {
        bail!("input not found: {}", song.display());
    }

    let model_dir = default_model_dir();
    // Weights auto-fetch on first use of a preset (story 8), before the
    // engine environment resolves so the two slow first-run surprises
    // arrive in one visible block. Warm caches stay quiet.
    let entries = registry::resolve(preset.name).ok_or_else(|| {
        anyhow!(
            "preset '{}' names models missing from the manifest",
            preset.name
        )
    })?;
    fetch::ensure_weights(&entries, &model_dir, &HttpFetcher, print_fetch_event)?;

    let config = JobConfig {
        input: song.clone(),
        preset,
        parameters: serde_json::json!({}),
        device: device.clone(),
        model_dir,
        state_dir: state::default_state_dir(),
        engine_python: engine::resolve_engine_python(print_provision_event)?,
        output,
        project_root: project_root.clone(),
    };

    let mut progress = Progress::new();
    let outcome = run_job(&config, |event| match event {
        JobEvent::Started { job_folder } => print_header(&song, preset, &device, &job_folder),
        JobEvent::Stage { stage, percent, .. } => progress.stage(&stage, percent),
        JobEvent::Stem { name } => progress.stem(&name),
    });

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(e) if e.is::<Cancelled>() => {
            progress.finish();
            eprintln!("cancelled; removed partial stems");
            // 128 + SIGINT, the conventional interrupted-by-Ctrl+C code.
            std::process::exit(130);
        }
        Err(e) => return Err(e),
    };
    progress.finish();

    print_hints(
        &outcome.job_folder,
        outcome.stems.len(),
        outcome.stems.first(),
    );

    // The separation is complete and job.json (the completion marker) is
    // written. In project mode, chain the registration so exit 0 means
    // separated *and* registered (#67 res. 7). A registration failure never
    // costs the separation: the job folder stays intact.
    if let Some(root) = project_root {
        register_job(&root, &outcome.job_folder)?;
    }
    Ok(())
}

/// The exact-string project manifest `schema` URL that pre-flight matches
/// (#67 res. 9). Full strict validation stays uncompose-project's job; here
/// the URL match is only the "is this a project manifest at all" gate.
const PROJECT_SCHEMA_URL: &str =
    "https://uncompose.org/schemas/project/v0/uncompose.project.schema.json";

/// Pre-flight `separate --project` before any engine or provisioning work:
/// the manifest exists at the fixed location and carries the project schema
/// URL, `uncompose-project` is on PATH, and both the input and the destination
/// job folder resolve inside the project root. Returns the canonical project
/// root for the later chained import.
fn preflight_project(dir: &Path, song: &Path, output: Option<&Path>) -> Result<PathBuf> {
    // `--project <dir>` names the project root itself; the manifest must be
    // exactly `<dir>/uncompose.project.json`, with no upward walk (#67 res. 4).
    let root = dir
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", dir.display()))?;
    let manifest = root.join("uncompose.project.json");
    let bytes = std::fs::read(&manifest).map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            anyhow!(
                "no project manifest at {} \
                 (--project names the project root itself; no parent directories are searched)",
                manifest.display()
            )
        } else {
            anyhow!("reading {}: {e}", manifest.display())
        }
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("{} is not valid JSON", manifest.display()))?;
    match value.get("schema").and_then(|s| s.as_str()) {
        Some(PROJECT_SCHEMA_URL) => {}
        other => bail!(
            "{} is not a v0 project manifest: expected schema {PROJECT_SCHEMA_URL}, found {}",
            manifest.display(),
            other
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "no schema field".to_string())
        ),
    }

    // The registrar must be runnable, else fail fast with the family install
    // hint (#67 res. 1) — the same hint dispatch prints on a `project` miss.
    if !dispatch::on_path("uncompose-project") {
        bail!(
            "uncompose-project not found on PATH (required to register with --project)\n\
             install it with: uv tool install uncompose-project"
        );
    }

    // Input inside the root — else import would refuse the job after the fact.
    let input = song
        .canonicalize()
        .with_context(|| format!("input not found: {}", song.display()))?;
    if !input.starts_with(&root) {
        bail!(
            "input {} is outside the project root {}",
            input.display(),
            root.display()
        );
    }

    // The destination job folder (default `<input>.stems`, or the `-o`
    // override) inside the root, refused here rather than after inference.
    let base = uncompose_core::job::job_folder_base(&input, output)?;
    let base_resolved = resolve_lexically(&base)?;
    if !base_resolved.starts_with(&root) {
        bail!(
            "output folder {} would land outside the project root {}",
            base.display(),
            root.display()
        );
    }

    Ok(root)
}

/// Resolve a possibly-not-yet-existing path to an absolute one, so an `-o`
/// override can be range-checked against the project root before the folder
/// is created: canonicalize the deepest existing ancestor (the filesystem
/// resolves its symlinks and `..`), then fold the not-yet-existing
/// remainder's `.`/`..` components lexically — sound there because nothing on
/// the remainder exists, so no symlink can bend what `..` means.
fn resolve_lexically(path: &Path) -> Result<PathBuf> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolving output path against the current directory")?
            .join(path)
    };
    let Some((base, remainder)) = abs.ancestors().find_map(|ancestor| {
        let canonical = ancestor.canonicalize().ok()?;
        let remainder = abs.strip_prefix(ancestor).ok()?;
        Some((canonical, remainder))
    }) else {
        // Not even the filesystem root canonicalizes; nothing better to offer.
        return Ok(abs.clone());
    };
    let mut resolved = base;
    for component in remainder.components() {
        match component {
            Component::Normal(name) => resolved.push(name),
            // Popping at the base's root is a no-op, the kernel's own `/..`
            // rule.
            Component::ParentDir => {
                resolved.pop();
            }
            // `.` is dropped; RootDir/Prefix cannot appear in a remainder.
            _ => {}
        }
    }
    Ok(resolved)
}

/// Register the finished job with `uncompose-project` via the pinned cross-tool
/// argv (`uncompose-project import --project <abs-root> <abs-job.json>`, #67
/// res. 9). On failure the separation is never the casualty: the job folder
/// and job.json are already written and stay untouched; the import's stderr is
/// relayed (inherited), and we exit nonzero after printing the exact recovery
/// command, which idempotency makes safe to rerun (#67 res. 7).
fn register_job(root: &Path, job_folder: &Path) -> Result<()> {
    let job_json = job_folder
        .join("job.json")
        .canonicalize()
        .context("locating job.json for registration")?;
    let status = Process::new("uncompose-project")
        .arg("import")
        .arg("--project")
        .arg(root)
        .arg(&job_json)
        .status()
        .context("running uncompose-project import")?;
    if status.success() {
        println!();
        println!(
            "✓ registered in {}",
            root.join("uncompose.project.json").display()
        );
        return Ok(());
    }
    eprintln!();
    eprintln!("registration failed; the separation is intact.");
    eprintln!(
        "re-register with: uncompose project import {}",
        job_json.display()
    );
    // The status is a failure, so any code here is the import's own nonzero
    // one; a signal death (no code) exits 1.
    std::process::exit(status.code().unwrap_or(1));
}

/// The last job's folder from the pointer `separate` writes on success;
/// `play`/`open` resolve against it so they work without an argument.
fn last_job_folder() -> Result<PathBuf> {
    state::read_last_job(&state::default_state_dir())?
        .map(|last| last.job_folder)
        .ok_or_else(|| anyhow!("no completed job yet: run `uncompose separate` first"))
}

/// Audition a stem: resolve the target, then shell out to a player. mpv is
/// preferred; ffplay is the fallback; if neither is installed, say so plainly
/// instead of leaking a spawn error.
fn play(stem: String) -> Result<()> {
    let target = resolve_stem(&stem)?;
    for player in ["mpv", "ffplay"] {
        let mut cmd = Process::new(player);
        // ffplay otherwise opens a video window and waits; keep the audition
        // audio-only and self-terminating.
        if player == "ffplay" {
            cmd.args(["-autoexit", "-nodisp"]);
        }
        cmd.arg(&target);
        match cmd.status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => bail!("{player} exited with {status}"),
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e).with_context(|| format!("spawning {player}")),
        }
    }
    bail!("no audio player found: install mpv or ffplay to use `uncompose play`")
}

/// A stem argument is either a path to an existing audio file (path
/// addressable) or a stem name resolved against the last job's folder.
fn resolve_stem(stem: &str) -> Result<PathBuf> {
    let as_path = Path::new(stem);
    if as_path.is_file() {
        return Ok(as_path.to_path_buf());
    }
    let folder = last_job_folder()?;
    let name = if stem.ends_with(".wav") {
        stem.to_string()
    } else {
        format!("{stem}.wav")
    };
    let target = folder.join(&name);
    if !target.is_file() {
        bail!(
            "stem not found: {} (looked in last job {})",
            name,
            folder.display()
        );
    }
    Ok(target)
}

/// Open the last job's folder with xdg-open.
fn open() -> Result<()> {
    let folder = last_job_folder()?;
    match Process::new("xdg-open").arg(&folder).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("xdg-open exited with {status}"),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            bail!("xdg-open not found: install xdg-utils to use `uncompose open`")
        }
        Err(e) => Err(e).context("spawning xdg-open"),
    }
}

/// Announce the one-time Engine Environment build before it starts: the
/// multi-GB download must never look like a hang, and the CPU-only escape
/// hatch has to be visible before the CUDA torch download begins. uv's own
/// progress output follows on stderr.
fn print_provision_event(event: uncompose_core::provision::ProvisionEvent) {
    use uncompose_core::provision::ProvisionEvent;
    match event {
        ProvisionEvent::Started { env_dir } => {
            println!(
                "first run: building the engine environment in {}",
                env_dir.display()
            );
            println!(
                "  one-time download of the ML stack, several GB (CUDA torch by default; \
                 rerun with UV_TORCH_BACKEND=cpu for a CPU-only machine)"
            );
        }
        ProvisionEvent::Step { description } => println!("  {description}"),
    }
}

/// The pre-run header, printed once the job folder is resolved and before
/// any slow work: input / preset / one model line per pipeline step (with
/// relayed license) / device / output.
fn print_header(song: &Path, preset: &Preset, device: &str, job_folder: &Path) {
    println!("  {:<8} {}", "input", song.display());
    println!(
        "  {:<8} {}  ({})",
        "preset",
        preset.name,
        preset.stems.join(", ")
    );
    for step in preset.steps {
        println!(
            "  {:<8} {}  — weights: {}",
            "model", step.model.id, step.model.license
        );
    }
    println!("  {:<8} {}", "device", device);
    println!("  {:<8} {}", "output", job_folder.display());
    println!();
}

/// The always-on post-run hints: the two next commands a user reaches for.
fn print_hints(job_folder: &Path, stem_count: usize, first_stem: Option<&String>) {
    let stem = first_stem.map(String::as_str).unwrap_or("vocals");
    println!();
    println!("✓ {}  ({stem_count} stems)", job_folder.display());
    println!();
    println!("  {:<14} uncompose play {stem}", "play a stem:");
    println!("  {:<14} uncompose open", "open folder:");
}

/// Renders per-stage progress. On a terminal each stage updates in place and
/// collapses to a single line with its elapsed time when the next stage
/// begins; stems accrue onto one `write` line. When stdout is not a terminal
/// (pipes, tests) it emits plain, observable lines instead of `\r` redraws.
struct Progress {
    tty: bool,
    stage: Option<(String, Instant)>,
    stems: Vec<String>,
    write_committed: bool,
}

impl Progress {
    fn new() -> Self {
        Progress {
            tty: std::io::stdout().is_terminal(),
            stage: None,
            stems: Vec::new(),
            write_committed: false,
        }
    }

    fn stage(&mut self, name: &str, percent: Option<f64>) {
        if let Some((current, _)) = &self.stage {
            if current == name {
                self.redraw(name, percent);
                return;
            }
            self.finalize_stage();
        }
        self.stage = Some((name.to_string(), Instant::now()));
        if self.tty {
            self.redraw(name, percent);
        } else {
            // A committed line so a mid-run stage is observable on a pipe.
            println!("  {name}");
        }
    }

    fn stem(&mut self, name: &str) {
        // The first stem ends the separation stage.
        self.finalize_stage();
        self.stems.push(name.to_string());
        if self.tty {
            print!("\r  {:<9} {}\x1b[K", "write", self.stems.join("  "));
            let _ = std::io::stdout().flush();
        }
    }

    fn finish(&mut self) {
        self.finalize_stage();
        self.finalize_write();
    }

    /// The live, in-place stage line (terminal only).
    fn redraw(&self, name: &str, percent: Option<f64>) {
        if !self.tty {
            return;
        }
        let pct = percent.map(|p| format!("  {p:.0}%")).unwrap_or_default();
        print!("\r  {name:<9} …{pct}\x1b[K");
        let _ = std::io::stdout().flush();
    }

    /// Collapse the active stage to one committed line with elapsed time.
    fn finalize_stage(&mut self) {
        if let Some((name, start)) = self.stage.take() {
            let line = format!("  {name:<9} {}", fmt_elapsed(start.elapsed().as_secs()));
            self.commit(&line);
        }
    }

    fn finalize_write(&mut self) {
        if self.stems.is_empty() || self.write_committed {
            return;
        }
        self.write_committed = true;
        let line = format!("  {:<9} {}", "write", self.stems.join("  "));
        self.commit(&line);
    }

    /// Write a finished line: overwrite the in-place draw on a terminal,
    /// otherwise a plain line.
    fn commit(&self, line: &str) {
        if self.tty {
            print!("\r{line}\x1b[K\n");
            let _ = std::io::stdout().flush();
        } else {
            println!("{line}");
        }
    }
}

fn fmt_elapsed(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

extern "C" fn on_sigint(_sig: libc::c_int) {}

/// Replace the default SIGINT disposition with a no-op handler: on Ctrl+C the
/// engine (same process group) still dies, but this process keeps running long
/// enough for the core to clean up and report the cancellation.
///
/// It must be a real handler, not `SIG_IGN`: exec resets *handled* signals to
/// their default, so the spawned engine still dies on SIGINT, but it *inherits*
/// `SIG_IGN` — which would make the engine ignore Ctrl+C and never stop.
fn install_sigint_handler() {
    // SAFETY: the handler does nothing, so it is trivially async-signal-safe.
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
    }
}
