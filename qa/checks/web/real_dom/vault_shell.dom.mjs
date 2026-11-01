// The Software Vault shell must render the creation, navigation and consumer
// copy a browser actually shows, observed through the parsed and styled DOM.

import { assert, assertEqual, mount, test } from './harness.mjs';

const VAULT = '/apps/kaskold-vault-web/web';

const RETIRED_COPY = [
    'Create new recovery words securely.',
    'Words, SeedQR, local backup file, or advanced restore.',
    'Maximum 20 UTF-8 bytes, matching the M5 wallet slot limit.',
    'Web equivalent: password-encrypted ciphertext is stored locally in this browser.',
    'Wallet is cleared when this Vault session is locked or closed.',
    'Vault unlocked.',
];

async function fetchVaultFile(path) {
    const response = await fetch(`${VAULT}/${path}`, { cache: 'no-store' });
    if (!response.ok) throw new Error(`${path} returned HTTP ${response.status}`);
    return response.text();
}

/** The Vault index.html body mounted into the document; scripts stay inert. */
async function mountVaultShell() {
    const markup = new DOMParser().parseFromString(await fetchVaultFile('index.html'), 'text/html');
    const host = mount();
    for (const script of markup.body.querySelectorAll('script')) script.remove();
    for (const node of [...markup.body.childNodes]) host.append(document.importNode(node, true));
    return host;
}

/** Applies vault.css to the mounted shell so computed styles reflect the Vault. */
async function styleVaultShell(host) {
    const style = document.createElement('style');
    style.textContent = await fetchVaultFile('css/vault.css');
    host.append(style);
    return host;
}

function requireOne(host, selector) {
    const element = host.querySelector(selector);
    assert(element, `missing element matching ${selector}`);
    return element;
}

function requireAbsent(host, selector) {
    assert(!host.querySelector(selector), `unexpected element matching ${selector}`);
}

function precedes(first, second, label) {
    assert(first && second, `${label}: element missing`);
    assert(first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING, `${label}: wrong order`);
}

test('the Vault shell exposes every browser creation and restore control', async () => {
    const host = await mountVaultShell();
    for (const id of ['onboarding-create', 'onboarding-12', 'onboarding-24', 'recovery-material-submit', 'review-approve']) {
        requireOne(host, `#${id}`);
    }
});

test('the Vault shell loads its own stylesheet, script and QR library', async () => {
    const markup = new DOMParser().parseFromString(await fetchVaultFile('index.html'), 'text/html');
    const references = [...markup.querySelectorAll('link[href], script[src]')]
        .map(element => (element.getAttribute('href') || element.getAttribute('src')).split('?')[0].replace(/^\.\//, ''));
    for (const asset of ['css/vault.css', 'js/vault_main.js', 'lib/vault_jsQR.js']) {
        assert(references.includes(asset), `index.html does not reference ${asset}`);
        await fetchVaultFile(asset);
    }
});

test('the Vault shell shows no retired copy', async () => {
    const host = await mountVaultShell();
    for (const retired of RETIRED_COPY) {
        assert(!host.textContent.includes(retired), `retired copy is rendered: ${retired}`);
    }
    const signerLabel = [...host.querySelectorAll('*')].some(element => element.children.length === 0 && element.textContent.trim() === 'SIGNER');
    assert(!signerLabel, 'retired SIGNER label is rendered');
});

test('the Vault home screen uses the consumer wallet layout', async () => {
    const host = await mountVaultShell();
    requireOne(host, '#screen-locked');
    const headings = [...host.querySelectorAll('h2')].map(heading => heading.textContent);
    assert(headings.includes('Wallets'), 'the wallets heading must read exactly "Wallets"');
    requireOne(host, '#wallet-add-open');
    requireAbsent(host, '#wallet-list');
    requireAbsent(host, '#unlock-saved');
    requireOne(host, '#saved-wallets');
    requireOne(host, '#saved-wallet-list');
});

test('screen headings are centred', async () => {
    const host = await styleVaultShell(await mountVaultShell());
    // The review title sits beside the Sompi toggle in a flex row and is left-aligned by design.
    const headings = [...host.querySelectorAll('.screen h2')].filter(heading => !heading.closest('.review-heading-row'));
    assert(headings.length > 0, 'the Vault has no screen headings');
    for (const heading of headings) {
        assertEqual(getComputedStyle(heading).textAlign, 'center', `heading "${heading.textContent}" is not centred`);
    }
});

test('the 24-word option is offered before the 12-word option', async () => {
    const host = await mountVaultShell();
    precedes(requireOne(host, '#onboarding-24'), requireOne(host, '#onboarding-12'), '24 words before 12 words');
});

test('the wallet name field caps input at 64 characters', async () => {
    const host = await mountVaultShell();
    requireOne(host, '[maxlength="64"]');
});

test('backup and response copy controls are present', async () => {
    const host = await mountVaultShell();
    requireOne(host, '#backup-show-all');
    requireOne(host, '#response-copy-all');
});

test('the kpub QR appears before the home kpub and scanning can be cancelled', async () => {
    const host = await mountVaultShell();
    precedes(requireOne(host, '#kpub-qr'), requireOne(host, '#home-kpub'), 'kpub QR before home kpub');
    requireOne(host, '#scan-cancel[data-back]');
});

test('the Vault shell has no PIN storage option', async () => {
    const host = await mountVaultShell();
    requireAbsent(host, '#storage-use-pin');
});

test('the Vault shell does not force uppercase text', async () => {
    const host = await styleVaultShell(await mountVaultShell());
    for (const element of host.querySelectorAll('*')) {
        assertEqual(getComputedStyle(element).textTransform, 'none', `${element.tagName}#${element.id} is uppercased`);
    }
    assert(![...host.querySelectorAll('h2')].some(heading => heading.textContent === 'WALLETS'), 'the wallets heading is uppercase');
});

test('the Companion welcome offers management and watch-only tools only', async () => {
    const markup = new DOMParser().parseFromString(
        await fetch('/apps/kaskold-companion-web/web/html/screens/system/welcome.html', { cache: 'no-store' }).then(response => response.text()),
        'text/html',
    );
    const rendered = markup.body.textContent;
    for (const required of ['Manage Wallets', 'Multisig', 'Broadcast TX']) {
        assert(rendered.includes(required), `welcome is missing ${required}`);
    }
    for (const forbidden of ['Multisig / Multi-spend', 'Watch only · no private keys', 'Create 12', 'Create 24', 'Restore wallet', 'Hot Wallet']) {
        assert(!rendered.includes(forbidden), `welcome still shows ${forbidden}`);
    }
});
