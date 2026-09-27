#!/usr/bin/env python3
"""Regenerates samples/sample.pdx from the same fixture ODX XML used in
repro-toolkit-core's own unit tests (crates/repro-toolkit-core/src/lib.rs),
so the examples in examples/ have a small, real PDX file to run against
without needing a real vendor PDX.

Usage: python3 scripts/generate_sample_pdx.py
"""
import re
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LIB_RS = REPO_ROOT / "crates/repro-toolkit-core/src/lib.rs"
OUTPUT = REPO_ROOT / "samples/sample.pdx"


def extract(name: str, src: str) -> str:
    match = re.search(name + r': &str = r#"(.*?)"#;', src, re.S)
    if not match:
        raise SystemExit(f"could not find {name} in {LIB_RS}")
    return match.group(1)


def main() -> None:
    src = LIB_RS.read_text()
    odx_d = extract("SAMPLE_ODX_D", src)
    odx_f = extract("SAMPLE_ODX_F", src)

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(OUTPUT, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("sample.odx-d", odx_d)
        z.writestr("sample.odx-f", odx_f)

    print(f"wrote {OUTPUT}")


if __name__ == "__main__":
    main()
