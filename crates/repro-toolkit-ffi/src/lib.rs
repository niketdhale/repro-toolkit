//! C ABI over `repro-toolkit-core`, so the parser can be called from any
//! language with FFI support (C#'s P/Invoke, Python's ctypes/cffi, C/C++
//! directly) without needing to embed a managed runtime.
//!
//! Contract: every call returns a heap-allocated, NUL-terminated UTF-8
//! JSON string. `repro_toolkit_parse_pdx`/`repro_toolkit_generate_sequence`
//! return `{"ok": true, "sequence": {...}}` on success;
//! `repro_toolkit_validate_sequence` returns
//! `{"ok": true, "valid": bool, "issues": [...]}`; any of them may instead
//! return `{"ok": false, "error": "..."}`. The caller must free every non-null
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
    repro_toolkit_generate_sequence(path, std::ptr::null())
}

/// Like [`repro_toolkit_parse_pdx`], but if `sequence_path` is non-NULL it
/// is used as a custom sequence JSON file instead of auto-generating the
/// sequence from the PDX (see `docs/custom-sequence-guide.md`). Pass NULL
/// for `sequence_path` to get the same behavior as
/// [`repro_toolkit_parse_pdx`]. Returns NULL only if `pdx_path` itself is
/// NULL.
///
/// # Safety
/// `pdx_path` must be either NULL or a valid pointer to a NUL-terminated
/// UTF-8 C string that remains valid for the duration of this call.
/// `sequence_path` must be either NULL or likewise a valid, live,
/// NUL-terminated UTF-8 C string pointer.
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_generate_sequence(
    pdx_path: *const c_char,
    sequence_path: *const c_char,
) -> *mut c_char {
    repro_toolkit_generate_sequence_ex(pdx_path, sequence_path, 0)
}

/// Like [`repro_toolkit_generate_sequence`], with an explicit TransferData
/// `max_block_length` (UDS maxNumberOfBlockLength, including the SID and
/// block sequence counter bytes). Pass 0 for the library default (0x0FFF).
/// Ignored when `sequence_path` is non-NULL.
///
/// # Safety
/// Same requirements as [`repro_toolkit_generate_sequence`].
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_generate_sequence_ex(
    pdx_path: *const c_char,
    sequence_path: *const c_char,
    max_block_length: u32,
) -> *mut c_char {
    if pdx_path.is_null() {
        return std::ptr::null_mut();
    }
    let (pdx, seq) = match read_paths(pdx_path, sequence_path) {
        Ok(paths) => paths,
        Err(json) => return to_c_string(&json),
    };

    let options = options_from(max_block_length);
    let result_json =
        match repro_toolkit_core::generate_sequence_with_options(pdx, seq, &options) {
            Ok(sequence) => match repro_toolkit_core::to_json_string(&sequence) {
                Ok(json) => format!("{{\"ok\":true,\"sequence\":{json}}}"),
                Err(e) => error_json(&e.to_string()),
            },
            Err(e) => error_json(&e.to_string()),
        };

    to_c_string(&result_json)
}

/// Validates the repro sequence for `pdx_path` (optionally overridden by
/// `sequence_path`, exactly as in [`repro_toolkit_generate_sequence`]) and
/// returns the result as JSON: `{"ok": true, "valid": bool, "issues": [...]}`,
/// where each issue is `{"severity": "error"|"warning", "step_index": int|null, "message": "..."}`.
/// `valid` is `false` if and only if at least one issue has `"severity": "error"`.
/// Returns NULL only if `pdx_path` itself is NULL.
///
/// # Safety
/// Same requirements as [`repro_toolkit_generate_sequence`]: `pdx_path`
/// must be either NULL or a valid, live, NUL-terminated UTF-8 C string
/// pointer, and likewise for `sequence_path`.
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_validate_sequence(
    pdx_path: *const c_char,
    sequence_path: *const c_char,
) -> *mut c_char {
    repro_toolkit_validate_sequence_ex(pdx_path, sequence_path, 0)
}

/// Like [`repro_toolkit_validate_sequence`], with an explicit
/// `max_block_length` as in [`repro_toolkit_generate_sequence_ex`] (0 for
/// the default).
///
/// # Safety
/// Same requirements as [`repro_toolkit_generate_sequence`].
#[no_mangle]
pub unsafe extern "C" fn repro_toolkit_validate_sequence_ex(
    pdx_path: *const c_char,
    sequence_path: *const c_char,
    max_block_length: u32,
) -> *mut c_char {
    if pdx_path.is_null() {
        return std::ptr::null_mut();
    }
    let (pdx, seq) = match read_paths(pdx_path, sequence_path) {
        Ok(paths) => paths,
        Err(json) => return to_c_string(&json),
    };

    let options = options_from(max_block_length);
    let result_json = match repro_toolkit_core::generate_sequence_with_options(pdx, seq, &options)
    {
        Ok(sequence) => {
            let issues = repro_toolkit_core::validate_sequence(&sequence);
            let valid = !issues
                .iter()
                .any(|issue| issue.severity == repro_toolkit_core::Severity::Error);
            let issues_json = issues
                .iter()
                .map(issue_to_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"ok\":true,\"valid\":{valid},\"issues\":[{issues_json}]}}")
        }
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

/// Reads the (non-NULL) PDX path and optional sequence path, or returns
/// the error JSON to hand back to the caller.
///
/// # Safety
/// `pdx_path` must be non-NULL; both must be valid NUL-terminated strings
/// when non-NULL.
unsafe fn read_paths<'a>(
    pdx_path: *const c_char,
    sequence_path: *const c_char,
) -> Result<(&'a str, Option<&'a str>), String> {
    let pdx = CStr::from_ptr(pdx_path)
        .to_str()
        .map_err(|_| error_json("pdx_path is not valid UTF-8"))?;
    let seq = if sequence_path.is_null() {
        None
    } else {
        Some(
            CStr::from_ptr(sequence_path)
                .to_str()
                .map_err(|_| error_json("sequence_path is not valid UTF-8"))?,
        )
    };
    Ok((pdx, seq))
}

fn options_from(max_block_length: u32) -> repro_toolkit_core::GenerateOptions {
    if max_block_length == 0 {
        repro_toolkit_core::GenerateOptions::default()
    } else {
        repro_toolkit_core::GenerateOptions { max_block_length }
    }
}

fn error_json(message: &str) -> String {
    format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json_string(message))
}

fn issue_to_json(issue: &repro_toolkit_core::ValidationIssue) -> String {
    let severity = match issue.severity {
        repro_toolkit_core::Severity::Error => "error",
        repro_toolkit_core::Severity::Warning => "warning",
    };
    let step_index = match issue.step_index {
        Some(index) => index.to_string(),
        None => "null".to_string(),
    };
    format!(
        "{{\"severity\":\"{severity}\",\"step_index\":{step_index},\"message\":\"{}\"}}",
        escape_json_string(&issue.message)
    )
}

fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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

    #[test]
    fn validate_missing_file_returns_error_json() {
        let path = CString::new("/nonexistent/path.pdx").unwrap();
        unsafe {
            let result = repro_toolkit_validate_sequence(path.as_ptr(), std::ptr::null());
            assert!(!result.is_null());
            let json = CStr::from_ptr(result).to_str().unwrap();
            assert!(json.contains("\"ok\":false"));
            repro_toolkit_free_string(result);
        }
    }

    #[test]
    fn generate_ex_reports_too_small_block_length() {
        let pdx = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/sample.pdx");
        let pdx = CString::new(pdx.to_str().unwrap()).unwrap();
        unsafe {
            let result = repro_toolkit_generate_sequence_ex(pdx.as_ptr(), std::ptr::null(), 2);
            let json = CStr::from_ptr(result).to_str().unwrap();
            assert!(json.contains("\"ok\":false"), "{json}");
            assert!(json.contains("max_block_length"), "{json}");
            repro_toolkit_free_string(result);
        }
    }

    #[test]
    fn validate_null_path_returns_null() {
        unsafe {
            assert!(repro_toolkit_validate_sequence(std::ptr::null(), std::ptr::null()).is_null());
        }
    }
}
