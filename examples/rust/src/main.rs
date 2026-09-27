//! Minimal Rust example: calling `repro-toolkit-core` directly, no FFI
//! needed since this is Rust talking to Rust.
//!
//! Usage: `cargo run -- <ecu.pdx> [custom-sequence.json]`

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <ecu.pdx> [custom-sequence.json]", args[0]);
        return ExitCode::FAILURE;
    }

    let pdx_path = &args[1];
    let sequence_path = args.get(2);

    match run(pdx_path, sequence_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(pdx_path: &str, sequence_path: Option<&String>) -> Result<(), String> {
    let default_sequence =
        repro_toolkit_core::parse_pdx_file(pdx_path).map_err(|e| e.to_string())?;
    let json = repro_toolkit_core::to_json_string(&default_sequence).map_err(|e| e.to_string())?;
    println!("=== default sequence ===\n{json}\n");

    if let Some(sequence_path) = sequence_path {
        let custom_sequence = repro_toolkit_core::generate_sequence(pdx_path, Some(sequence_path))
            .map_err(|e| e.to_string())?;
        let json = repro_toolkit_core::to_json_string(&custom_sequence).map_err(|e| e.to_string())?;
        println!("=== custom sequence ===\n{json}");
    }

    Ok(())
}
