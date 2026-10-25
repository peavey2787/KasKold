#!/usr/bin/env python3
"""Persistent funded-wallet E2E against a real Kaspa public testnet node."""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import stat
import subprocess
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
FUNDED_BROWSER_CASE = ROOT / "qa/checks/integration/funded_testnet_e2e_case.mjs"
def funded_state_root() -> Path:
    override = os.environ.get("KASKOLD_FUNDED_E2E_STATE_DIR")
    if override:
        return Path(override).expanduser()
    xdg_state = os.environ.get("XDG_STATE_HOME")
    base = Path(xdg_state).expanduser() if xdg_state else Path.home() / ".local/state"
    return base / "kaskold/funded-e2e"


STATE_ROOT = funded_state_root()
FAUCETS = {
    "testnet-10": "https://faucet-testnet.kaspanet.io/",
    "testnet-12": "https://faucet-tn12.kaspanet.io/",
}
NETWORKS = (("1", "testnet-10", "Testnet-10"), ("2", "testnet-12", "Testnet-12"))
POLL_SECONDS = 10
CONFIRMATION_TIMEOUT_SECONDS = 600
STATUS_TIMEOUT_SECONDS = 90
RPC_PHASE_TIMEOUT_SECONDS = 180
STATUS_ATTEMPTS = 3
STATUS_RETRY_SECONDS = 5
LOCAL_ADDRESS_TIMEOUT_SECONDS = 20
NONINTERACTIVE_SKIP = 77
NETWORK_SELECTION_TIMEOUT_SECONDS = 120



def interactive_stdin_available(stream=None) -> bool:
    """Return whether funded-E2E can safely ask the maintainer for input.

    Only stdin needs to be interactive.  stdout is frequently tee'd or captured
    by Windows launchers even while the console remains available for input.
    On native Windows, fall back to GetConsoleMode for runtimes whose isatty()
    does not recognize the inherited console handle.
    """
    input_stream = sys.stdin if stream is None else stream
    if input_stream is None:
        return False
    try:
        if input_stream.isatty():
            return True
    except (AttributeError, OSError, ValueError):
        return False
    if os.name != "nt":
        return False
    try:
        import ctypes
        import msvcrt

        handle = msvcrt.get_osfhandle(input_stream.fileno())
        mode = ctypes.c_ulong()
        return bool(ctypes.windll.kernel32.GetConsoleMode(handle, ctypes.byref(mode)))
    except (AttributeError, OSError, ValueError):
        return False

class BrowserPhaseError(RuntimeError):
    def __init__(self, message: str, *, retryable: bool = False) -> None:
        super().__init__(message)
        self.retryable = retryable


def restore_windows_console_line_input(stream=None) -> bool:
    """Restore normal line/echo input before an interactive Windows prompt.

    Native console programs such as QEMU may temporarily switch the shared
    console input handle into raw/VT mode.  A later Python input() can then
    appear frozen: keystrokes are not echoed and Enter does not complete the
    line normally.  Keep this repair Windows-only and only touch a real console
    handle; redirected/non-console stdin is left unchanged.
    """
    if os.name != "nt":
        return False
    input_stream = sys.stdin if stream is None else stream
    if input_stream is None:
        return False
    try:
        import ctypes
        import msvcrt

        handle = msvcrt.get_osfhandle(input_stream.fileno())
        mode = ctypes.c_ulong()
        kernel32 = ctypes.windll.kernel32
        if not kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            return False
        enable_processed_input = 0x0001
        enable_line_input = 0x0002
        enable_echo_input = 0x0004
        enable_virtual_terminal_input = 0x0200
        repaired = (
            mode.value
            | enable_processed_input
            | enable_line_input
            | enable_echo_input
        ) & ~enable_virtual_terminal_input
        if repaired != mode.value and not kernel32.SetConsoleMode(handle, repaired):
            return False
        return True
    except (AttributeError, OSError, ValueError):
        return False


def interactive_input(prompt: str) -> str:
    # Reassert canonical line/echo mode before every prompt because make qa can
    # run native console programs immediately beforehand.
    restore_windows_console_line_input()
    return input(prompt)


def _windows_timed_input(prompt: str, timeout_seconds: float) -> str | None:
    import msvcrt

    restore_windows_console_line_input()
    sys.stdout.write(prompt)
    sys.stdout.flush()
    deadline = time.monotonic() + timeout_seconds
    characters: list[str] = []
    while time.monotonic() < deadline:
        if not msvcrt.kbhit():
            time.sleep(0.05)
            continue
        character = msvcrt.getwch()
        if character in {"\r", "\n"}:
            sys.stdout.write("\n")
            sys.stdout.flush()
            return "".join(characters)
        if character == "\x03":
            raise KeyboardInterrupt
        if character in {"\x00", "\xe0"}:
            if msvcrt.kbhit():
                msvcrt.getwch()
            continue
        if character in {"\b", "\x7f"}:
            if characters:
                characters.pop()
                sys.stdout.write("\b \b")
                sys.stdout.flush()
            continue
        if character.isprintable():
            characters.append(character)
            sys.stdout.write(character)
            sys.stdout.flush()
    sys.stdout.write("\n")
    sys.stdout.flush()
    return None


def timed_interactive_input(prompt: str, timeout_seconds: float) -> str | None:
    """Read one interactive line, returning None when the deadline expires."""
    if timeout_seconds <= 0:
        return None
    if os.name == "nt":
        return _windows_timed_input(prompt, timeout_seconds)

    import select

    restore_windows_console_line_input()
    sys.stdout.write(prompt)
    sys.stdout.flush()
    try:
        readable, _writable, _exceptional = select.select([sys.stdin], [], [], timeout_seconds)
    except (OSError, ValueError):
        readable = []
    if not readable:
        sys.stdout.write("\n")
        sys.stdout.flush()
        return None
    line = sys.stdin.readline()
    if line == "":
        return None
    return line.rstrip("\r\n")


def select_network(timeout_seconds: float = NETWORK_SELECTION_TIMEOUT_SECONDS) -> tuple[str, str]:
    print("Choose the Kaspa network for the funded wallet E2E:")
    for number, _network, label in NETWORKS:
        print(f"  {number}) {label}")
    while True:
        response = timed_interactive_input("Network [1]: ", timeout_seconds)
        if response is None:
            print(
                f"No network selection received within {int(timeout_seconds)} seconds; "
                "automatically selecting Testnet-10."
            )
            return "testnet-10", "Testnet-10"
        choice = response.strip() or "1"
        normalized = choice.lower()
        for number, network, label in NETWORKS:
            if normalized in (number, network, label.lower()):
                return network, label
        print("Please choose 1 (Testnet-10) or 2 (Testnet-12).")


def build_real_wasm() -> None:
    print("\n==> Building the real KasKold Companion WebAssembly package")
    subprocess.run(["make", "companion"], cwd=ROOT, check=True)


def require_real_wasm() -> None:
    missing = [str(path.relative_to(ROOT)) for path in (PKG_JS, PKG_WASM) if not path.is_file()]
    if missing:
        raise SystemExit("ERROR: real Companion WASM package is missing: " + ", ".join(missing))
    if PKG_WASM.stat().st_size < 4096:
        raise SystemExit("ERROR: Companion WASM package is implausibly small; fixture/stub builds are forbidden here")


def ensure_state_directory() -> None:
    STATE_ROOT.mkdir(parents=True, exist_ok=True)
    if os.name == "posix":
        STATE_ROOT.chmod(stat.S_IRWXU)


def ensure_tools_lock_current() -> None:
    """Refresh a stale tools lock transactionally, then require --locked."""
    toolchain = os.environ.get("KASKOLD_STABLE_RUST", "").strip()
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+){2}", toolchain):
        raise SystemExit("ERROR: KASKOLD_STABLE_RUST is not a pinned stable Rust version")

    manifest = ROOT / "tools/Cargo.toml"
    lockfile = ROOT / "tools/Cargo.lock"
    base = [
        "rustup",
        "run",
        toolchain,
        "cargo",
        "metadata",
        "--manifest-path",
        str(manifest),
        "--format-version",
        "1",
    ]
    locked = subprocess.run([*base, "--locked"], cwd=ROOT, capture_output=True, text=True)
    if locked.returncode == 0:
        return

    original = lockfile.read_bytes() if lockfile.exists() else None
    print("  Funded-E2E tools lock is stale; refreshing it transactionally with pinned Cargo.")
    refresh = subprocess.run([*base, "--offline"], cwd=ROOT, capture_output=True, text=True)
    if refresh.returncode != 0:
        refresh = subprocess.run(base, cwd=ROOT, capture_output=True, text=True)
    if refresh.returncode != 0:
        if original is None:
            lockfile.unlink(missing_ok=True)
        else:
            lockfile.write_bytes(original)
        detail = (refresh.stderr or refresh.stdout).strip()
        raise SystemExit(f"ERROR: funded-E2E could not refresh tools/Cargo.lock\n{detail}")

    verified = subprocess.run([*base, "--locked"], cwd=ROOT, capture_output=True, text=True)
    if verified.returncode != 0:
        if original is None:
            lockfile.unlink(missing_ok=True)
        else:
            lockfile.write_bytes(original)
        detail = (verified.stderr or verified.stdout).strip()
        raise SystemExit(f"ERROR: refreshed tools/Cargo.lock still fails --locked\n{detail}")
    print("  Refreshed and verified tools/Cargo.lock with pinned Cargo.")


def run_signer_helper(*args: str) -> str:
    toolchain = os.environ.get("KASKOLD_STABLE_RUST", "").strip()
    if not re.fullmatch(r"[0-9]+(?:\.[0-9]+){2}", toolchain):
        raise SystemExit("ERROR: KASKOLD_STABLE_RUST is not a pinned stable Rust version")
    command = [
        "rustup",
        "run",
        toolchain,
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "--manifest-path",
        str(ROOT / "tools/Cargo.toml"),
        "--bin",
        "kaskold-funded-e2e",
        "--",
        *args,
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise SystemExit(f"ERROR: funded-E2E signer helper failed\n{detail}")
    return result.stdout.strip()


def ensure_wallet(network: str) -> tuple[Path, str, bool]:
    ensure_state_directory()
    secret_path = STATE_ROOT / f"{network}.wallet"
    output = run_signer_helper("wallet", str(secret_path))
    values = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
    kpub = values.get("kpub", "")
    if not kpub.startswith("kpub1:"):
        raise SystemExit("ERROR: funded-E2E helper did not return a canonical kpub")
    return secret_path, kpub, values.get("created") == "1"


def sign_kspt(secret_path: Path, kspt_hex: str) -> str:
    signed = run_signer_helper("sign", str(secret_path), kspt_hex)
    if not re.fullmatch(r"[0-9a-f]+", signed):
        raise SystemExit("ERROR: funded-E2E signer helper returned invalid signed KSPT")
    return signed


class FundedHandler(LoopbackRequestHandler):
    config: dict[str, object] = {}
    result_event = threading.Event()
    result_payload: dict[str, object] | None = None
    max_result_bytes = 8 * 1024 * 1024

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler API
        if not self.path.startswith("/__qa_funded_result__"):
            self.send_error(404)
            return
        result = self.read_result(frozenset({"pass", "pending", "fail"}), type(self).max_result_bytes)
        if result is None:
            return
        type(self).result_payload = result
        self.send_no_content()
        type(self).result_event.set()

    def do_GET(self) -> None:  # noqa: N802 - stdlib handler API
        if self.path.startswith("/__qa_funded_config__"):
            self.send_body(json.dumps(type(self).config, separators=(",", ":")).encode(), "application/json")
        elif self.path.startswith("/__qa_funded_e2e__"):
            self.send_body(funded_browser_document(), "text/html; charset=utf-8")
        else:
            super().do_GET()


def funded_browser_document() -> bytes:
    """Embed the funded-E2E harness so Chromium does not fetch it dynamically."""
    module_source = FUNDED_BROWSER_CASE.read_text(encoding="utf-8").replace(
        "</script", r"<\/script"
    )
    prefix = """<!doctype html><html><body>running
<script id=\"funded-e2e-module\" type=\"module\">
"""
    suffix = """
</script></body></html>"""
    return (prefix + module_source + suffix).encode("utf-8")


def start_phase_browser(
    chromium: str,
    debug_port: int,
    profile_dir: Path,
    browser_log,
    port: int,
    phase: str,
) -> subprocess.Popen[str]:
    """Launch Chromium and open the funded-E2E tab, or fail the phase retryably."""
    process: subprocess.Popen[str] | None = None
    try:
        process = launch_chromium(chromium, debug_port, profile_dir, browser_log)
        open_chromium_target(debug_port, f"http://127.0.0.1:{port}/__qa_funded_e2e__")
        return process
    except (OSError, ChromiumError, urllib.error.URLError) as error:
        if process is not None:
            stop_chromium(process)
        output = browser_log_tail(browser_log)
        suffix = f"\n{output}" if output else ""
        raise BrowserPhaseError(
            f"funded-wallet browser phase '{phase}' could not start its Chromium test tab: {error}{suffix}",
            retryable=phase == "status",
        ) from None


def chromium_result(
    port: int,
    config: dict[str, object],
    timeout: int = RPC_PHASE_TIMEOUT_SECONDS,
) -> tuple[str, dict[str, object]]:
    FundedHandler.config = config
    FundedHandler.result_payload = None
    FundedHandler.result_event.clear()
    chromium = browser_executable()
    phase = str(config.get("phase", "unknown"))
    debug_port = free_port()
    profile_dir = Path(tempfile.mkdtemp(prefix="kaskold-funded-chrome-"))

    # Run a normal real-time browser session. DevTools HTTP is used only to
    # create the test tab; the page itself executes the same Companion WASM and
    # public-node code as before, then POSTs its structured result to the local
    # QA server. No DOM-capture or virtual-time flag participates in completion.
    try:
        with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as browser_log:
            process = start_phase_browser(chromium, debug_port, profile_dir, browser_log, port, phase)
            try:
                deadline = time.monotonic() + timeout
                while True:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        output = browser_log_tail(browser_log)
                        suffix = f"\nLast browser output:\n{output}" if output else ""
                        raise BrowserPhaseError(
                            f"funded-wallet browser phase '{phase}' timed out after {timeout} wall-clock seconds "
                            f"using {chromium}. The wallet/address remains saved locally; this phase can be retried."
                            f"{suffix}",
                            retryable=phase == "status",
                        )
                    if FundedHandler.result_event.wait(timeout=min(0.25, remaining)):
                        break
                    returncode = process.poll()
                    if returncode is not None:
                        output = browser_log_tail(browser_log)
                        raise BrowserPhaseError(
                            f"funded-wallet browser phase '{phase}' exited before returning a structured result "
                            f"(exit {returncode})\n{output}",
                            retryable=phase == "status",
                        )

                payload = FundedHandler.result_payload
                if payload is None:
                    raise BrowserPhaseError(
                        f"funded-wallet browser phase '{phase}' signaled completion without a result payload",
                        retryable=phase == "status",
                    )
                status = str(payload["status"])
                detail = payload["detail"]
                if not isinstance(detail, dict):
                    raise BrowserPhaseError(
                        f"funded-wallet browser phase '{phase}' returned invalid result detail",
                        retryable=phase == "status",
                    )
                if status == "fail":
                    raise BrowserPhaseError(
                        f"funded-wallet browser phase '{phase}' failed\n{json.dumps(detail, indent=2)}",
                        retryable=phase == "status",
                    )
                return status, detail
            finally:
                stop_chromium(process)
    finally:
        cleanup_chromium_profile(profile_dir)


def status_with_retries(port: int, network: str, kpub: str) -> dict[str, object]:
    last_error: BrowserPhaseError | None = None
    for attempt in range(1, STATUS_ATTEMPTS + 1):
        try:
            _status, detail = chromium_result(
                port,
                {"phase": "status", "network": network, "kpub": kpub},
                timeout=STATUS_TIMEOUT_SECONDS,
            )
            return detail
        except BrowserPhaseError as error:
            last_error = error
            if not error.retryable or attempt == STATUS_ATTEMPTS:
                raise
            print(
                f"  Public-node status attempt {attempt}/{STATUS_ATTEMPTS} failed; "
                f"retrying in {STATUS_RETRY_SECONDS}s...",
                file=sys.stderr,
            )
            time.sleep(STATUS_RETRY_SECONDS)
    assert last_error is not None
    raise last_error


def metadata_path(network: str) -> Path:
    return STATE_ROOT / f"{network}.meta.json"


def destination_index(network: str) -> int:
    path = metadata_path(network)
    if not path.is_file():
        return 1
    try:
        current = json.loads(path.read_text())
        previous = int(current.get("destination_index", 0))
    except (OSError, ValueError, TypeError, json.JSONDecodeError):
        return 1
    return 1 if previous >= 19 or previous < 1 else previous + 1


def record_success(network: str, txid: str, destination: str, index: int) -> None:
    path = metadata_path(network)
    previous_runs = 0
    if path.is_file():
        try:
            previous_runs = int(json.loads(path.read_text()).get("successful_runs", 0))
        except (OSError, ValueError, TypeError, json.JSONDecodeError):
            previous_runs = 0
    data = {
        "network": network,
        "successful_runs": previous_runs + 1,
        "destination_index": index,
        "destination": destination,
        "last_txid": txid,
    }
    path.write_text(json.dumps(data, indent=2) + "\n")


def wait_for_resulting_utxo(port: int, config: dict[str, object]) -> dict[str, object]:
    deadline = time.monotonic() + CONFIRMATION_TIMEOUT_SECONDS
    while True:
        status, detail = chromium_result(port, config)
        if status == "pass":
            return detail
        if status != "pending":
            raise SystemExit(f"ERROR: unexpected funded-E2E verification status: {status}")
        if time.monotonic() >= deadline:
            raise SystemExit(
                "ERROR: public node accepted the transaction but the resulting controlled UTXO "
                f"was not observed within {CONFIRMATION_TIMEOUT_SECONDS} seconds; txid={config['txid']}"
            )
        print("  Transaction accepted; waiting for the public node to report the resulting UTXO...")
        time.sleep(POLL_SECONDS)


def print_wallet_address(network_label: str, address: str, existing: bool) -> None:
    print("\n" + "=" * 72)
    print("FUNDED-E2E WALLET")
    print("=" * 72)
    print(f"Network: {network_label}")
    print(f"Funding address: {address}")
    print(
        "Wallet state: existing dedicated local wallet."
        if existing
        else "Wallet state: new dedicated local wallet created outside the repository tree."
    )
    print("Checking public-node funding status automatically.")
    print("=" * 72 + "\n")


def print_funding_prompt(network: str, network_label: str, address: str, existing: bool) -> None:
    faucet = FAUCETS.get(network)
    if faucet is None:
        raise SystemExit(f"ERROR: no funded-E2E faucet is configured for {network}")
    print("\n" + "=" * 72)
    print("TESTNET WALLET REQUIRES FUNDING")
    print("=" * 72)
    print(f"Network: {network_label}")
    print(f"Address: {address}")
    print("Ensure this address has at least 1 test KAS.")
    print(f"Faucet: {faucet}")
    if existing:
        print("Wallet state: existing dedicated local wallet; balance is below the funded-E2E minimum.")
    else:
        print("Wallet state: new dedicated local wallet; balance is below the funded-E2E minimum.")
    print("The automatic public-node check did not find enough funding yet.")
    print("=" * 72 + "\n")


def wait_for_funding_confirmation() -> None:
    while True:
        answer = interactive_input("Funding complete and ready to re-check the public node? [y/q]: ").strip().lower()
        if answer in {"y", "yes"}:
            return
        if answer in {"q", "quit"}:
            raise SystemExit(
                "Funded-E2E stopped while waiting for testnet funding. "
                "The dedicated wallet remains saved locally for a later retry."
            )
        print("Type y after funding the address, or q to quit.")


def query_funding_status(
    port: int, network: str, kpub: str, funding_address: str
) -> dict[str, object]:
    status = status_with_retries(port, network, kpub)
    status_address = str(status.get("funding_address", ""))
    if status_address != funding_address:
        raise BrowserPhaseError(
            "public-node status phase derived a different funding address than the local wallet phase"
        )
    return status


def ensure_funding_ready(
    port: int,
    network: str,
    network_label: str,
    kpub: str,
    funding_address: str,
    *,
    existing: bool,
) -> dict[str, object]:
    """Check funding immediately and only prompt when the reusable balance is absent."""
    status = query_funding_status(port, network, kpub, funding_address)
    if status.get("funded"):
        balance = int(str(status["balance_sompi"]))
        print(
            "Existing funding detected automatically: "
            f"{balance / 100_000_000:.8f} KAS. Starting the funded-wallet E2E without a funding prompt."
        )
        return status

    print_funding_prompt(network, network_label, funding_address, existing=existing)
    while not status.get("funded"):
        wait_for_funding_confirmation()
        status = query_funding_status(port, network, kpub, funding_address)
        if status.get("funded"):
            return status
        balance = int(str(status["balance_sompi"]))
        print(
            "Public node still does not report the required funding: "
            f"{balance / 100_000_000:.8f} KAS. "
            "Fund the address, then confirm again."
        )
    return status


def main() -> int:
    if not interactive_stdin_available():
        print(
            "SKIP: funded testnet E2E requires an interactive input terminal for network/funding confirmation; "
            "run make qa from a maintainer terminal or invoke the funded E2E entrypoint directly."
        )
        return NONINTERACTIVE_SKIP

    network, network_label = select_network()
    build_real_wasm()
    require_real_wasm()
    ensure_tools_lock_current()
    secret_path, kpub, created = ensure_wallet(network)

    with serve(FundedHandler) as port:
        _local_status, local_wallet = chromium_result(
            port,
            {"phase": "address", "network": network, "kpub": kpub},
            timeout=LOCAL_ADDRESS_TIMEOUT_SECONDS,
        )
        funding_address = str(local_wallet.get("funding_address", ""))
        if not funding_address.startswith("kaspatest:"):
            raise BrowserPhaseError("local funded-E2E wallet derivation returned an invalid testnet address")

        print_wallet_address(network_label, funding_address, existing=not created)
        ensure_funding_ready(
            port,
            network,
            network_label,
            kpub,
            funding_address,
            existing=not created,
        )

        print("\nWallet funded; running the automated funded-wallet E2E.")
        index = destination_index(network)
        _status, prepared = chromium_result(
            port,
            {"phase": "prepare", "network": network, "kpub": kpub, "destination_index": index},
        )
        signed_kspt = sign_kspt(secret_path, str(prepared["kspt_wire_hex"]))
        _status, broadcast = chromium_result(
            port,
            {
                "phase": "broadcast",
                "network": network,
                "kpub": kpub,
                "pskb_wire_hex": prepared["pskb_wire_hex"],
                "signed_kspt_hex": signed_kspt,
            },
        )
        txid = str(broadcast["txid"])
        destination = str(prepared["destination"])
        verified = wait_for_resulting_utxo(
            port,
            {
                "phase": "verify",
                "network": network,
                "kpub": kpub,
                "txid": txid,
                "destination": destination,
            },
        )
        record_success(network, txid, destination, index)

    print("\nPASS: funded KasKold transaction E2E completed against a real Kaspa public testnet node")
    print(f"Network: {network_label}")
    print(f"TXID: {txid}")
    print(f"Controlled destination: {destination}")
    print(f"Resulting UTXO: {verified['resulting_utxo_amount_sompi']} sompi")
    print(f"Wallet balance after acceptance: {verified['wallet_balance_sompi']} sompi")
    return 0


def entrypoint() -> int:
    try:
        return main()
    except BrowserPhaseError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(entrypoint())
