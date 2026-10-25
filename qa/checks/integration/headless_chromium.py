#!/usr/bin/env python3
"""Shared headless-Chromium harness for the browser-backed QA gates.

The real-node, funded-testnet, and real-DOM gates each serve the repository
over loopback HTTP and drive one headless Chromium through DevTools. Browser
startup and teardown are test infrastructure, not the behaviour under test,
so they live here once and must be deterministic on Linux and Windows.
"""

from __future__ import annotations

from collections.abc import Iterator, Mapping
import contextlib
import http.server
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
LAUNCH_ATTEMPTS = 2


class ChromiumError(RuntimeError):
    pass


def windows_browser_candidates(environ: Mapping[str, str] | None = None) -> list[Path]:
    env = os.environ if environ is None else environ
    roots = [
        Path(value)
        for variable in ("PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA")
        if (value := env.get(variable))
    ]
    relative_candidates = (
        Path("Microsoft/Edge/Application/msedge.exe"),
        Path("Google/Chrome/Application/chrome.exe"),
        Path("Chromium/Application/chrome.exe"),
    )
    return [root / relative for root in roots for relative in relative_candidates]


def browser_executable(*, platform_name: str | None = None, environ: Mapping[str, str] | None = None) -> str:
    """Return an installed Chromium-family browser on the current host."""
    path_names = (
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
        "chrome",
        "chrome.exe",
        "msedge",
        "msedge.exe",
    )
    for name in path_names:
        value = shutil.which(name)
        if value:
            return value

    if (os.name if platform_name is None else platform_name) == "nt":
        for candidate in windows_browser_candidates(environ):
            if candidate.is_file():
                return str(candidate)

    raise SystemExit(
        "ERROR: required Chromium-family browser not found "
        "(Chromium, Google Chrome, or Microsoft Edge)"
    )


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


class LoopbackHttpServer(http.server.ThreadingHTTPServer):
    """Loopback server sized for a browser's parallel module requests.

    socketserver's default listen backlog of 5 drops connections when Chromium
    fetches a module graph in parallel, surfacing as a spurious
    "Failed to fetch dynamically imported module".
    """

    request_queue_size = 128
    daemon_threads = True

    def handle_error(self, request, client_address) -> None:
        # Chromium drops idle keep-alive sockets when it exits; that is a
        # normal disconnect, not a server fault worth a traceback.
        if isinstance(sys.exc_info()[1], ConnectionError):
            return
        super().handle_error(request, client_address)


class LoopbackRequestHandler(http.server.SimpleHTTPRequestHandler):
    """Quiet repository file server for one QA browser page.

    HTTP/1.1 keep-alive is required on Windows: one TCP connection per module
    fetch leaves thousands of TIME_WAIT sockets across back-to-back suites,
    until connect() fails with WSAENOBUFS and Chromium reports
    "Failed to fetch dynamically imported module".
    """

    protocol_version = "HTTP/1.1"
    mime_types = {
        ".js": "text/javascript",
        ".mjs": "text/javascript",
        ".wasm": "application/wasm",
        ".json": "application/json",
    }

    def log_message(self, _format: str, *_args: object) -> None:
        pass

    def guess_type(self, path: str) -> str:
        """Use browser-safe MIME types independent of the Windows registry."""
        suffix = Path(urllib.parse.urlsplit(path).path).suffix.lower()
        return self.mime_types.get(suffix) or super().guess_type(path)

    def send_body(self, body: bytes, content_type: str) -> None:
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def send_no_content(self) -> None:
        self.send_response(204)
        self.send_header("Cache-Control", "no-store")
        self.end_headers()

    def do_GET(self) -> None:  # noqa: N802 - stdlib handler API
        if urllib.parse.urlsplit(self.path).path == "/favicon.ico":
            self.send_no_content()
        else:
            super().do_GET()

    def read_result(self, statuses: frozenset[str], max_bytes: int) -> dict[str, object] | None:
        """Read a page's `{status, detail}` JSON result, or answer with an error."""
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            length = -1
        if not 0 < length <= max_bytes:
            self.send_error(413, "invalid result size")
            return None
        try:
            payload = json.loads(self.rfile.read(length))
        except (UnicodeDecodeError, json.JSONDecodeError):
            payload = None
        detail = payload.get("detail") if isinstance(payload, dict) else None
        if not isinstance(detail, dict) or payload.get("status") not in statuses:
            self.send_error(400, "invalid result payload")
            return None
        return {"status": payload["status"], "detail": detail}


@contextlib.contextmanager
def serve(handler: type[LoopbackRequestHandler]) -> Iterator[int]:
    """Serve the repository root with `handler` on a free loopback port."""
    port = free_port()
    server = LoopbackHttpServer(
        ("127.0.0.1", port),
        lambda *args, **kwargs: handler(*args, directory=str(ROOT), **kwargs),
    )
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield port
    finally:
        server.shutdown()
        thread.join(timeout=5)
        server.server_close()


def browser_environment() -> dict[str, str]:
    """Headless Chromium needs no D-Bus; a stale bus address stalls its startup."""
    return {key: value for key, value in os.environ.items() if not key.startswith("DBUS_")}


def chromium_command(chromium: str, debug_port: int, profile_dir: Path) -> list[str]:
    """Real-time headless Chromium; completion comes from the page's result POST."""
    return [
        chromium,
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--no-first-run",
        "--no-default-browser-check",
        "--remote-debugging-address=127.0.0.1",
        f"--remote-debugging-port={debug_port}",
        f"--user-data-dir={profile_dir}",
        "about:blank",
    ]


def wait_for_chromium_debugger(debug_port: int, process: subprocess.Popen[str], timeout: float = 30.0) -> None:
    endpoint = f"http://127.0.0.1:{debug_port}/json/version"
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        returncode = process.poll()
        if returncode is not None:
            raise ChromiumError(f"Chromium exited before DevTools became ready (exit {returncode})")
        try:
            with urllib.request.urlopen(endpoint, timeout=0.25) as response:
                if response.status == 200:
                    return
        except (OSError, TimeoutError, urllib.error.URLError) as error:
            last_error = error
        time.sleep(0.05)
    suffix = f": {last_error}" if last_error else ""
    raise ChromiumError(f"Chromium DevTools did not become ready within {timeout:.0f}s{suffix}")


def open_chromium_target(debug_port: int, target_url: str) -> None:
    encoded = urllib.parse.quote(target_url, safe="")
    request = urllib.request.Request(
        f"http://127.0.0.1:{debug_port}/json/new?{encoded}",
        method="PUT",
    )
    with urllib.request.urlopen(request, timeout=2.0) as response:
        if response.status != 200:
            raise ChromiumError(f"Chromium DevTools target creation failed: HTTP {response.status}")
        payload = json.load(response)
    if not isinstance(payload, dict) or not payload.get("id"):
        raise ChromiumError("Chromium DevTools target creation returned no target id")


def browser_log_tail(browser_log, limit: int = 5000) -> str:
    browser_log.flush()
    browser_log.seek(0)
    return browser_log.read()[-limit:]


def _signal_process_group(pid: int, signum: int) -> None:
    try:
        os.killpg(pid, signum)
    except ProcessLookupError:
        pass


def _kill_process_tree(process: subprocess.Popen[str]) -> None:
    """Kill Chromium with every renderer and profile-writer child.

    Orphaned children keep the profile locked: they make cleanup fail and make
    a relaunch on that profile exit with Chromium's profile-in-use code.
    """
    if os.name == "posix":
        _signal_process_group(process.pid, signal.SIGKILL)
    else:
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )


def stop_chromium(process: subprocess.Popen[str]) -> None:
    """Stop Chromium and all of its children, then reap the parent."""
    if os.name == "posix":
        _signal_process_group(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
    elif process.poll() is None:
        _kill_process_tree(process)
    # The parent may exit before its children; reap any POSIX stragglers.
    if os.name == "posix":
        _kill_process_tree(process)
    if process.poll() is None:
        process.kill()
        process.wait(timeout=5)


def cleanup_chromium_profile(
    profile_dir: Path,
    *,
    attempts: int = 10,
    delay_seconds: float = 0.05,
) -> None:
    """Best-effort cleanup resilient to Chromium's late profile writers."""
    last_error: OSError | None = None
    for attempt in range(attempts):
        try:
            shutil.rmtree(profile_dir)
            return
        except FileNotFoundError:
            return
        except OSError as error:
            last_error = error
            if attempt + 1 < attempts:
                time.sleep(delay_seconds)

    # Profile cleanup must never overwrite the gate result. This directory
    # contains only ephemeral QA browser state under the OS temporary directory.
    if profile_dir.exists():
        print(
            f"WARNING: could not fully remove temporary Chromium profile {profile_dir}: {last_error}",
            file=sys.stderr,
        )


def launch_chromium(chromium: str, debug_port: int, profile_dir: Path, browser_log) -> subprocess.Popen[str]:
    """Start headless Chromium and wait for DevTools, relaunching once.

    A single stalled start is retried before the run fails with the browser
    log. Each attempt gets its own profile under `profile_dir`, so a relaunch
    never contends with a lock left by the attempt it replaces.
    """
    for attempt in range(1, LAUNCH_ATTEMPTS + 1):
        command = chromium_command(chromium, debug_port, profile_dir / f"attempt-{attempt}")
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            env=browser_environment(),
            stdout=browser_log,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=os.name == "posix",
        )
        try:
            wait_for_chromium_debugger(debug_port, process)
            return process
        except ChromiumError as error:
            stop_chromium(process)
            if attempt == LAUNCH_ATTEMPTS:
                output = browser_log_tail(browser_log)
                raise ChromiumError(f"{error}\nLast browser output:\n{output}") from error
    raise AssertionError("unreachable: launch loop always returns or raises")
