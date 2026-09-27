"""Thin ctypes wrapper over repro-toolkit-ffi's C ABI.

Usage:
    from repro_toolkit import ReproToolkit

    lib = ReproToolkit("../../target/release/librepro_toolkit_ffi.so")
    sequence = lib.parse_pdx("ecu.pdx")                     # dict, auto-generated
    sequence = lib.generate_sequence("ecu.pdx", "seq.json")  # dict, custom sequence
"""
import ctypes
import json
import platform
from pathlib import Path
from typing import Optional


def default_library_path(target_dir: Path) -> Path:
    """Guesses the built library's filename for the current OS."""
    system = platform.system()
    if system == "Windows":
        return target_dir / "repro_toolkit_ffi.dll"
    if system == "Darwin":
        return target_dir / "librepro_toolkit_ffi.dylib"
    return target_dir / "librepro_toolkit_ffi.so"


class ReproToolkitError(RuntimeError):
    """Raised when the library itself reports an error (`"ok": false`)."""


class ReproToolkit:
    def __init__(self, library_path: str):
        self._lib = ctypes.CDLL(library_path)

        self._lib.repro_toolkit_parse_pdx.argtypes = [ctypes.c_char_p]
        self._lib.repro_toolkit_parse_pdx.restype = ctypes.c_void_p

        self._lib.repro_toolkit_generate_sequence.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
        self._lib.repro_toolkit_generate_sequence.restype = ctypes.c_void_p

        self._lib.repro_toolkit_free_string.argtypes = [ctypes.c_void_p]
        self._lib.repro_toolkit_free_string.restype = None

    def parse_pdx(self, pdx_path: str) -> dict:
        """Auto-generates the repro sequence from a PDX file."""
        return self.generate_sequence(pdx_path, None)

    def generate_sequence(self, pdx_path: str, sequence_path: Optional[str]) -> dict:
        """Generates the repro sequence, using a custom sequence file when given."""
        pdx_bytes = pdx_path.encode("utf-8")
        seq_bytes = sequence_path.encode("utf-8") if sequence_path is not None else None

        ptr = self._lib.repro_toolkit_generate_sequence(pdx_bytes, seq_bytes)
        if not ptr:
            raise ReproToolkitError("pdx_path was empty")

        try:
            raw = ctypes.cast(ptr, ctypes.c_char_p).value
            envelope = json.loads(raw.decode("utf-8"))
        finally:
            self._lib.repro_toolkit_free_string(ptr)

        if not envelope.get("ok"):
            raise ReproToolkitError(envelope.get("error", "unknown error"))
        return envelope["sequence"]
