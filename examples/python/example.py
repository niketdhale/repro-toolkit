#!/usr/bin/env python3
"""Minimal Python example using repro_toolkit.py (ctypes wrapper).

Usage: python3 example.py <ecu.pdx> [custom-sequence.json] [path/to/librepro_toolkit_ffi.so]
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from repro_toolkit import ReproToolkit, default_library_path  # noqa: E402


def main() -> int:
    if len(sys.argv) < 2:
        print(f"usage: {sys.argv[0]} <ecu.pdx> [custom-sequence.json] [library_path]", file=sys.stderr)
        return 1

    pdx_path = sys.argv[1]
    sequence_path = sys.argv[2] if len(sys.argv) >= 3 else None
    target_dir = Path(__file__).resolve().parents[2] / "target" / "release"
    library_path = sys.argv[3] if len(sys.argv) >= 4 else str(default_library_path(target_dir))

    lib = ReproToolkit(library_path)

    default_sequence = lib.parse_pdx(pdx_path)
    print("=== default sequence ===")
    print(json.dumps(default_sequence, indent=2))

    if sequence_path is not None:
        custom_sequence = lib.generate_sequence(pdx_path, sequence_path)
        print("\n=== custom sequence ===")
        print(json.dumps(custom_sequence, indent=2))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
