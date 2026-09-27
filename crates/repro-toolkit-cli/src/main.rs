use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "repro-toolkit", version, about = "Automotive ECU reprogramming toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a PDX file and emit its UDS repro sequence as JSON.
    Parse {
        /// Path to the input .pdx file.
        input: PathBuf,
        /// Write JSON to this file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse { input, output } => match run_parse(&input, output.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_parse(input: &std::path::Path, output: Option<&std::path::Path>) -> Result<(), String> {
    let sequence = repro_toolkit_core::parse_pdx_file(input).map_err(|e| e.to_string())?;
    let json = repro_toolkit_core::to_json_string(&sequence).map_err(|e| e.to_string())?;

    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| e.to_string())?,
        None => println!("{json}"),
    }

    Ok(())
}
