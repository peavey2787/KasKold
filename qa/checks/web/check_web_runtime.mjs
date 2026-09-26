#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isolateWebPackage } from './web_pkg_fixture.mjs';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, '..', '..', '..');
const webRoot = path.join(root, 'apps', 'kaskold-companion-web', 'web');
const jsRoot = path.join(webRoot, 'js');
const pkgDir = path.join(webRoot, 'pkg');
const generatedModule = path.join(pkgDir, 'companion_web.js');

function parseWasmExports(source) {
    const match = source.match(/const\s+GENERATED_WASM_EXPORTS\s*=\s*Object\.freeze\(\[([\s\S]*?)\]\);/);
    if (!match) throw new Error('Unable to parse wasm/api.js export inventory');
    return ['init', ...[...match[1].matchAll(/['"]([A-Za-z_][A-Za-z0-9_]*)['"]/g)].map(item => item[1])];
}

function buildWasmStub(names) {
    return names.map(name => {
        if (name === 'init') {
            return `export default async function init() {
                if (globalThis.__COMPANION_TEST_INIT_FAILURE__) throw new Error('intentional WASM init failure');
            }`;
        }
        if (name === 'version') return "export function version() { return 'web-runtime-smoke'; }";
        if (name === 'import_kpub') {
            return `export function import_kpub(kpub) {
                return JSON.stringify({
                    kpub,
                    receive_addresses: ['kaspa:runtime-receive'],
                    change_addresses: ['kaspa:runtime-change'],
                    next_receive_index: 0,
                    next_change_index: 0,
                });
            }`;
        }
        if (name === 'import_kpub_raw') {
            return `export function import_kpub_raw() {
                return JSON.stringify({
                    kpub: 'kpub1:${'22'.repeat(78)}',
                    receive_addresses: ['kaspa:runtime-receive'],
                    change_addresses: ['kaspa:runtime-change'],
                    next_receive_index: 0,
                    next_change_index: 0,
                });
            }`;
        }
        if (name === 'parse_kpub') {
            return "export function parse_kpub() { return JSON.stringify({ account_pubkey: '02' + '11'.repeat(32) }); }";
        }
        if (name === 'scan_multisig_branch_js') {
            return `export async function scan_multisig_branch_js(requestJson) {
                const request = JSON.parse(requestJson);
                const prefix = request.address_prefix || 'kaspa';
                globalThis.__COMPANION_TEST_MS_SCAN_REQUEST__ = request;
                return JSON.stringify({
                    balance_sompi: '0',
                    utxo_count: 0,
                    utxos: [],
                    next_receive_index: 0,
                    next_receive_address: prefix + ':runtime-ms-receive',
                    next_change_index: 0,
                    next_change_address: prefix + ':runtime-ms-change',
                    cosigner_index: request.cosigner_index,
                    depth: 40,
                });
            }`;
        }
        return `export function ${name}(...args) { void args; return ''; }`;
    }).join('\n') + '\n';
}

function createClassList(initial = []) {
    const classes = new Set(initial);
    return {
        add(...names) { names.forEach(name => classes.add(name)); },
        remove(...names) { names.forEach(name => classes.delete(name)); },
        toggle(name, force) {
            if (force === true) classes.add(name);
            else if (force === false) classes.delete(name);
            else if (classes.has(name)) classes.delete(name);
            else classes.add(name);
        },
        contains(name) { return classes.has(name); },
    };
}

function createElement(id = '', classes = []) {
    const element = {
        id,
        classList: createClassList(classes),
        style: {}, dataset: {}, value: '', textContent: '', innerHTML: '', checked: false,
        files: [], onclick: null, onchange: null, onkeydown: null,
        parentElement: null, previousElementSibling: null,
        querySelectorAll() { return []; }, querySelector() { return null; }, closest() { return null; },
        addEventListener() {}, appendChild() {}, replaceChildren() {}, insertBefore() {}, remove() {}, focus() {}, play() {},
        click() { if (typeof this.onclick === 'function') this.onclick({ target: this }); },
        getAttribute() { return null; }, setAttribute() {},
        getContext() {
            return {
                drawImage() {},
                getImageData() { return { data: new Uint8ClampedArray(4), width: 1, height: 1 }; },
            };
        },
    };
    return new Proxy(element, {
        get(target, property) { return property in target ? target[property] : undefined; },
        set(target, property, value) { target[property] = value; return true; },
    });
}

function createStorage() {
    const values = new Map();
    return {
        get length() { return values.size; },
        getItem(key) { return values.has(key) ? values.get(key) : null; },
        setItem(key, value) { values.set(key, String(value)); },
        removeItem(key) { values.delete(key); },
        clear() { values.clear(); },
        key(index) { return [...values.keys()][index] ?? null; },
    };
}

async function installBrowserStubs() {
    const html = await fs.readFile(path.join(webRoot, 'index.html'), 'utf8');
    const elements = new Map();
    for (const match of html.matchAll(/<([A-Za-z][A-Za-z0-9:-]*)\b([^>]*?\bid="([^"]+)"[^>]*)>/g)) {
        const attributes = match[2];
        const id = match[3];
        const classMatch = attributes.match(/\bclass="([^"]+)"/);
        const classes = classMatch ? classMatch[1].split(/\s+/).filter(Boolean) : [];
        elements.set(id, createElement(id, classes));
    }

    globalThis.window = globalThis;
    const windowListeners = new Map();
    globalThis.addEventListener = (name, handler) => windowListeners.set(name, handler);
    const statusDot = createElement('', ['dot', 'connecting']);
    const statusLabel = createElement();
    statusLabel.textContent = 'Checking';
    globalThis.document = {
        getElementById(id) { return elements.get(id) || null; },
        querySelector(selector) {
            if (selector === '#status-dot .dot') return statusDot;
            if (selector === '#status-dot .label') return statusLabel;
            if (selector === '.screen.active') {
                return [...elements.values()].find(element =>
                    element.id.startsWith('screen-') && element.classList.contains('active')) || null;
            }
            if (selector === 'main') return elements.get('main') || createElement('main');
            return null;
        },
        querySelectorAll(selector) {
            if (selector === '.screen') {
                return [...elements.values()].filter(element => element.id.startsWith('screen-'));
            }
            if (selector === '.gear-tab') {
                return [...elements.values()].filter(element => element.id.startsWith('gear-tab-'));
            }
            return [];
        },
        addEventListener() {},
        createElement() { return createElement(); },
        body: createElement(),
    };
    for (const target of ['kpub-manager', 'addresses', 'utxos', 'tokens', 'history', 'settings']) {
        const tab = elements.get(`gear-tab-${target}`);
        if (tab) tab.dataset.target = target;
    }
    globalThis.localStorage = createStorage();
    globalThis.sessionStorage = createStorage();
    globalThis.__COMPANION_TEST_COPIED__ = null;
    Object.defineProperty(globalThis, 'navigator', {
        configurable: true,
        value: {
            clipboard: { async writeText(text) { globalThis.__COMPANION_TEST_COPIED__ = text; } },
            mediaDevices: { async getUserMedia() { return { getTracks: () => [] }; } },
            onLine: true,
        },
    });
    globalThis.__COMPANION_TEST_RELOADS__ = 0;
    globalThis.location = { href: 'http://localhost/', reload() { globalThis.__COMPANION_TEST_RELOADS__ += 1; } };
    globalThis.fetch = async () => ({ ok: false, status: 404, async json() { return {}; }, async text() { return ''; } });
    globalThis.setInterval = () => 1;
    globalThis.clearInterval = () => {};
    globalThis.setTimeout = () => 1;
    globalThis.clearTimeout = () => {};
    globalThis.requestAnimationFrame = () => 1;
    globalThis.cancelAnimationFrame = () => {};
    globalThis.QRCode = function QRCode() {};
    globalThis.jsQR = () => null;
    globalThis.alert = () => {};
    globalThis.confirm = () => false;
    globalThis.prompt = () => null;
    globalThis.Image = class Image { async decode() {} };

    elements.set('__status-dot', statusDot);
    elements.set('__status-label', statusLabel);
    return elements;
}

const pkgFixture = await isolateWebPackage(pkgDir);
try {
    const apiSource = await fs.readFile(path.join(jsRoot, 'wasm', 'api.js'), 'utf8');
    const wasmExports = parseWasmExports(apiSource);

    const elements = await installBrowserStubs();
    const globalsBeforeStartup = new Set(Reflect.ownKeys(globalThis));
    const startupErrors = [];
    const originalError = console.error;
    console.error = (...args) => startupErrors.push(args.map(String).join(' '));

    const { bindShellControls } = await import(pathToFileURL(path.join(jsRoot, 'app', 'shell_controls.js')).href);
    bindShellControls();
    assert.equal(elements.get('__status-label').textContent, 'Online',
        'active browser connectivity must not be labeled Offline');
    assert.equal(elements.get('__status-dot').className, 'dot online',
        'active browser connectivity must use the online indicator');
    assert.equal(elements.has('gear-menu'), false,
        'the settings cog must not expose a secondary menu');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'settings cog must open Node Connection directly before the application module graph loads');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'pressing the settings cog again must close Node Connection in shell mode');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'settings cog must reopen Node Connection after toggling it closed in shell mode');
    elements.get('btn-logo').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'logo must navigate Home before the application module graph loads');
    elements.get('btn-scan-kpub').click();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(elements.get('screen-kpub-manager').classList.contains('active'), true,
        'Load kpub must open centralized kpub management before the application module graph loads');
    assert.equal(elements.get('kpub-import-form').classList.contains('hidden'), false,
        'Load kpub must reveal the managed camera, image, and text import form');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'Node Connection must open directly from Wallet Management in shell mode');
    elements.get('btn-settings-back').click();
    assert.equal(elements.get('screen-kpub-manager').classList.contains('active'), true,
        'settings Back must return to Wallet Management in shell mode');
    elements.get('btn-kpub-manager-back').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'Wallet Management Back must return to Welcome in shell mode');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'Node Connection must open directly from Welcome in shell mode');
    elements.get('btn-settings-back').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'settings Back must return to Welcome in shell mode');

    const { startApplication } = await import(pathToFileURL(path.join(jsRoot, 'app', 'bootstrap.js')).href);
    await startApplication();
    // Exercise the real stable entry module under V8 coverage as well. The entry
    // intentionally owns only shell binding + bootstrap dispatch, so importing it
    // here proves every reachable js/ module has an explicit trace record without
    // fabricating synthetic zero-coverage entries.
    await import(`${pathToFileURL(path.join(jsRoot, 'main.js')).href}?runtime-coverage-entry=1`);
    await new Promise(resolve => setImmediate(resolve));
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'full application settings cog must open Node Connection directly');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'pressing the settings cog again must close Node Connection after application startup');
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'settings cog must reopen Node Connection after toggling it closed');
    elements.get('btn-logo').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'full application logo handler must navigate Home');

    assert.equal(typeof elements.get('btn-scan-kpub').onclick, 'function',
        'the centralized Load kpub entry point must bind when the generated WASM package is missing');
    assert.equal(typeof elements.get('btn-open-kpub-import').onclick, 'function',
        'the kpub manager Load kpub button must be wired');
    assert.equal(typeof elements.get('btn-scan-managed-kpub').onclick, 'function',
        'managed camera QR import must be wired');
    assert.equal(typeof elements.get('btn-upload-managed-kpub').onclick, 'function',
        'managed QR image import must be wired');
    assert.equal(typeof elements.get('input-managed-kpub-image').onchange, 'function',
        'managed QR image file selection must be wired');
    assert.equal(typeof elements.get('input-kpub-friendly-name').onkeydown, 'function',
        'managed kpub naming must support Enter');
    assert.equal(typeof elements.get('btn-load-signed-qr-image').onclick, 'function',
        'signed transaction QR image import button must be wired');
    assert.equal(typeof elements.get('input-signed-qr-image').onchange, 'function',
        'signed transaction QR image file selection must be wired');
    assert.equal(typeof elements.get('btn-header-settings').onclick, 'function',
        'settings cog must remain wired after full application startup');
    assert.equal(typeof elements.get('btn-save-managed-kpub').onclick, 'function',
        'saved kpub creation must be wired after full application startup');
    assert.equal(typeof elements.get('btn-use-current-kpub').onclick, 'function',
        'current-wallet kpub capture must be wired after full application startup');
    elements.get('btn-scan-kpub').click();
    assert.equal(elements.get('screen-kpub-manager').classList.contains('active'), true,
        'Load kpub must open centralized kpub management');
    assert.equal(elements.get('kpub-import-form').classList.contains('hidden'), false,
        'centralized kpub management must reveal all import methods');
    elements.get('btn-kpub-manager-back-top').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'Wallet Management Back must return to Welcome after application startup');
    elements.get('btn-scan-kpub').click();
    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'Node Connection must open directly from centralized Wallet Management');
    elements.get('btn-settings-back-top').click();
    assert.equal(elements.get('screen-kpub-manager').classList.contains('active'), true,
        'settings Back must return to centralized kpub management');
    assert.equal(elements.get('companion-startup-status').dataset.state, 'error');
    assert.match(elements.get('companion-startup-status').textContent, /controls are available/i,
        'missing WASM must leave controls active and explain the degraded state');

    elements.get('btn-multisig-welcome').click();
    assert.equal(elements.get('screen-multisig-spend').classList.contains('active'), true,
        'non-WASM welcome navigation must keep the direct pre-wallet multisig spend path usable');

    elements.get('input-managed-kpub').value = `kpub1:${'11'.repeat(78)}`;
    elements.get('input-kpub-friendly-name').value = 'Runtime wallet';
    elements.get('btn-save-managed-kpub').click();
    assert.match(elements.get('toast').textContent, /WebAssembly is unavailable/i,
        'WASM-dependent controls must report why the action cannot complete');

    await pkgFixture.create();
    await fs.writeFile(generatedModule, buildWasmStub(wasmExports));

    globalThis.__COMPANION_TEST_INIT_FAILURE__ = true;
    await startApplication();
    assert.equal(elements.get('companion-startup-status').dataset.state, 'error');
    assert.match(elements.get('companion-startup-status').textContent, /controls are available/i,
        'a failed WASM initializer must also preserve controls');

    globalThis.__COMPANION_TEST_INIT_FAILURE__ = false;
    startupErrors.length = 0;
    globalThis.fetch = async url => {
        if (String(url).includes('/v2/kaspa/')) {
            return { ok: true, status: 200, async json() { return { url: 'ws://runtime-node' }; }, async text() { return ''; } };
        }
        return { ok: false, status: 404, async json() { return {}; }, async text() { return ''; } };
    };
    await startApplication();
    assert.equal(elements.get('companion-startup-status').dataset.state, 'ready');
    assert.match(elements.get('companion-startup-status').textContent, /No saved wallets/i);
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'successful startup without an automatic kpub must preserve the first landing screen');
    assert.equal(elements.get('welcome-saved-kpubs').classList.contains('hidden'), true,
        'the landing screen must hide the saved-kpub list when no entries exist');

    const { networkState, transactionState, walletSession } = await import(pathToFileURL(path.join(jsRoot, 'app', 'state', 'index.js')).href);
    networkState.customNodeUrl = 'ws://runtime-node';
    networkState.network = 'testnet-10';
    elements.get('btn-multisig-welcome').click();
    elements.get('input-ms-descriptor').value = 'multi_hd45(2,runtime-a,runtime-b,runtime-c)';
    elements.get('input-ms-cosigner').value = '';
    elements.get('btn-ms-discover').click();
    await new Promise(resolve => setImmediate(resolve));
    assert.match(elements.get('ms-discovery-info').textContent, /Receive #0: kaspatest:runtime-ms-receive/,
        'multisig discovery must visibly render the next receive address even when there are no UTXOs');
    assert.match(elements.get('ms-discovery-info').textContent, /Change #0: kaspatest:runtime-ms-change/,
        'multisig discovery must visibly render the next change address');
    assert.equal(elements.get('input-ms-source').value, 'kaspatest:runtime-ms-receive',
        'empty multisig source must default to the discovered receive address for funding');
    assert.equal(elements.get('input-ms-cosigner').value, '0',
        'blank 45-prime cosigner branch input must normalize to branch 0');
    assert.equal(elements.get('input-ms-cosigner').max, '2',
        'descriptor participant count must bound the 45-prime cosigner branch input');
    assert.equal(globalThis.__COMPANION_TEST_MS_SCAN_REQUEST__.address_prefix, 'kaspatest',
        'multisig discovery must derive its address prefix from the selected testnet network');
    delete globalThis.__COMPANION_TEST_MS_SCAN_REQUEST__;
    assert.match(elements.get('ms-discovery-info').textContent, /regular wallet balance is separate/i,
        'zero-balance discovery must explain that ordinary wallet funds are not multisig funds');
    assert.equal(elements.get('btn-ms-discover').disabled, false,
        'multisig discovery must re-enable its control after completion');
    networkState.customNodeUrl = null;
    networkState.network = 'mainnet';
    elements.get('btn-ms-back').click();

    elements.get('btn-header-settings').click();
    assert.equal(elements.get('screen-settings').classList.contains('active'), true,
        'settings cog must enter Node Connection directly');
    elements.get('btn-settings-back-top').click();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'Node Back must return to Welcome through the shared history stack');

    const { kpubRepository, useKpubOnce } = await import(pathToFileURL(path.join(
        jsRoot,
        'features',
        'wallet',
        'kpub_manager',
        'index.js',
    )).href);
    elements.get('input-managed-kpub').value = `kpub1:${'44'.repeat(78)}`;
    assert.equal(useKpubOnce(), true, 'Use kpub once must load a valid temporary wallet');
    assert.equal(walletSession.profile(), null, 'one-time wallet must not have a saved profile');
    assert.equal(kpubRepository.list().length, 0, 'Use kpub once must not write the saved-kpub repository');
    assert.equal(elements.get('btn-reset-wallet').textContent, 'Unload wallet',
        'one-time wallet must use the consumer-facing Unload wallet action');
    transactionState._currentKsptHex = 'one-time-deadbeef';
    sessionStorage.setItem('companion_private_swap_v2', '{"role":"alice","stage":"offer"}');
    elements.get('btn-reset-wallet').click();
    assert.equal(elements.get('wallet-unload-modal').classList.contains('hidden'), false,
        'one-time unload must use the themed confirmation dialog');
    elements.get('btn-wallet-unload-confirm').click();
    assert.equal(walletSession.hasWallet(), false, 'one-time unload must clear the active wallet');
    assert.equal(transactionState._currentKsptHex, undefined, 'one-time unload must clear transaction state');
    assert.equal(sessionStorage.getItem('companion_private_swap_v2'), null, 'one-time unload must clear Private Swap session state');
    assert.equal(globalThis.__COMPANION_TEST_RELOADS__, 1, 'one-time unload must request a fresh JS/WASM realm');
    await startApplication();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'one-time unload must return the fresh app to Welcome');

    const startupEntry = kpubRepository.save({
        name: 'Startup wallet',
        kpub: `kpub1:${'33'.repeat(78)}`,
        network: 'mainnet',
    });
    await startApplication();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'saved kpubs without a startup selection must remain on the first landing screen');
    assert.equal(elements.get('welcome-saved-kpubs').classList.contains('hidden'), false,
        'saved kpubs without a startup selection must appear in a clickable landing-screen list');
    assert.equal(elements.get('companion-startup-status').textContent, '',
        'saved-kpub selection should not add redundant startup instructions');

    elements.get('btn-header-settings').click();
    elements.get('select-network').value = 'testnet-10';
    elements.get('btn-save-settings').click();
    assert.equal(networkState.network, 'testnet-10', 'network settings must apply the selected testnet');
    assert.equal(elements.get('welcome-saved-kpubs').classList.contains('hidden'), true,
        'switching to a network with no saved kpubs must hide mainnet saved entries immediately');

    elements.get('btn-header-settings').click();
    elements.get('select-network').value = 'mainnet';
    elements.get('btn-save-settings').click();
    assert.equal(networkState.network, 'mainnet', 'network settings must switch back to mainnet');
    assert.equal(elements.get('welcome-saved-kpubs').classList.contains('hidden'), false,
        'switching back to mainnet must restore only the mainnet saved-kpub section');

    kpubRepository.setAutoLoad(startupEntry.id);
    await startApplication();
    assert.equal(elements.get('screen-dashboard').classList.contains('active'), true,
        'a selected startup kpub must go directly to the loaded-wallet dashboard');
    assert.match(elements.get('companion-startup-status').textContent, /Loaded saved wallet/i);
    assert.equal(startupErrors.length, 0, 'successful startup must not report event-binding errors');

    // Coverage ratchet: exercise the lightweight UI handlers added for the
    // consumer dashboard/import flows. Keep these as real bound handlers so
    // coverage proves the application wiring rather than calling helpers
    // directly. Image-import change handlers intentionally use an empty file
    // list here; their decoder success/error paths are covered by dedicated
    // QR tests.
    elements.get('btn-upload-managed-kpub').click();
    await elements.get('input-managed-kpub-image').onchange({ currentTarget: elements.get('input-managed-kpub-image') });
    elements.get('btn-ms-descriptor-upload-saved').click();
    await elements.get('input-ms-descriptor-image-saved').onchange({ currentTarget: elements.get('input-ms-descriptor-image-saved') });

    elements.get('input-amount').value = '1.234567891x';
    elements.get('input-amount').oninput({ currentTarget: elements.get('input-amount') });
    assert.equal(elements.get('input-amount').value, '1.23456789',
        'bound amount input handler must enforce eight decimal places');

    elements.get('btn-scan-dest').click();
    elements.get('btn-scanner-cancel').click();
    elements.get('btn-scan-next-sig').click();
    elements.get('btn-scanner-cancel').click();
    elements.get('btn-qr-scan-signed').onclick({
        currentTarget: { dataset: { scanTitle: 'Runtime signed QR' } },
    });
    elements.get('btn-scanner-cancel').click();
    elements.get('btn-load-signed-qr-image').click();
    await elements.get('input-signed-qr-image').onchange({ currentTarget: elements.get('input-signed-qr-image') });

    elements.get('btn-advanced').click();
    assert.equal(elements.get('screen-advanced').classList.contains('active'), true,
        'loaded-wallet Advanced tile must open the conventional Advanced menu');
    elements.get('btn-broadcast').click();
    assert.equal(elements.get('screen-broadcast').classList.contains('active'), true,
        'Advanced Broadcast Transaction must open the broadcast screen');
    elements.get('btn-broadcast-back').click();
    assert.equal(elements.get('screen-advanced').classList.contains('active'), true,
        'Broadcast Back must return to Advanced when Broadcast was opened from Advanced');
    elements.get('btn-advanced-back').click();
    assert.equal(elements.get('screen-dashboard').classList.contains('active'), true,
        'Advanced Back must return to the dashboard');
    elements.get('btn-dashboard-history').click();
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(elements.get('screen-history').classList.contains('active'), true,
        'History must be reachable from the dashboard secondary row');
    elements.get('btn-history-back-top').click();
    assert.equal(elements.get('screen-dashboard').classList.contains('active'), true,
        'History Back must return to the dashboard');

    assert.equal(elements.get('btn-reset-wallet').textContent, 'Unload wallet',
        'saved wallets must use the same Unload wallet action');
    transactionState._currentKsptHex = 'saved-deadbeef';
    sessionStorage.setItem('companion_private_swap_v2', '{"role":"bob","stage":"ready"}');
    elements.get('btn-reset-wallet').click();
    assert.equal(elements.get('wallet-unload-modal').classList.contains('hidden'), false,
        'saved-wallet unload must use the themed confirmation dialog');
    elements.get('btn-wallet-unload-confirm').click();
    assert.equal(walletSession.hasWallet(), false, 'Unload wallet must clear the active saved wallet');
    assert.equal(transactionState._currentKsptHex, undefined, 'Unload wallet must use the hardened transaction cleanup');
    assert.equal(sessionStorage.getItem('companion_private_swap_v2'), null, 'Unload wallet must clear Private Swap session state');
    assert.equal(globalThis.__COMPANION_TEST_RELOADS__, 2, 'Unload wallet must request the same fresh JS/WASM realm');
    await startApplication();
    assert.equal(elements.get('screen-welcome').classList.contains('active'), true,
        'one-shot startup suppression must keep the app on Welcome after unloading a kpub');
    assert.equal(kpubRepository.autoLoadId(), startupEntry.id,
        'Unload wallet must preserve the saved startup-wallet preference');

    console.error = originalError;
    delete globalThis.__COMPANION_TEST_INIT_FAILURE__;
    delete globalThis.__COMPANION_TEST_RELOADS__;

    const addedGlobals = Reflect.ownKeys(globalThis)
        .filter(name => !globalsBeforeStartup.has(name))
        .filter(name => typeof name === 'string');
    if (addedGlobals.length) {
        throw new Error(`application startup leaked browser globals: ${addedGlobals.join(', ')}`);
    }

    console.log(`PASS: browser startup and centralized kpub controls (${wasmExports.length} WASM imports, no application globals)`);
} finally {
    await pkgFixture.restore();
}
