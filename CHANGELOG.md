# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-09-27

### Added
- `repro-toolkit-core`: parses PDX archives (ISO 22901 ODX in a ZIP) and builds an
  ordered UDS (ISO 14229) repro sequence, serialized as JSON (#1).
- `repro-toolkit-ffi`: C ABI (`repro_toolkit_parse_pdx`, `repro_toolkit_free_string`)
  built as a `.dll`/`.so`/`.dylib` for C, C++, C#, Python and other languages (#1).
- `repro-toolkit-cli`: `repro-toolkit parse <pdx> [-o out.json]` (#1).
- Custom sequences: supply a hand-written JSON sequence with the PDX through
  `--sequence`, `generate_sequence()` or `repro_toolkit_generate_sequence`. Without one,
  the default PDX-derived sequence is used (#2).
- `docs/custom-sequence-guide.md` and `samples/custom-sequence.example.json` (#2).
- Runnable examples for C, C++, Python, C# and Rust under `examples/`, plus
  `samples/sample.pdx` (#3, #4).
- Sequence validator: `repro-toolkit validate`, `validate_sequence()` and
  `repro_toolkit_validate_sequence`. It reports invalid hex, overlapping byte ranges,
  known steps with no request, and out-of-order UDS steps (#5).
- CI: build, test, clippy, the example programs and a CLI validate smoke test.

[Unreleased]: https://github.com/niketdhale/repro-toolkit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/niketdhale/repro-toolkit/releases/tag/v0.1.0
