#!/usr/bin/env python3
"""Run real Companion WASM against the public Kaspa resolver in real-time Chromium."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse

sys.path.insert(0, str(Path(__file__).resolve().parent))
from headless_chromium import (  # noqa: E402
    ROOT,
    ChromiumError,
    LoopbackRequestHandler,
    browser_executable,
    browser_log_tail,
    cleanup_chromium_profile,
    free_port,
    launch_chromium,
    open_chromium_target,
    serve,
    stop_chromium,
)

SITE = ROOT / "target/kaskold-companion-web/site"
PKG_JS = SITE / "pkg/companion_web.js"
PKG_WASM = SITE / "pkg/companion_web_bg.wasm"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout", type=int, default=75)
    parser.add_argument("--evidence", type=Path)
    return parser.parse_args()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_evidence(path: Path, detail: dict[str, object]) -> None:
    path = path if path.is_absolute() else ROOT / path
    path.parent.mkdir(parents=True, exist_ok=True)
    document = {
        "schema_version": 1,
        "healthy": True,
        "status": "pass",
        "completed_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "network": "mainnet",
        "mode": "production-public-resolver",
        "detail": detail,
        "inputs": {
            "browser_case_sha256": sha256_file(ROOT / "qa/checks/integration/browser_real_node_case.mjs"),
            "browser_runner_sha256": sha256_file(Path(__file__).resolve()),
            "production_resolver_sha256": sha256_file(ROOT / "apps/kaskold-companion-web/web/js/core/node/resolver.js"),
            "wasm_sha256": sha256_file(PKG_WASM),
        },
    }
    path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")

def require_real_wasm() -> None:
    missing = [str(path.relative_to(ROOT)) for path in (PKG_JS, PKG_WASM) if not path.is_file()]
    if missing:
        raise SystemExit("ERROR: real Companion WASM package is missing: " + ", ".join(missing) + ". Run `make companion`.")
    if PKG_WASM.stat().st_size < 4096:
        raise SystemExit("ERROR: Companion WASM package is implausibly small; fixture/stub builds are forbidden here")


class RealNodeHandler(LoopbackRequestHandler):
    result_event = threading.Event()
    result_payload: dict[str, object] | None = None
    max_result_bytes = 1024 * 1024

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler API
        if not self.path.startswith("/__qa_real_node_result__"):
            self.send_error(404)
            return
        result = self.read_result(frozenset({"pass", "fail"}), type(self).max_result_bytes)
        if result is None:
            return
        type(self).result_payload = result
        self.send_no_content()
        type(self).result_event.set()

    def do_GET(self) -> None:  # noqa: N802 - stdlib handler API
        if self.path.startswith("/__qa_real_node__"):
            body = b"""<!doctype html><html><body>running<script type=module>
const publishBootstrapFailure = async (error) => {
  const detail = {
    message: `real-node browser module failed to load: ${error instanceof Error ? error.message : String(error)}`,
    stack: error instanceof Error ? error.stack : null,
  };
  document.documentElement.dataset.qaStatus = 'fail';
  document.body.textContent = JSON.stringify(detail);
  try {
    await fetch('/__qa_real_node_result__', {
      method: 'POST',
      cache: 'no-store',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'fail', detail }),
    });
  } catch (_) {}
};
import('/qa/checks/integration/browser_real_node_case.mjs').catch(publishBootstrapFailure);
</script></body></html>"""
            self.send_body(body, "text/html; charset=utf-8")
        else:
            super().do_GET()


def chromium_run(port: int, timeout: int) -> dict[str, object]:
    chromium = browser_executable()
    debug_port = free_port()
    RealNodeHandler.result_payload = None
    RealNodeHandler.result_event.clear()
    profile_dir = Path(tempfile.mkdtemp(prefix="kaskold-real-node-chrome-"))

    try:
        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as browser_log:
            process = launch_chromium(chromium, debug_port, profile_dir, browser_log)
            try:
                open_chromium_target(
                    debug_port,
                    f"http://127.0.0.1:{port}/__qa_real_node__?network=mainnet",
                )

                deadline = time.monotonic() + timeout
                while True:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        output = browser_log_tail(browser_log)
                        suffix = f"\nLast browser output:\n{output}" if output else ""
                        raise ChromiumError(
                            f"Companion public-node E2E timed out after {timeout} wall-clock seconds using {chromium}{suffix}"
                        )
                    if RealNodeHandler.result_event.wait(timeout=min(0.25, remaining)):
                        break
                    returncode = process.poll()
                    if returncode is not None:
                        output = browser_log_tail(browser_log)
                        raise ChromiumError(
                            f"Chromium public-node E2E exited before returning a structured result "
                            f"(exit {returncode})\n{output}"
                        )

                payload = RealNodeHandler.result_payload
                if payload is None:
                    raise ChromiumError("public-node browser signaled completion without a result payload")
                detail = payload.get("detail")
                if not isinstance(detail, dict):
                    raise ChromiumError("public-node browser returned invalid structured detail")
                if payload.get("status") != "pass":
                    raise ChromiumError("Companion public-node E2E did not pass\n" + json.dumps(detail, indent=2))
                return detail
            finally:
                stop_chromium(process)
    finally:
        cleanup_chromium_profile(profile_dir)


def main() -> int:
    args = parse_args()
    require_real_wasm()
    try:
        with serve(RealNodeHandler) as http_port:
            detail = chromium_run(http_port, args.timeout)
    except (OSError, ChromiumError, TimeoutError, urllib.error.URLError) as error:
        raise SystemExit(f"ERROR: {error}") from None

    if args.evidence is not None:
        write_evidence(args.evidence, detail)

    print("PASS: real Companion WASM connected to mainnet through the production Kaspa public-node resolver")
    ws_url = detail.get("ws_url")
    if ws_url:
        print(f"Node: {ws_url}")
    attempted = detail.get("ws_urls_attempted")
    if isinstance(attempted, list) and attempted:
        print(f"Resolver/retry attempts: {len(attempted)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
