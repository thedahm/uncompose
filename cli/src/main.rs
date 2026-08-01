//! uncompose: thin clap CLI over uncompose-core's `run_job`. The interface
//! layer owns only the printed surface (#31): a pre-run header, per-stage
//! progress lines that update in place and collapse with their elapsed
//! time, and the always-on post-run hint lines. Exit code is 0 on success,
//! 130 on Ctrl+C, nonzero with an engine.log tail on stderr on failure.

use std::io::{ErrorKind, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use uncompose_core::fetch::{ensure_model, FetchEvent, HttpFetcher};
use uncompose_core::preset::{self, Preset};
use uncompose_core::registry;
use uncompose_core::{
    default_model_dir, engine, resolve_device, run_job, state, Cancelled, JobConfig, JobEvent,
};

#[derive(Parser)]
#[command(name = "uncompose", about = "Local-first music source separation")]
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
        /// A model id
        id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Separate {
            song,
            preset,
            device,
            output,
        } => separate(song, preset, device, output),
        Command::Play { stem } => play(stem),
        Command::Open => open(),
        Command::Models { command } => match command {
            ModelsCommand::List => models_list(),
            ModelsCommand::Fetch { target } => models_fetch(&target),
            ModelsCommand::Remove { id } => models_remove(&id),
        },
    }
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
        ensure_model(entry, &model_dir, &HttpFetcher, |event| match event {
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
                match total_bytes {
                    Some(total) => print!("\r  {file_name}: {downloaded}/{total} bytes"),
                    None => print!("\r  {file_name}: {downloaded} bytes"),
                }
                std::io::stdout().flush().ok();
            }
            FetchEvent::DownloadFinished { file_name } => {
                println!("\r  {file_name}: fetched and verified");
            }
        })?;
    }
    Ok(())
}

fn models_remove(id: &str) -> Result<()> {
    let entry = registry::find(id).ok_or_else(|| anyhow!("unknown model: {id}"))?;
    let model_dir = default_model_dir();
    let mut removed = false;
    for file in entry.files {
        let path = model_dir.join(file.file_name);
        if path.is_file() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            removed = true;
        }
    }
    if removed {
        println!("{id}: removed");
    } else {
        println!("{id}: not cached");
    }
    Ok(())
}

fn separate(
    song: PathBuf,
    preset_name: String,
    device: String,
    output: Option<PathBuf>,
) -> Result<()> {
    // Survive our own Ctrl+C so we can clean up before exiting. The engine
    // shares our process group and dies on the same SIGINT; the core then
    // sees it was cancelled and removes any partial stems.
    install_sigint_handler();

    let preset = preset::by_name(&preset_name)
        .ok_or_else(|| anyhow!("unknown preset '{preset_name}': try 6-stem or 2-stem"))?;
    // Resolve the device up front so the pre-run header shows where the run
    // will actually happen, not the literal `auto`.
    let device = resolve_device(&device)?;

    let config = JobConfig {
        input: song.clone(),
        preset,
        parameters: serde_json::json!({}),
        device: device.clone(),
        model_dir: default_model_dir(),
        state_dir: state::default_state_dir(),
        engine_python: engine::discover_engine_python()?,
        output,
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
    Ok(())
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
