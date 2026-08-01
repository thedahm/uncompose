//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! M1 surface: `separate <song>`, `play <stem>`, `open`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use uncompose_core::{default_model_dir, engine, run_job, state, JobConfig, JobEvent};

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
        /// Device: auto | cpu | cuda
        #[arg(long, default_value = "auto")]
        device: String,
    },
    /// Audition a stem of the last job with mpv (falling back to ffplay)
    Play {
        /// Stem name (e.g. `vocals`) or a path to an audio file
        stem: String,
    },
    /// Open the last job's folder in the file manager
    Open,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Separate { song, device } => separate(song, device),
        Command::Play { stem } => play(stem),
        Command::Open => open(),
    }
}

fn separate(song: PathBuf, device: String) -> Result<()> {
    let config = JobConfig {
        input: song.clone(),
        model_id: "htdemucs_6s".into(),
        device,
        model_dir: default_model_dir(),
        engine_python: engine::discover_engine_python()?,
    };

    println!("input:  {}", song.display());
    println!("model:  {}", config.model_id);
    println!("device: {}", config.device);

    let outcome = run_job(&config, |event| match event {
        JobEvent::Stage { stage, message, .. } => match message {
            Some(msg) => println!("[{stage}] {msg}"),
            None => println!("[{stage}]"),
        },
        JobEvent::Stem { name } => println!("  wrote {name}.wav"),
    })?;

    // Record the last-job pointer so `play` and `open` resolve without an
    // argument; a failed `separate` never reaches here, so the pointer only
    // ever names a completed job.
    state::write_last_job(&outcome.job_folder)?;

    println!(
        "done: {} stems in {}",
        outcome.stems.len(),
        outcome.job_folder.display()
    );
    Ok(())
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
    let folder = state::read_last_job()?;
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
    let folder = state::read_last_job()?;
    match Process::new("xdg-open").arg(&folder).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("xdg-open exited with {status}"),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            bail!("xdg-open not found: install xdg-utils to use `uncompose open`")
        }
        Err(e) => Err(e).context("spawning xdg-open"),
    }
}
