#!/usr/bin/env python3
import argparse
import json
import os
import pathlib
import socket
import subprocess
import sys
import time
import urllib.request


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke test a PlotForge static export over localhost HTTP.")
    parser.add_argument("--export-dir", required=True, type=pathlib.Path)
    args = parser.parse_args()

    export_dir = args.export_dir.resolve()
    assert_file(export_dir / "index.html")
    game_json = export_dir / "game.json"
    assert_file(game_json)

    manifest = json.loads(game_json.read_text(encoding="utf-8"))
    assert manifest["game"]["title"] == "Dynasty Embers"
    assert manifest["entry_scene"] == "court-crisis-001"
    assert len(manifest["scenes"]) >= 1
    assert len(manifest["scenes"][0]["beats"][0]["choices"]) >= 3

    for asset in manifest["assets"]:
        assert_file(export_dir / asset)
    assert_whitelisted_files(export_dir, manifest["assets"])

    port = free_port()
    server = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(port), "--directory", str(export_dir)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        wait_for_http(port)
        index = fetch(f"http://127.0.0.1:{port}/index.html")
        game = fetch(f"http://127.0.0.1:{port}/game.json")
        assert "PlotForge Player" in index
        assert "Dynasty Embers" in game
    finally:
        server.terminate()
        server.wait(timeout=5)

    print(f"static export HTTP smoke passed: {export_dir}")
    return 0


def assert_file(path: pathlib.Path) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise AssertionError(f"missing or empty file: {path}")


def assert_whitelisted_files(export_dir: pathlib.Path, assets: list[str]) -> None:
    expected = {"index.html", "game.json", *assets}
    actual = {
        path.relative_to(export_dir).as_posix()
        for path in export_dir.rglob("*")
        if path.is_file()
    }
    unexpected = sorted(actual - expected)
    missing = sorted(expected - actual)
    if unexpected:
        raise AssertionError(f"unexpected files in export package: {unexpected}")
    if missing:
        raise AssertionError(f"missing files in export package: {missing}")

    blocked_dirs = {"traces", "providers", "provider_config", "raw_responses"}
    blocked = sorted(
        file
        for file in actual
        if any(part in blocked_dirs for part in pathlib.PurePosixPath(file).parts)
    )
    if blocked:
        raise AssertionError(f"private files leaked into export package: {blocked}")


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_http(port: int) -> None:
    deadline = time.time() + 5
    last_error = None
    while time.time() < deadline:
        try:
            fetch(f"http://127.0.0.1:{port}/game.json")
            return
        except Exception as exc:
            last_error = exc
            time.sleep(0.1)
    raise RuntimeError(f"server did not become ready: {last_error}")


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=5) as response:
        return response.read().decode("utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
