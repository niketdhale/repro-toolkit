use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use repro_toolkit_core::Severity;

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
        /// Use this custom sequence JSON file instead of auto-generating
        /// the sequence from the PDX. See docs/custom-sequence-guide.md.
        #[arg(short, long)]
        sequence: Option<PathBuf>,
    },
    /// Validate a PDX's (or custom sequence's) repro sequence and report
    /// any issues, without printing the sequence itself.
    Validate {
        /// Path to the input .pdx file.
        input: PathBuf,
        /// Validate this custom sequence JSON file instead of the
        /// auto-generated one. See docs/custom-sequence-guide.md.
        #[arg(short, long)]
        sequence: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse {
            input,
            output,
            sequence,
        } => match run_parse(&input, output.as_deref(), sequence.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::FAILURE
            }
        },
        Command::Validate { input, sequence } => run_validate(&input, sequence.as_deref()),
    }
}

fn run_parse(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    sequence: Option<&std::path::Path>,
) -> Result<(), String> {
    let sequence =
        repro_toolkit_core::generate_sequence(input, sequence).map_err(|e| e.to_string())?;
    let json = repro_toolkit_core::to_json_string(&sequence).map_err(|e| e.to_string())?;

    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| e.to_string())?,
        None => println!("{json}"),
    }

    Ok(())
}

fn run_validate(input: &std::path::Path, sequence: Option<&std::path::Path>) -> ExitCode {
    let sequence = match repro_toolkit_core::generate_sequence(input, sequence) {
        Ok(sequence) => sequence,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let issues = repro_toolkit_core::validate_sequence(&sequence);
    if issues.is_empty() {
        println!("OK: {} step(s), no issues found", sequence.step_count);
        return ExitCode::SUCCESS;
    }

    let mut has_error = false;
    for issue in &issues {
        if issue.severity == Severity::Error {
            has_error = true;
        }
        println!("{issue}");
    }

    if has_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
