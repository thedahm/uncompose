//! uncompose: thin clap CLI over uncompose-core's `run_job`. The interface
//! layer owns only the printed surface (#31): a 5-line pre-run header, per-
//! stage progress lines that update in place and collapse with their elapsed
//! time, and the always-on post-run hint lines. Exit code is 0 on success,
//! nonzero with an engine.log tail on stderr on failure.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use uncompose_core::preset::{self, Preset};
use uncompose_core::{default_model_dir, engine, run_job, JobConfig, JobEvent};

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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Separate {
            song,
            preset,
            device,
        } => separate(song, preset, device),
    }
}

fn separate(song: PathBuf, preset_name: String, device: String) -> Result<()> {
    let preset = preset::lookup(&preset_name)
        .ok_or_else(|| anyhow!("unknown preset: {preset_name} (try 6-stem or 2-stem)"))?;

    let config = JobConfig {
        input: song.clone(),
        model_id: preset.model_id.into(),
        device: device.clone(),
        model_dir: default_model_dir(),
        engine_python: engine::discover_engine_python()?,
    };

    let mut progress = Progress::new();
    let outcome = run_job(&config, |event| match event {
        JobEvent::Started { job_folder } => print_header(&song, preset, &device, &job_folder),
        JobEvent::Stage { stage, percent, .. } => progress.stage(&stage, percent),
        JobEvent::Stem { name } => progress.stem(&name),
    })?;
    progress.finish();

    print_hints(
        &outcome.job_folder,
        outcome.stems.len(),
        outcome.stems.first(),
    );
    Ok(())
}

/// The 5-line pre-run header, printed once the job folder is resolved and
/// before any slow work: input / preset / model (with relayed license) /
/// device / output.
fn print_header(song: &Path, preset: &Preset, device: &str, job_folder: &Path) {
    println!("  {:<8} {}", "input", song.display());
    println!(
        "  {:<8} {}  ({})",
        "preset",
        preset.id,
        preset.stems.join(", ")
    );
    println!(
        "  {:<8} {}  — weights: {}",
        "model", preset.model_id, preset.license
    );
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
