from pathlib import Path
import os
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]


class KasKoldSdkArchitectureTests(unittest.TestCase):


    def test_dependency_direction_points_up_from_protocol(self):
        protocol = tomllib.loads((ROOT / 'crates/kaskold-protocol/Cargo.toml').read_text())
        sdk = tomllib.loads((ROOT / 'crates/kaskold-sdk/Cargo.toml').read_text())
        online = tomllib.loads((ROOT / 'crates/online-watcher/Cargo.toml').read_text())
        self.assertNotIn('online-watcher', protocol['dependencies'])
        self.assertIn('kaskold-protocol', sdk['dependencies'])
        self.assertNotIn('online-watcher', sdk['dependencies'])
        self.assertIn('kaskold-protocol', online['dependencies'])
        self.assertFalse((ROOT / 'crates/online-watcher/src/sdk_support').exists())


    @unittest.skipUnless(os.name == "posix", "Linux WASM helper execution is POSIX-specific")
    def test_linux_wasm_helper_passes_crate_type_to_cargo_not_rustc(self):
        helper_source = (ROOT / 'scripts/linux/lib/rust-wasm-sdk.sh').read_text()
        toolchains = (ROOT / 'qa/config/toolchains.env').read_text()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            helper = root / 'scripts/linux/lib/rust-wasm-sdk.sh'
            helper.parent.mkdir(parents=True)
            helper.write_text(helper_source)
            helper.chmod(0o755)
            config = root / 'qa/config/toolchains.env'
            config.parent.mkdir(parents=True)
            config.write_text(toolchains)
            for relative in ('scripts/linux/lib/cargo_locks.sh', 'scripts/linux/lib/rustup_bootstrap.sh'):
                source = ROOT / relative
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(source.read_text())
            crate = root / 'crates/kaskold-sdk'
            crate.mkdir(parents=True)
            (root / 'LICENSE').write_text('GNU General Public License v3.0')

            # The production helper deliberately prepends $HOME/.cargo/bin to PATH.
            # Give this regression an isolated HOME and put the fake rustup there so
            # a developer's real ~/.cargo/bin/rustup can never escape the test sandbox.
            fake_home = root / 'home'
            fake_cargo_bin = fake_home / '.cargo/bin'
            fake_cargo_bin.mkdir(parents=True)
            fake_bin = root / 'fake-bin'
            fake_bin.mkdir()
            rustup = fake_cargo_bin / 'rustup'
            rustup.write_text('''#!/usr/bin/env bash
set -Eeuo pipefail
printf '%s\n' "$*" >> "${FAKE_RUSTUP_LOG}"
if [[ "${1:-}" == target && "${2:-}" == list ]]; then
    printf '%s\n' wasm32-unknown-unknown
    exit 0
fi
if [[ "${1:-}" == run && " $* " == *" cargo metadata "* ]]; then
    exit 0
fi
if [[ "${1:-}" == run ]]; then
    [[ " $* " == *" --crate-type=cdylib "* ]] || exit 41
    [[ " $* " != *" -- --crate-type=cdylib "* ]] || exit 42
    mkdir -p "${CARGO_TARGET_DIR}/wasm32-unknown-unknown/release"
    printf wasm > "${CARGO_TARGET_DIR}/wasm32-unknown-unknown/release/kaskold_sdk.wasm"
    exit 0
fi
exit 43
''')
            rustup.chmod(0o755)

            cache = root / 'tool-cache'
            wasm_bindgen = cache / 'wasm-bindgen-cli-0.2.120/bin/wasm-bindgen'
            wasm_bindgen.parent.mkdir(parents=True)
            wasm_bindgen.write_text('''#!/usr/bin/env bash
set -Eeuo pipefail
if [[ "${1:-}" == --version ]]; then
    printf '%s\n' 'wasm-bindgen 0.2.120'
    exit 0
fi
out_dir=''
out_name=''
while [[ $# -gt 0 ]]; do
    case "$1" in
        --out-dir) out_dir="$2"; shift 2 ;;
        --out-name) out_name="$2"; shift 2 ;;
        *) shift ;;
    esac
done
mkdir -p "${out_dir}"
printf js > "${out_dir}/${out_name}.js"
printf dts > "${out_dir}/${out_name}.d.ts"
printf wasm > "${out_dir}/${out_name}_bg.wasm"
printf dts > "${out_dir}/${out_name}_bg.wasm.d.ts"
''')
            wasm_bindgen.chmod(0o755)

            output = root / 'sdk-output/pkg'
            log = root / 'rustup.log'
            env = os.environ.copy()
            env.update({
                'HOME': str(fake_home),
                'PATH': f'{fake_cargo_bin}:{fake_bin}:{env.get("PATH", "")}',
                'KASKOLD_TOOL_CACHE_DIR': str(cache),
                'FAKE_RUSTUP_LOG': str(log),
            })
            subprocess.run(
                [str(helper), 'kaskold-sdk', 'kaskold_sdk', str(output), 'SDK test', '@kaskold/sdk'],
                check=True,
                cwd=root,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            invocation = log.read_text()
            self.assertIn('cargo metadata', invocation)
            self.assertLess(invocation.index('cargo metadata'), invocation.index('cargo rustc'))
            self.assertIn('cargo rustc', invocation)
            self.assertIn('--crate-type=cdylib', invocation)
            self.assertNotIn('-- --crate-type=cdylib', invocation)
            self.assertTrue((output / 'kaskold_sdk_bg.wasm').is_file())


if __name__ == '__main__':
    unittest.main()
