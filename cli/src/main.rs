//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! M1 surface: `uncompose separate <song> [--preset] [--device]`.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use uncompose_core::{
    default_model_dir, engine, preset, resolve_device, run_job, JobConfig, JobEvent,
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
        /// Preset: 6-stem (default) | 2-stem
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
    let preset = preset::by_name(&preset_name)
        .ok_or_else(|| anyhow!("unknown preset '{preset_name}': try 6-stem or 2-stem"))?;
    // Resolve the device up front so the pre-run header shows where the run
    // will actually happen, not the literal `auto`.
    let device = resolve_device(&device)?;

    let config = JobConfig {
        input: song.clone(),
        preset,
        device: device.clone(),
        model_dir: default_model_dir(),
        engine_python: engine::discover_engine_python()?,
    };

    println!("input:  {}", song.display());
    println!("preset: {} ({})", preset.name, preset.hardware_tier.label());
    println!("device: {device}");

    let outcome = run_job(&config, |event| match event {
        JobEvent::Stage { stage, message, .. } => match message {
            Some(msg) => println!("[{stage}] {msg}"),
            None => println!("[{stage}]"),
        },
        JobEvent::Stem { name } => println!("  wrote {name}.wav"),
    })?;

    println!(
        "done: {} stems in {}",
        outcome.stems.len(),
        outcome.job_folder.display()
    );
    Ok(())
}
