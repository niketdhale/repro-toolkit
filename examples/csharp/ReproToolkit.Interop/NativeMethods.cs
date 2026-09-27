using System;
using System.Runtime.InteropServices;

namespace ReproToolkit.Interop;

/// Raw P/Invoke declarations for repro-toolkit-ffi's C ABI. Not intended
/// for direct use outside this project — use <see cref="ReproToolkitClient"/>.
internal static class NativeMethods
{
    private const string LibraryName = "repro_toolkit_ffi";

    static NativeMethods()
    {
        NativeLibraryLoader.EnsureResolverRegistered();
    }

    // Strings are marshaled explicitly as UTF-8 (LPUTF8Str) rather than
    // relying on CharSet.Ansi, which maps to the OS's default code page on
    // Windows (not UTF-8) and would mangle any non-ASCII path.
    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr repro_toolkit_parse_pdx(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr repro_toolkit_generate_sequence(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string pdxPath,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? sequencePath);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void repro_toolkit_free_string(IntPtr ptr);
}
