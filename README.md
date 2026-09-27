# repro-toolkit

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
```

## CLI usage

```
cargo run -p repro-toolkit-cli -- parse path/to/ecu.pdx
cargo run -p repro-toolkit-cli -- parse path/to/ecu.pdx -o sequence.json
```

## Library usage (Rust)

```rust
let sequence = repro_toolkit_core::parse_pdx_file("ecu.pdx")?;
let json = repro_toolkit_core::to_json_string(&sequence)?;
```

## FFI usage (any language)

Build the shared library:

```
cargo build --release -p repro-toolkit-ffi
# -> target/release/librepro_toolkit_ffi.so   (Linux)
# -> target/release/repro_toolkit_ffi.dll     (Windows)
```

Contract: two exported functions.

```c
char* repro_toolkit_parse_pdx(const char* path); // NUL-terminated UTF-8 JSON, or NULL if path is NULL
void  repro_toolkit_free_string(char* ptr);      // must be called on every non-null string returned above
```

The returned JSON is always one of:
```json
{"ok": true, "sequence": { ... }}
{"ok": false, "error": "..."}
```

Example from C#:

```csharp
[DllImport("repro_toolkit_ffi")]
static extern IntPtr repro_toolkit_parse_pdx(string path);

[DllImport("repro_toolkit_ffi")]
static extern void repro_toolkit_free_string(IntPtr ptr);

IntPtr ptr = repro_toolkit_parse_pdx("ecu.pdx");
string json = Marshal.PtrToStringUTF8(ptr);
repro_toolkit_free_string(ptr);
```

Example from Python:

```python
import ctypes
lib = ctypes.CDLL("./librepro_toolkit_ffi.so")
lib.repro_toolkit_parse_pdx.restype = ctypes.c_void_p
ptr = lib.repro_toolkit_parse_pdx(b"ecu.pdx")
json_str = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
lib.repro_toolkit_free_string(ptr)
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
