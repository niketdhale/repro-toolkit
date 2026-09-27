# C# example

Two projects:

- **`ReproToolkit.Interop`** — a managed class library you can reference
  like any normal C# project (or package as a NuGet package). It does the
  P/Invoke internally; the only public type is `ReproToolkitClient`.
- **`ReproToolkit.Example`** — a console app that references it and prints
  a repro sequence.

## About the native DLL/SO

`repro-toolkit-ffi` compiles to a **native** shared library
(`repro_toolkit_ffi.dll` on Windows, `librepro_toolkit_ffi.so` on Linux,
`librepro_toolkit_ffi.dylib` on macOS) — not a .NET assembly. That means:

- You can't "Add Reference" to it directly in Visual Studio's reference
  picker (it carries no .NET metadata to browse). What you reference
  instead is `ReproToolkit.Interop` — a normal managed project/NuGet
  package — which calls the native library via `[DllImport]` internally.
- The native library still has to be *findable at runtime*. The usual
  approaches, in order of how most real projects do it:
  1. Copy it into your app's output directory (next to the `.exe`), e.g.
     via a `<None Include="..." CopyToOutputDirectory="PreserveNewest" />`
     item in your `.csproj`.
  2. Package it inside your NuGet package under `runtimes/<rid>/native/`
     (e.g. `runtimes/win-x64/native/repro_toolkit_ffi.dll`), and the .NET
     runtime resolves it automatically for the running platform.
  3. Put it on `PATH` (Windows) or `LD_LIBRARY_PATH`/`DYLD_LIBRARY_PATH`
     (Linux/macOS).

  This example uses a fourth option instead, since this repo's native
  library is built by Cargo under `target/release/`, not copied anywhere
  by MSBuild: `NativeLibraryLoader.cs` registers a custom
  `NativeLibrary.SetDllImportResolver` that checks the
  `REPRO_TOOLKIT_NATIVE_LIB_DIR` environment variable (which
  `Program.cs` sets to `../../target/release` by default) before falling
  back to normal resolution. For your own project, prefer option 1 or 2
  instead — the custom resolver here exists specifically to make this
  example runnable without an extra manual copy step.
- Either way, C# never gets to call a managed method with IntelliSense
  pulled from the DLL's own metadata (there is none) — the "contract" is
  the hand-written `[DllImport]` signatures in `NativeMethods.cs`, which
  must match the C ABI exactly. That's the whole reason
  `ReproToolkit.Interop` exists: write that contract once, correctly, and
  give everyone else a normal C# API (`ReproToolkitClient.ParsePdx(...)`)
  instead of hand-writing `DllImport` themselves.

## Running

```
# from the repository root
cargo build --release -p repro-toolkit-ffi
cd examples/csharp/ReproToolkit.Example
dotnet run -- ../../../samples/sample.pdx ../../../samples/custom-sequence.example.json
```

> **Note:** this example was written carefully against the .NET 8 SDK and
> the P/Invoke conventions documented above, but — unlike the C, C++,
> Python, and Rust examples in this repository, which were all actually
> compiled and run — it has **not** been built/run in this repository's
> own CI/dev environment, because that environment's network policy
> blocks installing the .NET SDK. If you hit an issue running it, please
> open one; the C ABI it wraps (`repro_toolkit_generate_sequence` /
> `repro_toolkit_free_string`) is exercised directly and verified by the
> C and C++ examples in this same `examples/` directory.
