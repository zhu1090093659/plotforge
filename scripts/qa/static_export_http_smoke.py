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
    index_html = export_dir / "index.html"
    player_js = export_dir / "player.js"
    player_core_js = export_dir / "player-core.js"
    styles_css = export_dir / "styles.css"
    assert_file(index_html)
    assert_file(player_js)
    assert_file(player_core_js)
    assert_file(styles_css)
    game_json = export_dir / "game.json"
    ai_usage_json = export_dir / "ai-usage.json"
    assert_file(game_json)
    assert_file(ai_usage_json)

    manifest = json.loads(game_json.read_text(encoding="utf-8"))
    ai_usage = json.loads(ai_usage_json.read_text(encoding="utf-8"))
    assert manifest["game"]["title"] == "Dynasty Embers"
    assert manifest["entry_scene"] == "court-crisis-001"
    assert manifest["profile"]["id"] == "static-web"
    assert manifest["profile"]["requires_network_at_runtime"] is False
    assert manifest["ai_usage_manifest_path"] == "ai-usage.json"
    assert len(manifest["scenes"]) >= 1
    assert len(manifest["scenes"][0]["beats"][0]["choices"]) >= 3
    assert ai_usage["project_id"] == "dynasty-embers"
    assert ai_usage["export_profile"]["id"] == "static-web"
    assert ai_usage["external_model_calls_during_export"] is False
    assert ai_usage["provider_credentials_included"] is False
    assert ai_usage["raw_provider_responses_included"] is False
    assert ai_usage["private_traces_included"] is False

    for asset in manifest["assets"]:
        assert_file(export_dir / asset)
    assert_whitelisted_files(export_dir, manifest["assets"])
    assert_no_network_urls([index_html, player_js, player_core_js, styles_css])
    assert_no_secret_markers([game_json, ai_usage_json])

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
        ai_usage_text = fetch(f"http://127.0.0.1:{port}/ai-usage.json")
        player = fetch(f"http://127.0.0.1:{port}/player.js")
        styles = fetch(f"http://127.0.0.1:{port}/styles.css")
        assert "PlotForge Player" in index
        assert 'src="./player.js"' in index
        assert 'href="./styles.css"' in index
        assert 'data-field="progress"' in index
        assert 'data-field="outcome"' in index
        assert "Dynasty Embers" in game
        assert "static-web" in ai_usage_text
        assert "bootPlayer" in player
        assert ".pf-player" in styles
    finally:
        server.terminate()
        server.wait(timeout=5)

    print(f"static export HTTP smoke passed: {export_dir}")
    return 0


def assert_file(path: pathlib.Path) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise AssertionError(f"missing or empty file: {path}")


def assert_whitelisted_files(export_dir: pathlib.Path, assets: list[str]) -> None:
    expected = {
        "index.html",
        "ai-usage.json",
        "game.json",
        "player-core.js",
        "player.js",
        "styles.css",
        *assets,
    }
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


def assert_no_network_urls(paths: list[pathlib.Path]) -> None:
    blocked_markers = ["http://", "https://", "//cdn.", "//unpkg.", "//fonts."]
    for path in paths:
        text = path.read_text(encoding="utf-8")
        blocked = [marker for marker in blocked_markers if marker in text]
        if blocked:
            raise AssertionError(f"network URL marker(s) {blocked} found in {path}")


def assert_no_secret_markers(paths: list[pathlib.Path]) -> None:
    blocked_markers = [
        "OPENAI_API_KEY",
        "api_key",
        "secret_key",
        "sk-",
        "authorization:",
        "bearer",
        "token=",
        "raw_response",
        "request_id",
    ]
    for path in paths:
        text = path.read_text(encoding="utf-8")
        blocked = [marker for marker in blocked_markers if marker in text]
        if blocked:
            raise AssertionError(f"secret marker(s) {blocked} found in {path}")


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
