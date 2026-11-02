import fs from 'node:fs/promises';
import { registerHooks } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const GENERATED_MODULE = 'pkg/companion_web.js';

function packageMissing(specifier) {
    return Object.assign(new Error(`QA has not published a WASM package for ${specifier}`), {
        code: 'ERR_MODULE_NOT_FOUND',
    });
}

/**
 * Redirect this process's imports of the generated WASM package to a QA stub.
 *
 * Browser checks need deterministic WASM exports, while the real package may
 * already be built or in use by another check. Nothing under web/pkg is read or
 * written: until publish() the package is reported missing, as on a fresh clone,
 * and afterwards the stub lives in a private temporary directory.
 */
export async function stubWebPackage() {
    const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'kaskold-web-pkg-'));
    const stub = path.join(directory, 'companion_web.js');
    let published = false;
    let revision = 0;
    const hooks = registerHooks({
        resolve(specifier, context, next) {
            if (!specifier.endsWith(GENERATED_MODULE)) return next(specifier, context);
            if (!published) throw packageMissing(specifier);
            return { url: `${pathToFileURL(stub).href}?revision=${revision}`, shortCircuit: true };
        },
    });
    return {
        async publish(source) {
            await fs.writeFile(stub, source);
            published = true;
            revision += 1;
        },
        async restore() {
            hooks.deregister();
            await fs.rm(directory, { recursive: true, force: true });
        },
    };
}
