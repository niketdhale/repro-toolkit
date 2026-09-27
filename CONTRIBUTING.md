# Contributing

Contributions are welcome: bug reports, sample PDX edge cases, and pull requests.

## Getting started

You need a stable Rust toolchain. The C/C++ examples also need `gcc`/`clang` and `make`,
and the Python example needs `python3`.

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs these three commands. It also runs every example in `examples/` (except C#)
and a `repro-toolkit validate` smoke test. See `.github/workflows/ci.yml`.

To run the examples, see [`examples/README.md`](examples/README.md).

## Sample data

`samples/sample.pdx` is generated from the ODX fixture in
`crates/repro-toolkit-core/src/lib.rs`. If you change that fixture, regenerate the file:

```
python3 scripts/generate_sample_pdx.py
```

Never commit a real vendor PDX. OEM diagnostic data is usually confidential.

## Pull requests

- Keep each PR focused on one change.
- Add or update tests. Parser and validator changes need a unit test.
- `cargo test` and `cargo clippy -- -D warnings` must pass.
- If you change the JSON format, the FFI functions or the CLI, update `README.md`,
  `docs/`, and the `[Unreleased]` section of `CHANGELOG.md`.
- Start by reading [`docs/architecture.md`](docs/architecture.md). It explains where
  things live and how to extend them.

## Reporting bugs

Open an issue using the bug report template. If the issue is caused by a particular PDX,
please shrink it down to a small ODX snippet that shows the problem instead of attaching
the whole file.

For security issues, follow [`SECURITY.md`](SECURITY.md) instead.
