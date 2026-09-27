# Architecture

## Crates

| Crate | Kind | Role |
|-------|------|------|
| `crates/repro-toolkit-core` | Rust library | Does all the work: parsing, building the sequence, validation, JSON |
| `crates/repro-toolkit-ffi` | `cdylib` + `staticlib` | Thin C ABI over the core crate, for use from other languages |
| `crates/repro-toolkit-cli` | binary (`repro-toolkit`) | Thin command-line wrapper over the core crate |

The FFI and CLI crates contain no parsing logic of their own. Changes in behavior
belong in `repro-toolkit-core`.

## Data flow

```
 .pdx (ZIP)                      custom sequence .json (optional)
     │                                        │
     ▼                                        │
 pdx.rs            opens the ZIP and picks   │
                   the .odx-d/.odx-fd/.odx-f  │
     │                                        │
     ▼                                        │
 odx/parser.rs     XML → model (roxmltree)    │
     │                                        │
     ▼                                        │
 odx/model.rs      DiagLayer, DiagService,    │
                   Param, FlashDataBlock      │
     │                                        ▼
     ▼                                   custom.rs   loads and checks the file
 sequence.rs       classifies services         │     (steps are used as written)
                   into StepCategory, orders   │
                   them, one TransferData      │
                   step per flash block        │
     │                                        │
     └──────────────► ReproSequence ◄──────────┘
                          │
                          ├──► validate.rs   → Vec<ValidationIssue>
                          └──► serde_json    → JSON output
```

Entry points in `lib.rs`:
- `parse_pdx_file` / `parse_pdx_bytes`: build the default sequence.
- `generate_sequence(pdx, Option<custom>)`: use the custom file if one is given,
  otherwise build the default sequence. The PDX is still opened to fill in
  `ecu_variant` when the custom file leaves it out.
- `validate_sequence(&ReproSequence)`: run the linter.
- `to_json_string`: pretty-printed JSON.

`ReproSequence` can be both written and read as JSON: the output of `parse` is valid
input for `--sequence`.

## Errors

All fallible calls return `repro_toolkit_core::Result<T>`, which uses the `ReproError`
enum in `error.rs`. Every error message names the file or archive entry it relates to.
Validation findings are *not* errors: `validate_sequence` returns them as data, and
the caller decides what to do with them.

## FFI contract

Defined in `crates/repro-toolkit-ffi/src/lib.rs`:
- Every function takes NUL-terminated UTF-8 paths and returns a heap-allocated JSON
  string. It returns NULL only when the required `pdx_path` is NULL.
- The returned JSON is always an envelope: `{"ok": true, ...}` or
  `{"ok": false, "error": "..."}`. Errors are never reported through return codes or
  exceptions.
- **The caller must free every string it gets back with `repro_toolkit_free_string`.**
  Freeing it with `free`, `delete` or a .NET allocator is undefined behavior, because
  the memory belongs to Rust's allocator.

The per-language wrappers in `examples/` (the C++ RAII class, the Python `ctypes`
class, C# `ReproToolkit.Interop`) exist so callers don't have to handle ownership by
hand.

## Extending

- **New validation check**: add a function in `validate.rs`, call it from
  `validate_sequence`, and add a test in the same file. Document it in the
  Validation section of `docs/custom-sequence-guide.md`.
- **New step category**: add a variant to `StepCategory` in `sequence.rs`, then update
  `classify`, `category_rank` and, if ordering matters, `ORDERED_CHAIN` in
  `validate.rs`. The JSON name comes from `SCREAMING_SNAKE_CASE`.
- **More ODX coverage**: extend `odx/model.rs` and `odx/parser.rs`, and add the
  construct to the fixture in `lib.rs` tests. After that, regenerate
  `samples/sample.pdx` with `scripts/generate_sample_pdx.py`.
- **New FFI function**: follow the same null-check → UTF-8 → JSON-envelope pattern,
  add a test to the FFI crate, and add the declaration to `examples/c/repro_toolkit.h`.
