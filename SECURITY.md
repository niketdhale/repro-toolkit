# Security Policy

## Supported versions

Security fixes go into the latest release only.

## Reporting a vulnerability

Please do **not** report security problems in a public issue. Report them privately
instead, through GitHub's "Report a vulnerability" button on the repository's
**Security** tab, or by email to niketdhale12@gmail.com.

Include the affected version, a description of the problem, and a minimal way to
reproduce it (for example, a crafted PDX/ODX snippet). You should get an initial
response within 7 days.

## Scope

repro-toolkit only parses untrusted input: PDX (ZIP) archives, ODX XML and custom
sequence JSON. The following are in scope:
- a crash, panic or hang caused by malformed input,
- excessive memory or CPU use caused by malformed input (for example a zip bomb or
  deeply nested XML),
- memory-safety bugs in the FFI layer (`crates/repro-toolkit-ffi`).

repro-toolkit **never talks to an ECU and never runs seed/key algorithms**. It only
describes a sequence. Security problems in a flashing tool built on top of it belong
to that tool.
