#!/usr/bin/env python3
"""Fail if current Steam-facing docs make launch, approval, or legal promises."""

from __future__ import annotations

import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]

TEXT_SURFACES = [
    pathlib.Path("AGENTS.md"),
    pathlib.Path("README.md"),
    pathlib.Path("docs/steam-submission-kit.md"),
    pathlib.Path("docs/steam-compliance-qa.md"),
]

FORBIDDEN_MARKERS = [
    "one-click steam release",
    "one click steam release",
    "automatic steam store release",
    "auto-publish to steam",
    "automatically publish to steam",
    "publish directly to steam",
    "guaranteed approval",
    "guarantees approval",
    "steam-approved",
    "valve-approved",
    "legal guarantee",
    "legal guarantees",
    "guaranteed compliance",
    "compliance guaranteed",
    "platform approval guaranteed",
    "release-ready",
]


def main() -> int:
    failures: list[str] = []
    for relative_path in TEXT_SURFACES:
        path = ROOT / relative_path
        if not path.is_file():
            failures.append(f"{relative_path}: missing expected Steam QA text surface")
            continue
        text = path.read_text(encoding="utf-8")
        lowered = text.lower()
        for marker in FORBIDDEN_MARKERS:
            if marker in lowered:
                failures.append(f"{relative_path}: forbidden launch promise marker `{marker}`")

    if failures:
        print("Steam no-launch-promise lint failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print("Steam no-launch-promise lint passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
