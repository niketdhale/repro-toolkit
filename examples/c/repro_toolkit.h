/*
 * C ABI declarations for repro-toolkit-ffi.
 *
 * Every call returns a heap-allocated, NUL-terminated UTF-8 JSON string of
 * the shape {"ok": true, "sequence": {...}} or {"ok": false, "error": "..."}.
 * The caller must free every non-NULL string returned from this library
 * with repro_toolkit_free_string, and must never free it any other way.
 */
#ifndef REPRO_TOOLKIT_H
#define REPRO_TOOLKIT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Parse a PDX file and return its default, auto-generated UDS repro
 * sequence as JSON. Returns NULL only if `path` itself is NULL. */
char *repro_toolkit_parse_pdx(const char *path);

/* Like repro_toolkit_parse_pdx, but if `sequence_path` is non-NULL it is
 * used as a custom sequence JSON file instead of auto-generating the
 * sequence from the PDX (see docs/custom-sequence-guide.md). Pass NULL
 * for `sequence_path` to get the same behavior as repro_toolkit_parse_pdx.
 * Returns NULL only if `pdx_path` itself is NULL. */
char *repro_toolkit_generate_sequence(const char *pdx_path, const char *sequence_path);

/* Validates the repro sequence for `pdx_path` (optionally overridden by
 * `sequence_path`, exactly as above) and returns
 * {"ok": true, "valid": bool, "issues": [...]} as JSON, or the usual
 * {"ok": false, "error": "..."} on failure. Returns NULL only if
 * `pdx_path` itself is NULL. */
char *repro_toolkit_validate_sequence(const char *pdx_path, const char *sequence_path);

/* Same as the two functions above, but with an explicit TransferData
 * max_block_length (UDS maxNumberOfBlockLength, including the SID and
 * block sequence counter bytes). Pass 0 for the default (0x0FFF). */
char *repro_toolkit_generate_sequence_ex(const char *pdx_path, const char *sequence_path,
                                         uint32_t max_block_length);
char *repro_toolkit_validate_sequence_ex(const char *pdx_path, const char *sequence_path,
                                         uint32_t max_block_length);

/* Frees a string previously returned by any repro_toolkit_* function.
 * Passing NULL is a no-op. */
void repro_toolkit_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* REPRO_TOOLKIT_H */
