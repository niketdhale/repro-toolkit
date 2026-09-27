using System;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace ReproToolkit.Interop;

/// Thrown when the native library itself reports an error
/// (the `{"ok": false, "error": "..."}` envelope), as opposed to a .NET
/// exception from marshaling or argument validation.
public sealed class ReproToolkitException : Exception
{
    public ReproToolkitException(string message) : base(message)
    {
    }
}

/// Managed entry point for repro-toolkit-ffi. This is the only public type
/// consumers of this project need — <see cref="NativeMethods"/> and
/// <see cref="NativeLibraryLoader"/> are internal implementation details.
public static class ReproToolkitClient
{
    /// Auto-generates the repro sequence from a PDX file and returns it as
    /// a JSON string (just the sequence object, with the `{"ok": ...}`
    /// envelope already unwrapped).
    public static string ParsePdx(string pdxPath) => GenerateSequence(pdxPath, sequencePath: null);

    /// Generates the repro sequence for `pdxPath`. If `sequencePath` is
    /// non-null, it's used as a custom sequence JSON file instead of
    /// auto-generating the sequence (see
    /// ../../docs/custom-sequence-guide.md); pass null for the default
    /// behavior.
    public static string GenerateSequence(string pdxPath, string? sequencePath)
    {
        if (pdxPath is null)
        {
            throw new ArgumentNullException(nameof(pdxPath));
        }

        IntPtr ptr = NativeMethods.repro_toolkit_generate_sequence(pdxPath, sequencePath);
        if (ptr == IntPtr.Zero)
        {
            // The native side only returns NULL when pdx_path itself was
            // NULL, which the null-check above already rules out — but
            // guard anyway rather than trust that invariant blindly.
            throw new ReproToolkitException("repro_toolkit_generate_sequence unexpectedly returned NULL");
        }

        try
        {
            string json = Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
            using JsonDocument document = JsonDocument.Parse(json);
            JsonElement root = document.RootElement;

            bool ok = root.TryGetProperty("ok", out JsonElement okElement) && okElement.GetBoolean();
            if (!ok)
            {
                string error = root.TryGetProperty("error", out JsonElement errorElement)
                    ? errorElement.GetString() ?? "unknown error"
                    : "unknown error";
                throw new ReproToolkitException(error);
            }

            return root.GetProperty("sequence").GetRawText();
        }
        finally
        {
            // Must be freed with repro_toolkit_free_string, never with a
            // .NET allocator — the string was allocated by Rust's.
            NativeMethods.repro_toolkit_free_string(ptr);
        }
    }
}
