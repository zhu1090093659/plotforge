#!/usr/bin/env python3
"""Fail if current Steam-facing docs make launch, approval, or legal promises."""

from __future__ import annotations

import re
from dataclasses import dataclass
import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]

TEXT_SURFACES = [
    pathlib.Path("AGENTS.md"),
    pathlib.Path("README.md"),
    pathlib.Path("docs/steam-submission-kit.md"),
    pathlib.Path("docs/steam-compliance-qa.md"),
    pathlib.Path("docs/prd-completion-audit.md"),
]


@dataclass(frozen=True)
class ForbiddenPattern:
    label: str
    regex: re.Pattern[str]


def pattern(label: str, expression: str) -> ForbiddenPattern:
    return ForbiddenPattern(label, re.compile(expression, re.IGNORECASE))


FORBIDDEN_PATTERNS = [
    pattern(
        "one-button Steam store publishing",
        r"\b(?:one[- ]click|single[- ]click|1[- ]click)\s+(?:steam\s+)?(?:store\s+)?"
        r"(?:release|launch|publish(?:ing)?|submission|upload)\b",
    ),
    pattern(
        "automatic Steam store publishing",
        r"\b(?:automatic\s+(?:(?:steam\s+)(?:store\s+)?"
        r"(?:release|launch|publish(?:ing)?|submission|upload)|"
        r"(?:store\s+)?(?:release|launch|publish(?:ing)?|submission|upload)\s+(?:to\s+)?steam)|"
        r"automatically\s+(?:release|launch|publish|submit|upload)\s+(?:directly\s+)?(?:to\s+)?steam|"
        r"auto[- ]publish(?:ing)?\s+(?:directly\s+)?(?:to\s+)?steam)\b",
    ),
    pattern(
        "direct Steam publishing claim",
        r"\b(?:publish(?:es|ing)?|release(?:s|ing)?|launch(?:es|ing)?|submit(?:s|ting)?|upload(?:s|ing)?)"
        r"\s+(?:directly\s+)?(?:to\s+)?steam(?:\s+store)?\b",
    ),
    pattern(
        "approval or compliance guarantee",
        r"\b(?:guaranteed|guarantees?|ensures?|certifies?)\s+"
        r"(?:steam\s+|platform\s+|valve\s+)?(?:approval|acceptance|compliance|release|launch|publishing)\b",
    ),
    pattern(
        "approval or compliance guaranteed",
        r"\b(?:steam\s+|platform\s+|valve\s+)?(?:approval|acceptance|compliance|release|launch|publishing)"
        r"\s+(?:is\s+)?(?:guaranteed|ensured|certified)\b",
    ),
    pattern(
        "Steam or Valve endorsement claim",
        r"\b(?:steam|valve)[- ]approved\b|\bapproved\s+by\s+(?:steam|valve)\b",
    ),
    pattern(
        "legal advice or guarantee",
        r"\b(?:provides?|serves\s+as|is)\s+(?:legal\s+)?(?:advice|counsel|guidance)\b|"
        r"\blegal\s+(?:guarantee|guarantees|certification|approval)\b",
    ),
    pattern(
        "legal or compliance status claim",
        r"\b(?:legally|fully)\s+compliant\b|\b(?:compliance|legality)\s+(?:proof|certificate|certification)\b",
    ),
    pattern(
        "PlotForge-owned Steamworks workflow",
        r"\bplotforge\s+(?:handles|manages|owns|submits|uploads|publishes|releases|pays|registers)\b"
        r".{0,80}\bsteam(?:works| direct| store)?\b",
    ),
    pattern(
        "first-person Steamworks workflow ownership",
        r"\bwe\s+(?:handle|manage|own|submit|upload|publish|release|pay|register)\b"
        r".{0,80}\bsteam(?:works| direct| store)?\b",
    ),
    pattern(
        "release-ready status claim",
        r"\b(?:steam[- ]ready|release[- ]ready|ready\s+for\s+steam\s+release|ready\s+for\s+review)\b",
    ),
    pattern(
        "Steam app id leakage",
        r"\b(?:steam[_ -]?app[_ -]?id|steam_appid|app[_ -]?id)\s*[:=]\s*\d{3,}\b",
    ),
    pattern(
        "Steam depot or build id leakage",
        r"\b(?:depot|build)[_ -]?id\s*[:=]\s*\d{3,}\b",
    ),
    pattern(
        "Steam upload or release state leakage",
        r"\b(?:upload|publish|release)[_ -]?state\s*[:=]\s*"
        r"(?:uploaded|submitted|approved|released|live|ready_for_review|ready-for-review)\b",
    ),
    pattern("Chinese one-click Steam publishing", r"(?:一键|自动).{0,12}(?:上架|发布|提交|上传).{0,12}Steam"),
    pattern("Chinese managed Steam publishing", r"(?:代办|代管|代上架).{0,12}Steam"),
    pattern("Chinese approval or legal guarantee", r"(?:保证|确保).{0,12}(?:通过|批准|合规)|(?:法律意见|法务意见|合规保证)"),
]


SELF_TEST_FORBIDDEN = [
    "one-click Steam release",
    "single click Steam store publishing",
    "automatically publish to Steam",
    "auto-publish to Steam",
    "publish directly to Steam",
    "guaranteed approval",
    "platform compliance is guaranteed",
    "Steam-approved package",
    "approved by Valve",
    "this provides legal advice",
    "fully compliant",
    "PlotForge handles the creator Steamworks workflow",
    "we submit the build to Steam",
    "release-ready",
    "steam_appid=123456",
    "depot id: 987654",
    "upload_state=ready_for_review",
    "一键上架 Steam",
    "代管 Steam Direct",
    "保证通过审核",
]

SELF_TEST_ALLOWED = [
    "Local evidence preparation only.",
    "Creator-owned Steamworks review workflow.",
    "Generated drafts do not decide platform outcomes.",
    "No Steam app IDs or upload state may be written into generated drafts.",
    "Steam-facing docs must pass the no-launch-promise lint.",
]


def scan_text(relative_path: pathlib.Path | str, text: str) -> list[str]:
    failures: list[str] = []
    for forbidden in FORBIDDEN_PATTERNS:
        for match in forbidden.regex.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            snippet = " ".join(match.group(0).split())
            failures.append(
                f"{relative_path}:{line}: {forbidden.label} marker `{snippet}`"
            )
    return failures


def run_self_tests() -> list[str]:
    failures: list[str] = []
    for sample in SELF_TEST_FORBIDDEN:
        if not scan_text("<self-test-forbidden>", sample):
            failures.append(f"self-test missed forbidden marker `{sample}`")
    for sample in SELF_TEST_ALLOWED:
        matches = scan_text("<self-test-allowed>", sample)
        if matches:
            failures.extend(f"self-test false positive for `{sample}`: {match}" for match in matches)
    return failures


def main() -> int:
    failures: list[str] = []
    failures.extend(run_self_tests())

    for relative_path in TEXT_SURFACES:
        path = ROOT / relative_path
        if not path.is_file():
            failures.append(f"{relative_path}: missing expected Steam QA text surface")
            continue
        text = path.read_text(encoding="utf-8")
        failures.extend(scan_text(relative_path, text))

    if failures:
        print("Steam no-launch-promise lint failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print("Steam no-launch-promise lint passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
