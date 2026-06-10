#!/usr/bin/env python3
import argparse
import pathlib
import re


REQUIRED_UI_MARKERS = [
    "PlotForge Studio",
    "Creator Desktop",
    "Dynasty Embers",
    "Project Launchpad",
    "Command Dock",
    "Agent Mesh",
    "ACP Bridge Setup",
    "Playable Proof",
    "Trace Debug",
    "Export Package",
    "Local export package only",
    "Runtime Trace",
    "No raw responses",
    "No secret markers",
    "trace-001",
]

BLOCKED_MARKERS = [
    "sk-test-secret should not render",
    "automatic publishing",
    "approval guarantee",
    "legal guarantee",
    "one-click Steam launch",
    "Steam upload automation",
    "real ACP execution",
]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Smoke test the built PlotForge Creator Desktop bundle."
    )
    parser.add_argument(
        "--dist-dir",
        default="apps/creator-desktop/dist",
        type=pathlib.Path,
        help="Built Vite distribution directory.",
    )
    args = parser.parse_args()

    dist_dir = args.dist_dir.resolve()
    index = dist_dir / "index.html"
    assert_file(index)

    index_text = index.read_text(encoding="utf-8")
    if 'id="root"' not in index_text:
        raise AssertionError(f"missing Vite root element in {index}")

    asset_paths = find_asset_paths(index_text)
    if not asset_paths:
        raise AssertionError(f"no script or stylesheet assets referenced by {index}")

    asset_texts = []
    for asset_path in asset_paths:
        asset = resolve_asset(dist_dir, asset_path)
        assert_file(asset)
        if asset.suffix in {".js", ".css"}:
            asset_texts.append(asset.read_text(encoding="utf-8"))

    bundled_text = "\n".join(asset_texts)
    assert_markers_present(bundled_text, REQUIRED_UI_MARKERS)
    assert_markers_absent(bundled_text, BLOCKED_MARKERS)

    print(f"creator desktop build smoke passed: {dist_dir}")
    return 0


def find_asset_paths(index_text: str) -> list[str]:
    return re.findall(r'(?:src|href)="([^"]+)"', index_text)


def resolve_asset(dist_dir: pathlib.Path, asset_path: str) -> pathlib.Path:
    normalized = asset_path.lstrip("/")
    resolved = (dist_dir / normalized).resolve()
    if dist_dir not in resolved.parents and resolved != dist_dir:
        raise AssertionError(f"asset path escapes dist directory: {asset_path}")
    return resolved


def assert_file(path: pathlib.Path) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise AssertionError(f"missing or empty file: {path}")


def assert_markers_present(text: str, markers: list[str]) -> None:
    missing = [marker for marker in markers if marker not in text]
    if missing:
        raise AssertionError(f"missing Creator Desktop UI markers: {missing}")


def assert_markers_absent(text: str, markers: list[str]) -> None:
    present = [marker for marker in markers if marker in text]
    if present:
        raise AssertionError(f"blocked Creator Desktop markers found: {present}")


if __name__ == "__main__":
    raise SystemExit(main())
