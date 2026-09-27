use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use repro_toolkit_core::{GenerateOptions, Severity, DEFAULT_MAX_BLOCK_LENGTH};

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
        /// TransferData maxNumberOfBlockLength (includes SID + counter
        /// bytes), decimal or 0x-prefixed hex. Ignored for custom sequences.
        #[arg(short = 'b', long, value_parser = parse_u32, default_value_t = DEFAULT_MAX_BLOCK_LENGTH)]
        max_block_length: u32,
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
        /// TransferData maxNumberOfBlockLength, as for `parse`.
        #[arg(short = 'b', long, value_parser = parse_u32, default_value_t = DEFAULT_MAX_BLOCK_LENGTH)]
        max_block_length: u32,
    },
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let result = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => s.parse(),
    };
    result.map_err(|e| format!("invalid number '{s}': {e}"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse {
            input,
            output,
            sequence,
            max_block_length,
        } => {
            let options = GenerateOptions { max_block_length };
            match run_parse(&input, output.as_deref(), sequence.as_deref(), &options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("error: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Validate {
            input,
            sequence,
            max_block_length,
        } => run_validate(&input, sequence.as_deref(), &GenerateOptions { max_block_length }),
    }
}

fn run_parse(
    input: &Path,
    output: Option<&Path>,
    sequence: Option<&Path>,
    options: &GenerateOptions,
) -> Result<(), String> {
    let sequence = repro_toolkit_core::generate_sequence_with_options(input, sequence, options)
        .map_err(|e| e.to_string())?;
    let json = repro_toolkit_core::to_json_string(&sequence).map_err(|e| e.to_string())?;

    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| e.to_string())?,
        None => println!("{json}"),
    }

    Ok(())
}

fn run_validate(input: &Path, sequence: Option<&Path>, options: &GenerateOptions) -> ExitCode {
    let sequence =
        match repro_toolkit_core::generate_sequence_with_options(input, sequence, options) {
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
