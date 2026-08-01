//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! Walking-skeleton surface: `uncompose separate <song>` only.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use uncompose_core::models::{self, CurlDownloader, FetchOutcome};
use uncompose_core::registry;
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
        /// Device: auto | cpu | cuda
        #[arg(long, default_value = "auto")]
        device: String,
    },
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
        Command::Separate { song, device } => separate(song, device),
        Command::Models { command } => match command {
            ModelsCommand::List => models_list(),
            ModelsCommand::Fetch { target } => models_fetch(&target),
            ModelsCommand::Remove { id } => models_remove(&id),
        },
    }
}

fn models_list() -> Result<()> {
    let model_dir = default_model_dir();
    for status in models::list(&model_dir) {
        let state = if status.cached {
            "cached"
        } else {
            "not cached"
        };
        println!(
            "{}  [{}]  {}  ({})",
            status.entry.id,
            state,
            status.entry.hardware_tier.label(),
            status.entry.license,
        );
    }
    Ok(())
}

fn models_fetch(target: &str) -> Result<()> {
    let entries = registry::resolve(target)
        .ok_or_else(|| anyhow::anyhow!("unknown model or preset: {target}"))?;
    let model_dir = default_model_dir();
    for entry in entries {
        match models::fetch(&model_dir, entry, &CurlDownloader, |bytes| {
            print!("\r  {}: {bytes} bytes", entry.id);
            use std::io::Write;
            std::io::stdout().flush().ok();
        })? {
            FetchOutcome::AlreadyCached => println!("{}: already cached", entry.id),
            FetchOutcome::Fetched { bytes, verified } => {
                let note = if verified {
                    "verified"
                } else {
                    "unverified (no pinned hash)"
                };
                println!("\r{}: fetched {bytes} bytes, {note}", entry.id);
            }
        }
    }
    Ok(())
}

fn models_remove(id: &str) -> Result<()> {
    let entry = match registry::find(id) {
        Some(entry) => entry,
        None => bail!("unknown model: {id}"),
    };
    let model_dir = default_model_dir();
    if models::remove(&model_dir, entry)? {
        println!("{id}: removed");
    } else {
        println!("{id}: not cached");
    }
    Ok(())
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

    println!(
        "done: {} stems in {}",
        outcome.stems.len(),
        outcome.job_folder.display()
    );
    Ok(())
}
