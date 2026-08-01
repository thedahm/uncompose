//! uncompose: thin clap CLI over uncompose-core's `run_job`.
//! Walking-skeleton surface: `uncompose separate <song>` only.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use uncompose_core::{default_model_dir, engine, run_job, Cancelled, JobConfig, JobEvent};

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
            device,
            output,
        } => separate(song, device, output),
    }
}

fn separate(song: PathBuf, device: String, output: Option<PathBuf>) -> Result<()> {
    // Survive our own Ctrl+C so we can clean up before exiting. The engine
    // shares our process group and dies on the same SIGINT; the core then
    // sees it was cancelled and removes any partial stems.
    install_sigint_handler();

    let config = JobConfig {
        input: song.clone(),
        model_id: "htdemucs_6s".into(),
        device,
        model_dir: default_model_dir(),
        engine_python: engine::discover_engine_python()?,
        output,
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
