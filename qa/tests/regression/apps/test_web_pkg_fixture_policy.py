#!/usr/bin/env python3
"""Keep browser QA hermetic without touching a previously built WASM package."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[4]
HELPER = ROOT / "qa/checks/web/support/web_pkg_fixture.mjs"


class WebPackageFixturePolicyTests(unittest.TestCase):

    def test_stub_redirects_imports_and_never_touches_the_package_directory(self) -> None:
        node = shutil.which("node")
        if node is None:
            self.fail("node is required by the browser QA fixture")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "web" / "pkg"
            package.mkdir(parents=True)
            sentinel = b"export const origin = 'real generated package';\n"
            (package / "companion_web.js").write_bytes(sentinel)

            program = """
                import assert from 'node:assert/strict';
                import fs from 'node:fs';
                import path from 'node:path';
                import { pathToFileURL } from 'node:url';
                const { stubWebPackage } = await import(process.argv[1]);
                const generated = pathToFileURL(path.join(process.argv[2], 'companion_web.js')).href;

                const fixture = await stubWebPackage();
                await assert.rejects(import(generated), { code: 'ERR_MODULE_NOT_FOUND' });
                await fixture.publish("export const origin = 'qa stub';");
                assert.equal((await import(generated)).origin, 'qa stub');
                await fixture.publish("export const origin = 'second stub';");
                assert.equal((await import(generated)).origin, 'second stub');
                assert.deepEqual(fs.readdirSync(process.argv[2]), ['companion_web.js']);
                await fixture.restore();
                assert.equal((await import(generated)).origin, 'real generated package');
            """
            subprocess.run(
                [
                    node,
                    "--input-type=module",
                    "--eval",
                    program,
                    HELPER.resolve().as_uri(),
                    str(package),
                ],
                cwd=ROOT,
                check=True,
                text=True,
                capture_output=True,
            )

            self.assertEqual((package / "companion_web.js").read_bytes(), sentinel)


if __name__ == "__main__":
    unittest.main()
