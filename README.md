# repro-toolkit

[![CI](https://github.com/niketdhale/repro-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/niketdhale/repro-toolkit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](CHANGELOG.md)

A cross-platform, multi-language library and CLI for automotive ECU
reprogramming. It parses a **PDX** archive (the ZIP-packaged ODX
diagnostic data format defined by ISO 22901) and generates the ordered
**UDS (ISO 14229) repro sequence** — the requests and expected
positive/negative responses needed to flash the ECU — as a JSON document.

Written in Rust so the same implementation can be:
- used directly as a Rust library,
- called from **any language with FFI** (C#, Python, C/C++, ...) via a
  small C ABI, with no managed runtime to embed, or
- run as a **standalone CLI** that reads a `.pdx` and writes JSON, for
  tools that would rather spawn a process than link against a library.

It builds and runs the same way on Linux and Windows.

## Project layout

```
repro-toolkit/
  crates/
    repro-toolkit-core/   # library: PDX/ODX parsing, sequence builder, JSON output
    repro-toolkit-ffi/    # cdylib/staticlib: C ABI wrapper over core
    repro-toolkit-cli/    # binary: `repro-toolkit parse <file.pdx>`
  examples/               # one example per language: C, C++, Python, C#, Rust
  samples/                # sample.pdx and a worked custom-sequence.example.json
  docs/                   # architecture.md, custom-sequence-guide.md
```

For an overview of how the code fits together, see
[docs/architecture.md](docs/architecture.md).

## CLI usage

```
cargo run -p repro-toolkit-cli -- parse path/to/ecu.pdx
cargo run -p repro-toolkit-cli -- parse path/to/ecu.pdx -o sequence.json
cargo run -p repro-toolkit-cli -- parse path/to/ecu.pdx --sequence my-custom-sequence.json
cargo run -p repro-toolkit-cli -- validate path/to/ecu.pdx --sequence my-custom-sequence.json
```

## Library usage (Rust)

```rust
// Auto-generate the sequence from the PDX:
let sequence = repro_toolkit_core::parse_pdx_file("ecu.pdx")?;

// Or use a custom sequence file, falling back to the PDX-derived one when
// `custom_sequence_path` is `None`:
let sequence = repro_toolkit_core::generate_sequence("ecu.pdx", Some("my-custom-sequence.json"))?;

let json = repro_toolkit_core::to_json_string(&sequence)?;

// Check a sequence for problems before using it (see "Validation" below):
let issues = repro_toolkit_core::validate_sequence(&sequence);
```

## FFI usage (any language)

Build the shared library:

```
cargo build --release -p repro-toolkit-ffi
# -> target/release/librepro_toolkit_ffi.so   (Linux)
# -> target/release/repro_toolkit_ffi.dll     (Windows)
```

Contract: four exported functions.

```c
char* repro_toolkit_parse_pdx(const char* path); // NUL-terminated UTF-8 JSON, or NULL if path is NULL
char* repro_toolkit_generate_sequence(const char* pdx_path, const char* sequence_path); // sequence_path may be NULL
char* repro_toolkit_validate_sequence(const char* pdx_path, const char* sequence_path); // sequence_path may be NULL
void  repro_toolkit_free_string(char* ptr);      // must be called on every non-null string returned above
```

The returned JSON is one of:
```json
{"ok": true, "sequence": { ... }}
{"ok": true, "valid": true, "issues": []}
{"ok": true, "valid": false, "issues": [{"severity": "error", "step_index": 2, "message": "..."}]}
{"ok": false, "error": "..."}
```
(`repro_toolkit_validate_sequence` returns the `"valid"`/`"issues"` shape; the other two functions return `"sequence"`.)

Example from C#:

```csharp
[DllImport("repro_toolkit_ffi")]
static extern IntPtr repro_toolkit_parse_pdx(string path);

// sequencePath may be null to fall back to the default, PDX-derived sequence.
[DllImport("repro_toolkit_ffi")]
static extern IntPtr repro_toolkit_generate_sequence(string pdxPath, string? sequencePath);

[DllImport("repro_toolkit_ffi")]
static extern void repro_toolkit_free_string(IntPtr ptr);

// Default sequence:
IntPtr ptr = repro_toolkit_parse_pdx("ecu.pdx");
string json = Marshal.PtrToStringUTF8(ptr);
repro_toolkit_free_string(ptr);

// Custom sequence (see docs/custom-sequence-guide.md):
IntPtr customPtr = repro_toolkit_generate_sequence("ecu.pdx", "my-custom-sequence.json");
string customJson = Marshal.PtrToStringUTF8(customPtr);
repro_toolkit_free_string(customPtr);
```

Example from Python:

```python
import ctypes
lib = ctypes.CDLL("./librepro_toolkit_ffi.so")
lib.repro_toolkit_parse_pdx.restype = ctypes.c_void_p
lib.repro_toolkit_generate_sequence.restype = ctypes.c_void_p

# Default sequence:
ptr = lib.repro_toolkit_parse_pdx(b"ecu.pdx")
json_str = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
lib.repro_toolkit_free_string(ptr)

# Custom sequence (pass None/nullptr for sequence_path to get default behavior):
custom_ptr = lib.repro_toolkit_generate_sequence(b"ecu.pdx", b"my-custom-sequence.json")
custom_json = ctypes.cast(custom_ptr, ctypes.c_char_p).value.decode("utf-8")
lib.repro_toolkit_free_string(custom_ptr)
```

## Output shape

```json
{
  "ecu_variant": "Sample_ECU",
  "step_count": 8,
  "steps": [
    {
      "index": 0,
      "name": "DiagnosticSessionControl_Programming",
      "category": "SESSION_CONTROL",
      "semantic": "SESSION",
      "request": { "name": "...", "fields": [ { "name": "SID", "byte_position": 0, "bit_length": 8, "kind": "fixed", "value_hex": "10" }, ... ] },
      "expected_positive_responses": [ ... ],
      "expected_negative_responses": [ ... ],
      "notes": null
    }
  ]
}
```

Steps are ordered into the canonical UDS flash sequence:
`SESSION_CONTROL` → `SECURITY_SEED_REQUEST` → `SECURITY_KEY_SEND` →
`ERASE_MEMORY` → `REQUEST_DOWNLOAD` → `TRANSFER_DATA` (repeated once per
flash data block found in the PDX's `odx-f` data) →
`REQUEST_TRANSFER_EXIT` → `CHECK_MEMORY` → `ECU_RESET`. Any diagnostic
service that doesn't match one of these categories is still included,
tagged `OTHER`, so nothing from the source ODX is silently dropped.

Each request/response field is either:
- `"kind": "fixed"` — a byte value already known from the ODX
  `CODED-CONST` (e.g. the service ID/sub-function), given as hex, or
- `"kind": "variable"` — a value only known at runtime (a seed, an
  address, a data payload, ...), with its ODX base data type when known.

## Examples

Runnable examples for C, C++, Python, C#, and Rust live in
[`examples/`](examples), each calling this library against the same
sample PDX. See [`examples/README.md`](examples/README.md) to get
started.

## Custom sequences

If the default, auto-generated sequence isn't what you need — you want
to insert a vendor-specific step, add a delay, reorder steps, or replace
the whole thing — supply your own sequence as a JSON file alongside the
PDX (`--sequence` on the CLI, the second argument to `generate_sequence`/
`repro_toolkit_generate_sequence`). If you don't supply one, the library
falls back to its default PDX-derived sequence, unchanged.

See **[docs/custom-sequence-guide.md](docs/custom-sequence-guide.md)**
for the full schema and recipes, and
**[samples/custom-sequence.example.json](samples/custom-sequence.example.json)**
for a complete worked example (the standard flash sequence with an extra
vendor self-test step spliced in).

## Validation

`repro-toolkit validate <pdx> [--sequence <file>]` (also
`validate_sequence()` in the library, and `repro_toolkit_validate_sequence`
over FFI) lints a sequence — default or custom — for problems that are
valid JSON but still wrong: overlapping byte positions within one
message, invalid hex on a fixed field, a known UDS step with no request
at all, or known steps in the wrong order. Errors mean the sequence
shouldn't be used as-is; warnings are a heads-up, since a custom
sequence's order and content are fully under your control. See
[Validation in the custom sequence guide](docs/custom-sequence-guide.md#validation)
for the full list of checks.

## Scope and limitations

This targets the structures that matter for building a repro sequence,
not the entire ODX/ISO 22901 schema:

- Parses `DIAG-LAYER`/`ECU-VARIANT`/`BASE-VARIANT`/`PROTOCOL` diagnostic
  layers, their `DIAG-SERVICE`s, and `REQUEST`/`POS-RESPONSE`/
  `NEG-RESPONSE` parameter lists (`CODED-CONST` fixed bytes and variable
  params) from `.odx-d`/`.odx-fd` entries.
- Parses flash memory segments (address + size) from `.odx-f` entries.
- Does **not** resolve `DOP` (data object property) physical-value
  conversions, compression/encryption metadata on flash blocks, or
  OEM-specific security-access seed/key algorithms — those are
  OEM-specific/secret by nature and are surfaced as data (algorithm
  presence, variable field names/types) rather than executed.
- Service classification into the canonical sequence is keyword-based
  (ODX `SEMANTIC` attribute + short name), which covers standard UDS
  naming conventions but may need tuning per OEM's own ODX authoring
  style.

## Development

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

All three commands are known to run clean in this repository's CI/dev
environment (Rust toolchain via `cargo`/`rustc`, no extra setup needed).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), the [Code of Conduct](CODE_OF_CONDUCT.md),
and [SECURITY.md](SECURITY.md) for how to report vulnerabilities. Release notes are in
[CHANGELOG.md](CHANGELOG.md).

## License

MIT. See [LICENSE](LICENSE).
