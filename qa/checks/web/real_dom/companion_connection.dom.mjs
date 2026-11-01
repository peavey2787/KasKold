// Balance refresh must reconnect quietly, keep the last known balance when the
// node is unavailable, and treat history tracking as supplemental.

import { assert, assertEqual, fakeWasm, mountCompanionShell, test } from './harness.mjs';

const JS = '/apps/kaskold-companion-web/web/js';
const WALLET = { kpub: 'kpub-test-only' };
const FUNDED = JSON.stringify({ total_sompi: '123000000', utxo_count: 2, funded_addresses: 1 });

async function loadBalance() {
    return import(`${JS}/features/wallet/core/balance.js`);
}

async function loadNetworkState() {
    const state = await import(`${JS}/app/state/index.js`);
    return state.networkState;
}

async function seedWallet() {
    const state = await import(`${JS}/app/state/index.js`);
    state.walletSession.replace(WALLET);
    const network = await loadNetworkState();
    network.customNodeUrl = 'ws://fake-node.invalid';
    return state;
}

function statusLabel() {
    return document.querySelector('#status-dot .label').textContent;
}

// Force every public-resolver fetch to fail so exhausted retries end quickly and offline.
function failResolverFetches() {
    const original = globalThis.fetch;
    globalThis.fetch = () => Promise.reject(new TypeError('network unreachable'));
    return () => { globalThis.fetch = original; };
}

test('transient node errors reconnect with visible progress and render the balance', async () => {
    await mountCompanionShell();
    await seedWallet();
    const seen = [];
    let calls = 0;
    fakeWasm({
        fetch_balance: () => {
            calls++;
            seen.push(statusLabel());
            if (calls === 1) throw new Error('request timeout');
            if (calls === 2) throw new Error('websocket closed');
            return FUNDED;
        },
        fetch_utxos: () => '[]',
    });
    const balance = await loadBalance();
    await balance.refreshBalance();

    assertEqual(calls, 3, 'fetch_balance attempts');
    assertEqual(seen[1], 'Reconnecting 1/3', 'status before second attempt');
    assertEqual(seen[2], 'Reconnecting 2/3', 'status before third attempt');
    assertEqual(statusLabel(), 'Connected', 'final status');
    assertEqual(document.getElementById('balance-kas').dataset.balanceSompi, '123000000', 'balance stored');
    assertEqual(document.getElementById('balance-info').textContent, '2 UTXOs across 1 address', 'balance detail');
});

test('a reconnecting balance never reveals the full-screen loading overlay', async () => {
    await mountCompanionShell();
    await seedWallet();
    const loading = document.getElementById('loading');
    loading.classList.add('hidden');
    let calls = 0;
    fakeWasm({
        fetch_balance: () => {
            calls++;
            if (calls === 1) throw new Error('websocket closed');
            return FUNDED;
        },
        fetch_utxos: () => '[]',
    });
    const balance = await loadBalance();
    await balance.refreshBalance();
    assertEqual(calls, 2, 'fetch_balance attempts');
    assert(loading.classList.contains('hidden'), 'loading overlay stays hidden while reconnecting');
});

test('exhausted reconnect keeps the last known balance and reports the node as unavailable', async () => {
    await mountCompanionShell();
    await seedWallet();
    const errors = [];
    const originalError = console.error;
    console.error = (...args) => { errors.push(args.map(String).join(' ')); };
    let calls = 0;
    let fail = false;
    fakeWasm({
        fetch_balance: () => {
            calls++;
            if (fail) throw new Error('websocket closed');
            return FUNDED;
        },
        fetch_utxos: () => '[]',
    });
    const restoreFetch = failResolverFetches();
    try {
        const balance = await loadBalance();
        await balance.refreshBalance();
        assertEqual(document.getElementById('balance-kas').dataset.balanceSompi, '123000000', 'first balance');

        fail = true;
        calls = 0;
        await balance.refreshBalance();
        // Attempts 1 and 2 hit the custom node; attempt 3 goes to the public resolver, which fails.
        assertEqual(calls, 2, 'fetch_balance attempts before resolver fallback');
        assertEqual(
            document.getElementById('balance-info').textContent,
            'Unable to reconnect after 3 attempts. Last known balance shown.',
            'failure detail keeps last known balance',
        );
        assertEqual(document.getElementById('balance-kas').dataset.balanceSompi, '123000000', 'last known balance kept');
        assertEqual(statusLabel(), 'Node unavailable', 'node status');
        assert(!errors.some(line => line.includes('Balance fetch failed:')), 'no console error for balance failure');
    } finally {
        restoreFetch();
        console.error = originalError;
    }
});

test('exhausted reconnect with no prior balance shows Unavailable', async () => {
    await mountCompanionShell();
    await seedWallet();
    fakeWasm({
        fetch_balance: () => { throw new Error('websocket closed'); },
        fetch_utxos: () => '[]',
    });
    const restoreFetch = failResolverFetches();
    try {
        const balance = await loadBalance();
        await balance.refreshBalance();
        assertEqual(document.getElementById('balance-kas').textContent, 'Unavailable', 'balance placeholder');
        assertEqual(document.getElementById('balance-info').textContent, 'Unable to reconnect after 3 attempts.', 'failure detail');
    } finally {
        restoreFetch();
    }
});

test('a non-retryable balance error is not retried', async () => {
    await mountCompanionShell();
    await seedWallet();
    let calls = 0;
    fakeWasm({
        fetch_balance: () => {
            calls++;
            throw new Error('invalid wallet descriptor');
        },
        fetch_utxos: () => '[]',
    });
    const balance = await loadBalance();
    await balance.refreshBalance();
    assertEqual(calls, 1, 'fetch_balance attempts');
});

test('history failure keeps the successfully fetched zero balance', async () => {
    await mountCompanionShell();
    await seedWallet();
    fakeWasm({
        fetch_balance: () => JSON.stringify({ total_sompi: '0', utxo_count: 0, funded_addresses: 0 }),
        fetch_utxos: () => { throw new Error('websocket timeout'); },
    });
    const balance = await loadBalance();
    await balance.refreshBalance();
    assertEqual(document.getElementById('balance-kas').dataset.balanceSompi, '0', 'zero balance kept');
    assertEqual(document.getElementById('balance-info').textContent, '0 UTXOs across 0 addresses', 'balance detail kept');
    assertEqual(statusLabel(), 'Connected', 'history failure does not change connected status');
});
