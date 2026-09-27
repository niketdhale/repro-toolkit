using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace ReproToolkit.Interop;

/// Registers a custom DllImport resolver so this assembly can find the
/// native `repro_toolkit_ffi` library even when it isn't copied next to
/// the managed assembly (the normal .NET convention). This matters for
/// this repository's example specifically, where the native library is
/// built under `target/release/` by Cargo rather than by MSBuild.
///
/// In your own project, the simplest approach is usually to just copy the
/// built native library (repro_toolkit_ffi.dll / librepro_toolkit_ffi.so /
/// librepro_toolkit_ffi.dylib) into your output directory, e.g. via a
/// `<None Include="..." CopyToOutputDirectory="PreserveNewest" />` item in
/// your .csproj, or by shipping it under `runtimes/<rid>/native/` if you
/// package this wrapper as a NuGet package. This resolver is here so the
/// example works without requiring that extra build step, and as a
/// reference for anyone who does want to search a custom directory.
internal static class NativeLibraryLoader
{
    private const string LibraryName = "repro_toolkit_ffi";
    private static bool _registered;
    private static readonly object RegisterLock = new();

    public static void EnsureResolverRegistered()
    {
        lock (RegisterLock)
        {
            if (_registered)
            {
                return;
            }

            NativeLibrary.SetDllImportResolver(Assembly.GetExecutingAssembly(), Resolve);
            _registered = true;
        }
    }

    private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != LibraryName)
        {
            return IntPtr.Zero;
        }

        foreach (var candidate in CandidatePaths())
        {
            if (File.Exists(candidate) && NativeLibrary.TryLoad(candidate, out var handle))
            {
                return handle;
            }
        }

        // Fall back to the default probing behavior (next to the
        // assembly, PATH, LD_LIBRARY_PATH, DYLD_LIBRARY_PATH, etc.).
        return NativeLibrary.TryLoad(LibraryName, assembly, searchPath, out var fallbackHandle)
            ? fallbackHandle
            : IntPtr.Zero;
    }

    private static IEnumerable<string> CandidatePaths()
    {
        var fileName = PlatformFileName();

        var overrideDir = Environment.GetEnvironmentVariable("REPRO_TOOLKIT_NATIVE_LIB_DIR");
        if (!string.IsNullOrEmpty(overrideDir))
        {
            yield return Path.Combine(overrideDir, fileName);
        }

        yield return Path.Combine(AppContext.BaseDirectory, fileName);
    }

    private static string PlatformFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "repro_toolkit_ffi.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "librepro_toolkit_ffi.dylib";
        }

        return "librepro_toolkit_ffi.so";
    }
}
