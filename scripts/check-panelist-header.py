#!/usr/bin/env python3
"""Verify or repair the Panelist copyright header on Rust source files.

The canonical, language-neutral body lives in ``scripts/header.txt``. This
script renders it as a Rust ``//`` comment block and requires it at the very
top of every first-party ``.rs`` file.
"""

from __future__ import annotations

import argparse
import sys
from functools import lru_cache
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
COMMENT_PREFIX = "//"
RUST_EXTENSION = ".rs"
EXCLUDED_DIR_NAMES = frozenset({"target", ".git", ".venv", "node_modules"})
HEADER_PATH = Path(__file__).resolve().parent / "header.txt"
EXISTING_HEADER_SIGNATURES = (
    "Prisma Risk",
    "SPDX-License-Identifier: Apache-2.0",
)


@lru_cache(maxsize=1)
def render_header() -> str:
    """Render the canonical header body as a Rust line-comment block."""
    rendered = [
        f"{COMMENT_PREFIX}{line}" if line else COMMENT_PREFIX
        for line in HEADER_PATH.read_text(encoding="utf-8").splitlines()
    ]
    return "\n".join(rendered) + "\n//\n"


def display_path(path: Path) -> str:
    """Return a repository-relative path when possible."""
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(path)


def is_excluded(path: Path) -> bool:
    """Return whether a path belongs to a generated or third-party tree."""
    try:
        parts = path.resolve().relative_to(REPO_ROOT).parts
    except ValueError:
        return False
    return bool(EXCLUDED_DIR_NAMES.intersection(parts))


def header_matches(text: str) -> bool:
    """Compare the leading lines to the canonical header."""
    expected = render_header().splitlines()
    actual = text.splitlines()
    return len(actual) >= len(expected) and all(
        actual_line.rstrip() == expected_line.rstrip()
        for actual_line, expected_line in zip(actual[: len(expected)], expected)
    )


def find_existing_header_span(text: str) -> tuple[int, int] | None:
    """Find a stale Panelist or SPDX header at the start of a file."""
    lines = text.splitlines(keepends=True)
    end = 0
    while end < len(lines) and lines[end].lstrip().startswith(COMMENT_PREFIX):
        end += 1
    if end == 0:
        return None
    leading_comments = "".join(lines[:end])
    if not any(signature in leading_comments for signature in EXISTING_HEADER_SIGNATURES):
        return None
    while end < len(lines) and lines[end].strip() == "":
        end += 1
    return (0, sum(len(line) for line in lines[:end]))


def apply_fix(path: Path) -> None:
    """Replace a stale header or prepend the canonical header."""
    original = path.read_text(encoding="utf-8")
    span = find_existing_header_span(original)
    body = original[span[1] :] if span is not None else original
    separator = "" if not body or body.startswith("\n") else "\n"
    path.write_text(render_header() + separator + body, encoding="utf-8")


def iter_source_files(targets: list[Path]) -> list[Path]:
    """Resolve targets to a sorted, deduplicated list of Rust source files."""
    paths: set[Path] = set()

    def add(path: Path) -> None:
        if path.suffix == RUST_EXTENSION and not is_excluded(path):
            paths.add(path.resolve())

    if not targets:
        targets = [REPO_ROOT]
    for target in targets:
        if target.is_dir():
            for path in target.rglob(f"*{RUST_EXTENSION}"):
                add(path)
        elif target.is_file():
            add(target)
    return sorted(paths)


def main() -> int:
    """Check or repair canonical headers."""
    parser = argparse.ArgumentParser(
        description="Verify or repair the Panelist header on Rust source files."
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="repair files with missing or stale headers",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="files or directories to check (default: whole repository)",
    )
    args = parser.parse_args()

    paths = iter_source_files(args.paths)
    if not paths:
        print("no Rust source files to check", file=sys.stderr)
        return 0

    invalid: list[Path] = []
    fixed: list[Path] = []
    for path in paths:
        if header_matches(path.read_text(encoding="utf-8")):
            continue
        if args.fix:
            apply_fix(path)
            fixed.append(path)
        else:
            invalid.append(path)

    if args.fix:
        for path in fixed:
            print(f"fixed: {display_path(path)}")
        print(
            f"repaired {len(fixed)} file(s)."
            if fixed
            else "all files already have the canonical header."
        )
        return 0

    if invalid:
        for path in invalid:
            print(f"missing/stale header: {display_path(path)}", file=sys.stderr)
        print(
            f"\n{len(invalid)} file(s) need the Panelist header. "
            "Run with --fix to repair.",
            file=sys.stderr,
        )
        return 1

    print(f"checked {len(paths)} file(s): all headers OK.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
