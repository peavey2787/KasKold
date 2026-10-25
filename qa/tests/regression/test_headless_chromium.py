from concurrent.futures import ThreadPoolExecutor
import errno
import http.client
import importlib.util
import io
from pathlib import Path
import shutil
import tempfile
import unittest
import urllib.request
from contextlib import redirect_stderr
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]


def load_harness():
    path = ROOT / "qa/checks/integration/headless_chromium.py"
    spec = importlib.util.spec_from_file_location("kaskold_headless_chromium", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeProcess:
    def __init__(self, pid=4242, exits_on_poll=None):
        self.pid = pid
        self.returncode = exits_on_poll
        self.killed = False

    def poll(self):
        return self.returncode

    def wait(self, timeout=None):
        self.returncode = 0
        return 0

    def kill(self):
        self.killed = True
        self.returncode = -9


class BrowserDiscoveryTests(unittest.TestCase):
    def test_windows_edge_then_chrome_install_paths_are_found_without_path_entries(self):
        harness = load_harness()
        with tempfile.TemporaryDirectory(prefix="headless-chromium-discovery-") as tmp:
            program_files = Path(tmp) / "Program Files"
            local_app_data = Path(tmp) / "LocalAppData"
            edge = program_files / "Microsoft/Edge/Application/msedge.exe"
            chrome = local_app_data / "Google/Chrome/Application/chrome.exe"
            for browser in (edge, chrome):
                browser.parent.mkdir(parents=True)
                browser.write_bytes(b"MZ")
            env = {"PROGRAMFILES": str(program_files), "LOCALAPPDATA": str(local_app_data)}
            with patch.object(harness.shutil, "which", return_value=None):
                self.assertEqual(harness.browser_executable(platform_name="nt", environ=env), str(edge))
                edge.unlink()
                self.assertEqual(harness.browser_executable(platform_name="nt", environ=env), str(chrome))

    def test_path_browser_is_preferred(self):
        harness = load_harness()
        which = lambda name: "C:/Browser/msedge.exe" if name == "msedge.exe" else None  # noqa: E731
        with patch.object(harness.shutil, "which", side_effect=which):
            self.assertEqual(harness.browser_executable(), "C:/Browser/msedge.exe")

    def test_missing_browser_fails_with_a_named_requirement(self):
        harness = load_harness()
        with patch.object(harness.shutil, "which", return_value=None):
            with self.assertRaises(SystemExit) as raised:
                harness.browser_executable(platform_name="posix", environ={})
        self.assertIn("Chromium-family browser not found", str(raised.exception))


class ChromiumCommandTests(unittest.TestCase):
    def test_command_is_real_time_loopback_devtools_on_an_isolated_profile(self):
        harness = load_harness()
        command = harness.chromium_command("/usr/bin/chromium", 9222, Path("/tmp/profile"))
        self.assertEqual(command[0], "/usr/bin/chromium")
        self.assertIn("--headless=new", command)
        self.assertIn("--remote-debugging-address=127.0.0.1", command)
        self.assertIn("--remote-debugging-port=9222", command)
        self.assertIn(f"--user-data-dir={Path('/tmp/profile')}", command)
        self.assertFalse(any(argument.startswith("--virtual-time-budget") for argument in command))
        self.assertNotIn("--dump-dom", command)

    def test_browser_environment_drops_stale_dbus_addresses(self):
        harness = load_harness()
        environment = {"DBUS_SESSION_BUS_ADDRESS": "unix:path=/gone", "PATH": "/bin"}
        with patch.dict(harness.os.environ, environment, clear=True):
            self.assertEqual(harness.browser_environment(), {"PATH": "/bin"})


class DevToolsTargetTests(unittest.TestCase):
    def test_target_creation_is_a_loopback_put_with_an_encoded_url(self):
        harness = load_harness()

        class FakeResponse:
            status = 200

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def read(self):
                return b'{"id":"target-1"}'

        with patch.object(harness.urllib.request, "urlopen", return_value=FakeResponse()) as urlopen:
            harness.open_chromium_target(9222, "http://127.0.0.1:12345/__qa_page__")
        request = urlopen.call_args.args[0]
        self.assertEqual(request.get_method(), "PUT")
        self.assertTrue(request.full_url.startswith("http://127.0.0.1:9222/json/new?"))
        self.assertIn("http%3A%2F%2F127.0.0.1%3A12345%2F__qa_page__", request.full_url)


class LaunchTests(unittest.TestCase):
    def test_relaunch_uses_a_fresh_profile_so_a_stale_lock_cannot_block_it(self):
        harness = load_harness()
        processes = [FakeProcess(pid=1), FakeProcess(pid=2)]
        ready = [harness.ChromiumError("Chromium exited before DevTools became ready (exit 21)"), None]
        profile = Path(tempfile.gettempdir()) / "headless-chromium-launch"
        with patch.object(harness.subprocess, "Popen", side_effect=processes) as popen, patch.object(
            harness, "wait_for_chromium_debugger", side_effect=ready
        ), patch.object(harness, "stop_chromium") as stop, tempfile.TemporaryFile(mode="w+") as log:
            process = harness.launch_chromium("chromium", 9222, profile, log)
        self.assertIs(process, processes[1])
        stop.assert_called_once_with(processes[0])
        profiles = [
            next(argument for argument in call.args[0] if argument.startswith("--user-data-dir="))
            for call in popen.call_args_list
        ]
        self.assertEqual(
            profiles,
            [f"--user-data-dir={profile / 'attempt-1'}", f"--user-data-dir={profile / 'attempt-2'}"],
        )

    def test_second_failed_start_reports_the_browser_log(self):
        harness = load_harness()
        error = harness.ChromiumError("Chromium DevTools did not become ready within 30s")
        with patch.object(harness.subprocess, "Popen", side_effect=[FakeProcess(), FakeProcess()]), patch.object(
            harness, "wait_for_chromium_debugger", side_effect=[error, error]
        ), patch.object(harness, "stop_chromium"), tempfile.TemporaryFile(mode="w+") as log:
            log.write("browser said no")
            with self.assertRaises(harness.ChromiumError) as raised:
                harness.launch_chromium("chromium", 9222, Path("profile"), log)
        self.assertIn("did not become ready", str(raised.exception))
        self.assertIn("browser said no", str(raised.exception))


class StopTests(unittest.TestCase):
    def test_windows_stop_kills_the_whole_process_tree(self):
        harness = load_harness()
        process = FakeProcess(pid=777)
        with patch.object(harness.os, "name", "nt"), patch.object(harness.subprocess, "run") as run:
            harness.stop_chromium(process)
        run.assert_called_once()
        self.assertEqual(run.call_args.args[0], ["taskkill", "/PID", "777", "/T", "/F"])
        self.assertTrue(process.killed)

    def test_windows_stop_after_exit_does_not_kill_anything(self):
        harness = load_harness()
        process = FakeProcess(exits_on_poll=0)
        with patch.object(harness.os, "name", "nt"), patch.object(harness.subprocess, "run") as run:
            harness.stop_chromium(process)
        run.assert_not_called()
        self.assertFalse(process.killed)

    @unittest.skipUnless(hasattr(__import__("os"), "killpg"), "POSIX process groups")
    def test_posix_stop_terminates_then_reaps_the_process_group(self):
        harness = load_harness()
        process = FakeProcess(pid=555)
        with patch.object(harness.os, "killpg") as killpg:
            harness.stop_chromium(process)
        self.assertEqual(
            [call.args for call in killpg.call_args_list],
            [(555, harness.signal.SIGTERM), (555, harness.signal.SIGKILL)],
        )


class ProfileCleanupTests(unittest.TestCase):
    def test_cleanup_retries_a_directory_not_empty_race(self):
        harness = load_harness()
        profile = Path(tempfile.mkdtemp(prefix="headless-chromium-cleanup-"))
        (profile / "Default").mkdir()
        (profile / "Default" / "Preferences").write_text("{}")
        real_rmtree = shutil.rmtree
        calls = 0

        def flaky_rmtree(path):
            nonlocal calls
            calls += 1
            if calls == 1:
                raise OSError(errno.ENOTEMPTY, "Directory not empty", str(Path(path) / "Default"))
            real_rmtree(path)

        with patch.object(harness.shutil, "rmtree", side_effect=flaky_rmtree), patch.object(
            harness.time, "sleep"
        ) as sleep:
            harness.cleanup_chromium_profile(profile, attempts=3, delay_seconds=0.01)
        self.assertEqual(calls, 2)
        sleep.assert_called_once_with(0.01)
        self.assertFalse(profile.exists())

    def test_cleanup_failure_warns_and_never_overwrites_the_gate_result(self):
        harness = load_harness()
        profile = Path(tempfile.mkdtemp(prefix="headless-chromium-cleanup-"))
        error = OSError(errno.ENOTEMPTY, "Directory not empty", str(profile / "Default"))
        stderr = io.StringIO()
        real_rmtree = shutil.rmtree
        try:
            with patch.object(harness.shutil, "rmtree", side_effect=error), patch.object(harness.time, "sleep"):
                with redirect_stderr(stderr):
                    harness.cleanup_chromium_profile(profile, attempts=2, delay_seconds=0.01)
            self.assertIn("WARNING: could not fully remove temporary Chromium profile", stderr.getvalue())
            self.assertTrue(profile.exists())
        finally:
            real_rmtree(profile, ignore_errors=True)


class LoopbackServerTests(unittest.TestCase):
    def result_handler(self, harness):
        class ResultHandler(harness.LoopbackRequestHandler):
            received = []

            def do_POST(self):
                result = self.read_result(frozenset({"pass", "fail"}), 64)
                if result is not None:
                    type(self).received.append(result)
                    self.send_no_content()

        return ResultHandler

    def post(self, port, body, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
        try:
            connection.request("POST", "/result", body=body, headers=headers or {})
            response = connection.getresponse()
            response.read()
            return response.status
        finally:
            connection.close()

    def test_module_requests_share_one_keep_alive_connection(self):
        harness = load_harness()
        with harness.serve(harness.LoopbackRequestHandler) as port:
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
            try:
                peers = []
                for path in ("/qa/checks/integration/headless_chromium.py", "/qa/checks/web/real_dom/harness.mjs"):
                    connection.request("GET", path)
                    response = connection.getresponse()
                    response.read()
                    self.assertEqual(response.status, 200)
                    peers.append(connection.sock.getsockname())
                self.assertEqual(response.headers.get_content_type(), "text/javascript")
                self.assertEqual(peers[0], peers[1])
            finally:
                connection.close()

    def test_favicon_is_no_content(self):
        harness = load_harness()
        with harness.serve(harness.LoopbackRequestHandler) as port:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/favicon.ico", timeout=10) as response:
                self.assertEqual(response.status, 204)

    def test_result_post_accepts_only_bounded_status_and_detail_objects(self):
        harness = load_harness()
        handler = self.result_handler(harness)
        with harness.serve(handler) as port:
            self.assertEqual(self.post(port, b'{"status":"pass","detail":{"ok":1}}'), 204)
            self.assertEqual(self.post(port, b'{"status":"maybe","detail":{}}'), 400)
            self.assertEqual(self.post(port, b'{"status":"fail","detail":[]}'), 400)
            self.assertEqual(self.post(port, b"not json"), 400)
            self.assertEqual(self.post(port, b"[" + b" " * 70 + b"]"), 413)
            self.assertEqual(self.post(port, b"{}", {"Content-Length": "x"}), 413)
        self.assertEqual(handler.received, [{"status": "pass", "detail": {"ok": 1}}])

    def test_server_answers_a_parallel_module_graph_burst(self):
        harness = load_harness()

        class QuietHandler(harness.LoopbackRequestHandler):
            protocol_version = "HTTP/1.0"

        def fetch(port):
            url = f"http://127.0.0.1:{port}/qa/checks/integration/headless_chromium.py"
            with urllib.request.urlopen(url, timeout=10) as response:
                response.read()
                return response.status

        with harness.serve(QuietHandler) as port:
            with ThreadPoolExecutor(max_workers=96) as pool:
                statuses = list(pool.map(lambda _index: fetch(port), range(96)))
        self.assertEqual(statuses, [200] * 96)


if __name__ == "__main__":
    unittest.main()
