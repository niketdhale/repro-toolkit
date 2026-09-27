# Examples

One example per language, all calling the same underlying library
against the same sample PDX (`samples/sample.pdx`) and, optionally, the
same custom sequence file (`samples/custom-sequence.example.json`).

| Language | Directory | Calls |
|----------|-----------|-------|
| C | [`c/`](c) | `repro-toolkit-ffi` (C ABI) directly |
| C++ | [`cpp/`](cpp) | `repro-toolkit-ffi` (C ABI), via a small RAII wrapper |
| Python | [`python/`](python) | `repro-toolkit-ffi` (C ABI), via `ctypes` |
| C# | [`csharp/`](csharp) | `repro-toolkit-ffi` (C ABI), via P/Invoke, wrapped in a managed `ReproToolkit.Interop` project |
| Rust | [`rust/`](rust) | `repro-toolkit-core` directly — no FFI, since it's Rust calling Rust |

The C example additionally calls `repro_toolkit_validate_sequence` (see
[Validation](../docs/custom-sequence-guide.md#validation)) alongside
`repro_toolkit_generate_sequence`, printing the linter's output for both
the default and custom sequence.

## Prerequisites

All examples except Rust load the native library built from
`repro-toolkit-ffi`. Build it once from the repository root:

```
cargo build --release -p repro-toolkit-ffi
```

This produces `target/release/librepro_toolkit_ffi.so` (Linux),
`target/release/repro_toolkit_ffi.dll` (Windows), or
`target/release/librepro_toolkit_ffi.dylib` (macOS).

The sample PDX at `samples/sample.pdx` is checked into the repository and
regenerated (if you ever need to) with:

```
python3 scripts/generate_sample_pdx.py
```

## Running each example

```
# C
cd examples/c && make run

# C++
cd examples/cpp && make run

# Python
cd examples/python && python3 example.py ../../samples/sample.pdx ../../samples/custom-sequence.example.json

# Rust (builds repro-toolkit-core directly, no separate FFI build needed)
cd examples/rust && cargo run --release -- ../../samples/sample.pdx ../../samples/custom-sequence.example.json

# C# (see examples/csharp/README.md for the native-library-loading details)
cd examples/csharp/ReproToolkit.Example && dotnet run -- ../../../samples/sample.pdx ../../../samples/custom-sequence.example.json
```

Every example prints the default, auto-generated sequence, then — when a
custom sequence file is passed — the custom one, so you can compare both
outputs side by side. See
[`docs/custom-sequence-guide.md`](../docs/custom-sequence-guide.md) for
how to write your own custom sequence file.

## Verified in this repository's environment

The C, C++, Python, and Rust examples were built and actually run against
the real native library in this repository's own dev environment as part
of writing them. The C# example was written to the same C ABI contract
(and its P/Invoke signatures were checked carefully against it) but could
not be built/run here, since this environment's network policy blocks
installing the .NET SDK — see
[`examples/csharp/README.md`](csharp/README.md) for details.
