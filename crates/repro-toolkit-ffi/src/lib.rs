//! C ABI over `repro-toolkit-core`, so the parser can be called from any
//! language with FFI support (C#'s P/Invoke, Python's ctypes/cffi, C/C++
//! directly) without needing to embed a managed runtime.
//!
//! Contract: every call returns a heap-allocated, NUL-terminated UTF-8
//! JSON string of the shape `{"ok": true, "sequence": {...}}` or
//! `{"ok": false, "error": "..."}`. The caller must free every non-null
//! string returned from this library with [`repro_toolkit_free_string`],
//! and must never free it with anything else (the allocator on each side
//! of an FFI boundary is not guaranteed to match).

use std::ffi::{c_char, CStr, CString};

/// Parse a PDX file at `path` (a NUL-terminated UTF-8 path string) and
/// return its UDS repro sequence as a JSON string. Returns NULL only if
/// `path` itself is NULL.
///
/// # Safety
/// `path` must be either NULL or a valid pointer to a NUL-terminated
/// UTF-8 C string that remains valid for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_parse_pdx(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return to_c_string(&error_json("path is not valid UTF-8")),
    };

    let result_json = match repro_toolkit_core::parse_pdx_file(path_str) {
        Ok(sequence) => match repro_toolkit_core::to_json_string(&sequence) {
            Ok(json) => format!("{{\"ok\":true,\"sequence\":{json}}}"),
            Err(e) => error_json(&e.to_string()),
        },
        Err(e) => error_json(&e.to_string()),
    };

    to_c_string(&result_json)
}

/// Frees a string previously returned by any `repro_toolkit_*` function.
/// Passing NULL is a no-op. Never call this on a pointer not returned by
/// this library.
///
/// # Safety
/// `ptr` must be either NULL or a pointer previously returned by a
/// `repro_toolkit_*` function in this library, not yet freed.
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

fn error_json(message: &str) -> String {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    format!("{{\"ok\":false,\"error\":\"{escaped}\"}}")
}

fn to_c_string(s: &str) -> *mut c_char {
    // A NUL byte can't appear in our own generated JSON, so this is safe
    // to unwrap.
    CString::new(s).expect("generated JSON must not contain NUL bytes").into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_path_returns_null() {
        unsafe {
            assert!(repro_toolkit_parse_pdx(std::ptr::null()).is_null());
        }
    }

    #[test]
    fn missing_file_returns_error_json() {
        let path = CString::new("/nonexistent/path.pdx").unwrap();
        unsafe {
            let result = repro_toolkit_parse_pdx(path.as_ptr());
            assert!(!result.is_null());
            let json = CStr::from_ptr(result).to_str().unwrap();
            assert!(json.contains("\"ok\":false"));
            repro_toolkit_free_string(result);
        }
    }
}
