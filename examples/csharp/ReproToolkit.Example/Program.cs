// Minimal C# example using the ReproToolkit.Interop project reference.
//
// Usage: dotnet run -- <ecu.pdx> [custom-sequence.json] [native-lib-dir]
//
// native-lib-dir defaults to ../../../target/release relative to this
// file, i.e. this repository's own Cargo build output, since this repo
// doesn't copy the native library into the C# build output automatically
// (see NativeLibraryLoader.cs for why/how that's resolved).
using ReproToolkit.Interop;

if (args.Length < 1)
{
    Console.Error.WriteLine("usage: dotnet run -- <ecu.pdx> [custom-sequence.json] [native-lib-dir]");
    return 1;
}

string pdxPath = args[0];
string? sequencePath = args.Length >= 2 ? args[1] : null;
string nativeLibDir = args.Length >= 3
    ? args[2]
    : Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "target", "release");

Environment.SetEnvironmentVariable("REPRO_TOOLKIT_NATIVE_LIB_DIR", Path.GetFullPath(nativeLibDir));

try
{
    string defaultSequence = ReproToolkitClient.ParsePdx(pdxPath);
    Console.WriteLine("=== default sequence ===");
    Console.WriteLine(defaultSequence);

    if (sequencePath is not null)
    {
        string customSequence = ReproToolkitClient.GenerateSequence(pdxPath, sequencePath);
        Console.WriteLine("\n=== custom sequence ===");
        Console.WriteLine(customSequence);
    }

    return 0;
}
catch (ReproToolkitException ex)
{
    Console.Error.WriteLine($"repro-toolkit error: {ex.Message}");
    return 1;
}
