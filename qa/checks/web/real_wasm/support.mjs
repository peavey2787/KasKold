// The real Companion WASM package, as `make companion` builds it, for the tests
// that hold the JavaScript stubs and parsers to what the Rust code really does.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const SITE = process.env.KASKOLD_KASKOLD_COMPANION_WEB_SITE
    ?? path.join(ROOT, 'target', 'kaskold-companion-web', 'site');
const PACKAGE = path.join(SITE, 'pkg');
const GLUE = path.join(PACKAGE, 'companion_web.js');

if (!fs.existsSync(GLUE)) {
    throw new Error(`The real WASM package is missing at ${PACKAGE}: run the preflight.companion-build step first`);
}

export const unrecorded = await import(pathToFileURL(GLUE).href);
unrecorded.initSync({ module: fs.readFileSync(path.join(PACKAGE, 'companion_web_bg.wasm')) });

/** wasm-bindgen's own glue, not part of the contract the application calls. */
const BINDGEN_GLUE = new Set(['default', 'initSync', 'initialize_application']);

const SIGNATURE = /^export function (\w+)\(([^)]*)\): ([^;]+);$/gm;

function parameterType(parameter) {
    const type = parameter.split(': ')[1];
    return type.startsWith('string') ? 'string' : type;
}

/** Every application export with its parameter types, from the generated declarations. */
export function exportSignatures() {
    const declarations = fs.readFileSync(path.join(PACKAGE, 'companion_web.d.ts'), 'utf8');
    const signatures = new Map();
    for (const [, name, parameters, returned] of declarations.matchAll(SIGNATURE)) {
        if (BINDGEN_GLUE.has(name)) continue;
        signatures.set(name, {
            parameters: parameters.split(',').map(part => part.trim()).filter(Boolean).map(parameterType),
            isAsync: returned.startsWith('Promise'),
        });
    }
    return signatures;
}

export const called = new Set();

/** The real package; each export it serves is recorded so no export goes unexercised. */
export const real = new Proxy(unrecorded, {
    get(target, name) {
        if (typeof name === 'string') called.add(name);
        return target[name];
    },
});

export const webModule = relative => pathToFileURL(
    path.join(ROOT, 'apps', 'kaskold-companion-web', 'web', 'js', relative),
).href;

// Worked out away from Rust by a stdlib-only BIP39, BIP32 and Kaspa address
// derivation of "abandon x11 about" at m/44'/111111'/0', so the package cannot
// agree with itself. The firmware journeys hold the device to the same answers.
export const ACCOUNT_KEY = 'kpub1:038f332e0375ccff598000000086cc73fd0442c8d1154586697464ba0658599ff1d0471c6e18e1f5b265e33f05035300b5f129edeca4679d91de11bb2f49d59555bd5ec79ffc778738ed6da4a894';
export const ACCOUNT_PUBKEY = '5300b5f129edeca4679d91de11bb2f49d59555bd5ec79ffc778738ed6da4a894';
export const RECEIVE_0 = 'kaspa:qqd6e65yefepe9wk0m9vuxdufxd80sphy67gwwd0vdaumzdt4tc9s3qt0lqeh';
export const RECEIVE_1 = 'kaspa:qp6r0d88yj4fazlj057wc35245jfgs87n9jn6nahfg223996dfukvgpgq6pcp';
export const CHANGE_0 = 'kaspa:qqnkyg4hha30lnjystpugz42yh3paurdylxfrsshk3jfstgy95elu2a8l3jcs';
export const RECEIVE_0_PAYLOAD = '1bacea84ca721c95d67ecace19bc499a77c03726bc8739af637bcd89abaaf058';
/** secp256k1 multiples of the generator: 1G, 2G, 3G x coordinates. */
export const KEY_1G = '79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798';
export const KEY_2G = 'c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5';
export const KEY_3G = 'f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9';
export const KEY_1G_ADDRESS = 'kaspa:qpumuen7l8wthtz45p3ftn58pvrs9xlumvkuu2xet8egzkcklqtes4ypce9sf';

export function explicitUtxo(overrides = {}) {
    return {
        tx_id: '11'.repeat(32),
        index: 0,
        amount: '50000000',
        script_public_key: `20${RECEIVE_0_PAYLOAD}ac`,
        block_daa_score: '0',
        ...overrides,
    };
}

export function spendFromReceive0() {
    return real.create_send_pskb_with_utxos(
        real.import_kpub(ACCOUNT_KEY, 'mainnet'),
        KEY_1G_ADDRESS,
        20_000_000n,
        300_000n,
        JSON.stringify([explicitUtxo()]),
    );
}
