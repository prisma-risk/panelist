#!/usr/bin/env python3
"""Verify that first-party Rust sources carry the Apache-2.0 SPDX header."""

from __future__ import annotations

import pathlib
import sys

EXPECTED = "// SPDX-License-Identifier: Apache-2.0"
ROOT = pathlib.Path(__file__).resolve().parents[1]


def rust_sources() -> list[pathlib.Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.rs")
        if "target" not in path.parts and ".git" not in path.parts
    )


def main() -> int:
    invalid = []
    for path in rust_sources():
        first_line = path.read_text(encoding="utf-8").splitlines()[:1]
        if first_line != [EXPECTED]:
            invalid.append(path.relative_to(ROOT))

    if invalid:
        for path in invalid:
            print(f"missing canonical SPDX header: {path}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
