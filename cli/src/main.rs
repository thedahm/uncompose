//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! Walking-skeleton surface: `uncompose separate <song>` only.

use std::path::PathBuf;

use anyhow::Result;
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Separate { song, device } => separate(song, device),
    }
}

fn separate(song: PathBuf, device: String) -> Result<()> {
    let config = JobConfig {
        input: song.clone(),
        preset: "6-stem".into(),
        model_id: "htdemucs_6s".into(),
        parameters: serde_json::json!({}),
        device,
        model_dir: default_model_dir(),
        state_dir: state::default_state_dir(),
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

    println!(
        "done: {} stems in {}",
        outcome.stems.len(),
        outcome.job_folder.display()
    );
    Ok(())
}
