#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Pin the trusted release-PR publication boundary."""

from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = (REPOSITORY_ROOT / ".github/workflows/release-plz.yml").read_text(
    encoding="utf-8"
)


def require(fragment: str) -> None:
    """Fail when the release workflow no longer contains an exact guard fragment."""
    if fragment not in WORKFLOW:
        raise AssertionError(f"release workflow is missing:\n{fragment}")


def main() -> None:
    """Verify the automatic and recovery release entry points."""
    require(
        """  workflow_dispatch:
    inputs:
      command:
        description: Release operation
        type: choice
        options:
          - release-pr
          - release
        default: release-pr
  pull_request:
    types: [closed]
    branches:
      - main
"""
    )
    if "\n  push:\n" in WORKFLOW:
        raise AssertionError("CHANGELOG pushes must not trigger publication")

    require(
        """      (github.event_name == 'pull_request' &&
       github.event.action == 'closed' &&
       github.event.pull_request.merged == true &&
       github.event.pull_request.base.ref == 'main' &&
       github.event.pull_request.base.ref == github.event.repository.default_branch &&
       github.event.pull_request.head.repo.full_name == github.repository &&
       startsWith(github.event.pull_request.head.ref, 'release-plz-') &&
       github.event.pull_request.user.id == 286791072 &&
       github.event.pull_request.user.login == 'prismarisk-public-release[bot]' &&
       github.event.pull_request.user.type == 'Bot') ||
      (github.event_name == 'workflow_dispatch' &&
       inputs.command == 'release' &&
       github.ref == 'refs/heads/main')
"""
    )
    require(
        """      github.event_name == 'workflow_dispatch' &&
      inputs.command == 'release-pr' &&
      github.ref == 'refs/heads/main'
"""
    )
    require("command: release\n")
    require("command: release-pr\n")

    print("release workflow trust gates are pinned")


if __name__ == "__main__":
    main()
