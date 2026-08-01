//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! M1 surface: `uncompose separate <song> [--preset] [--device]`.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use uncompose_core::{
    default_model_dir, engine, preset, resolve_device, run_job, state, Cancelled, JobConfig,
    JobEvent,
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
        /// Output folder (default: `<song>.stems` next to the input)
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
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
    }
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

    println!("input:  {}", song.display());
    println!("preset: {} ({})", preset.name, preset.hardware_tier.label());
    println!("device: {device}");

    let outcome = run_job(&config, |event| match event {
        JobEvent::Stage { stage, message, .. } => match message {
            Some(msg) => println!("[{stage}] {msg}"),
            None => println!("[{stage}]"),
        },
        JobEvent::Stem { name } => println!("  wrote {name}.wav"),
    });

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(e) if e.is::<Cancelled>() => {
            eprintln!("cancelled; removed partial stems");
            // 128 + SIGINT, the conventional interrupted-by-Ctrl+C code.
            std::process::exit(130);
        }
        Err(e) => return Err(e),
    };

    println!(
        "done: {} stems in {}",
        outcome.stems.len(),
        outcome.job_folder.display()
    );
    Ok(())
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
