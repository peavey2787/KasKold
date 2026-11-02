// @ts-check
import { networkState, walletSession, extend_addresses } from './boundary.js';
import { kaspaRestApiBase } from '../../core/config/network.js';
import { exactUnsigned, exactUnsignedJsonField } from '../../core/exact.js';
import { importedTxIds, newId } from './repository.js';
import { kasValueMicro } from './exact_money.js';
import { loadBundledHistory, historicalPriceAt } from './pricing.js';

const DISCOVERY_GAP = 20;
const MAX_ADDRESSES_PER_CHAIN = 512;
const HISTORY_PAGE_LIMIT = 500;
const HISTORY_PAGE_CAP = 100;
const SYNC_VERSION = 1;

async function fetchText(url, options = {}) {
    let lastError;
    for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
            const response = await fetch(url, { ...options, signal: AbortSignal.timeout(20_000) });
            if (response.status === 429 && attempt < 2) { await new Promise(resolve => setTimeout(resolve, 1000 * (attempt + 1))); continue; }
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            return { text: await response.text(), headers: response.headers };
        } catch (error) {
            lastError = error;
        }
    }
    throw lastError || new Error('history request failed');
}

function quoteExactAmountFields(raw) {
    return raw.replace(/"(previous_outpoint_amount|amount)"\s*:\s*(0|[1-9]\d*)(?=\s*[,}])/g, '"$1":"$2"');
}

async function addressUsed(apiBase, address) {
    const result = await fetchText(`${apiBase}/addresses/${address}/transactions-count`);
    return exactUnsignedJsonField(result.text, 'total', 'transaction count') > 0n;
}

async function scanBatch(apiBase, addresses, start, end) {
    const results = await Promise.all(addresses.slice(start, end).map(address => addressUsed(apiBase, address)));
    const usedIndices = [];
    for (let offset = 0; offset < results.length; offset += 1) if (results[offset]) usedIndices.push(start + offset);
    return usedIndices;
}

function chainSync(previous, chain) {
    const value = previous?.[chain];
    if (!value || !Array.isArray(value.usedIndices)) return null;
    return {
        addressCount: Number.isSafeInteger(value.addressCount) && value.addressCount >= 0 ? value.addressCount : 0,
        usedIndices: value.usedIndices.filter(index => Number.isSafeInteger(index) && index >= 0 && index < MAX_ADDRESSES_PER_CHAIN),
    };
}

function chainAddresses(wallet, chain) {
    return chain === 'receive' ? wallet.receive_addresses : wallet.change_addresses;
}

function ensureChainAddresses(wallet, chain, minimum) {
    const current = chainAddresses(wallet, chain).length;
    if (current >= minimum) return wallet;
    const extra = minimum - current;
    return JSON.parse(extend_addresses(JSON.stringify(wallet), chain === 'receive' ? extra : 0, chain === 'change' ? extra : 0, networkState.network));
}

async function scanNextWindow(apiBase, scan) {
    const scanEnd = scan.scanned + DISCOVERY_GAP;
    scan.wallet = ensureChainAddresses(scan.wallet, scan.chain, Math.min(MAX_ADDRESSES_PER_CHAIN, scanEnd));
    const addresses = chainAddresses(scan.wallet, scan.chain);
    const end = Math.min(addresses.length, scanEnd);
    const batchUsed = await scanBatch(apiBase, addresses, scan.scanned, end);
    for (const index of batchUsed) scan.usedIndices.add(index);
    const batchHighest = batchUsed.length ? batchUsed.at(-1) : -1;
    if (batchHighest > scan.highestUsed) scan.highestUsed = batchHighest;
    scan.scanned = end;
}

function retainedAddressCount(highestUsed, length) {
    return highestUsed < 0 ? Math.min(DISCOVERY_GAP, length) : Math.min(highestUsed + DISCOVERY_GAP + 1, length);
}

/** @param {any} apiBase @param {any} walletJson @param {any} chain @param {{ addressCount: number, usedIndices: number[] } | null} [previous] */
async function discoverChain(apiBase, walletJson, chain, previous = null) {
    const priorCount = Math.min(MAX_ADDRESSES_PER_CHAIN, previous?.addressCount || 0);
    const usedIndices = new Set(previous?.usedIndices || []);
    // First run starts at zero for a deep discovery. Follow-up runs re-check the
    // previous trailing gap, then extend forward only as needed. Previously used
    // addresses are retained and queried for recent transactions separately.
    const scan = {
        chain,
        wallet: ensureChainAddresses(JSON.parse(walletJson), chain, priorCount),
        usedIndices,
        highestUsed: usedIndices.size ? Math.max(...usedIndices) : -1,
        scanned: previous ? Math.max(0, priorCount - DISCOVERY_GAP) : 0,
    };
    while (scan.scanned < MAX_ADDRESSES_PER_CHAIN) {
        await scanNextWindow(apiBase, scan);
        if (scan.highestUsed < scan.scanned - DISCOVERY_GAP) break;
    }
    const addresses = chainAddresses(scan.wallet, chain);
    const last = retainedAddressCount(scan.highestUsed, addresses.length);
    return { addresses: addresses.slice(0, last), usedIndices };
}

function currentWalletBinding() {
    if (!walletSession.hasWallet()) throw new Error('Load a wallet before fetching wallet history');
    const kpub = String(walletSession.kpub() || '').trim();
    if (!kpub) throw new Error('Loaded wallet does not expose a kpub');
    return {
        network: networkState.network,
        kpub,
        profileName: walletSession.profile()?.name || '',
    };
}

function validateExistingBinding(sync, binding) {
    if (!sync || sync.version !== SYNC_VERSION) return;
    if (sync.network !== binding.network) {
        throw new Error(`Portfolio history is linked to ${sync.network}; switch networks before fetching`);
    }
    if (sync.kpub !== binding.kpub) {
        throw new Error('Portfolio history is linked to a different wallet; load that kpub before fetching');
    }
}

export function walletHistoryFetchMode(store, accountId) {
    const account = store.accounts.find(candidate => candidate.id === accountId);
    return account?.walletHistory?.version === SYNC_VERSION ? 'incremental' : 'deep';
}

export async function discoverHistoricalWalletAddresses(previousSync = null) {
    const binding = currentWalletBinding();
    validateExistingBinding(previousSync, binding);
    const apiBase = kaspaRestApiBase(networkState.network);
    const walletJson = walletSession.json();
    const [receive, change] = await Promise.all([
        discoverChain(apiBase, walletJson, 'receive', chainSync(previousSync, 'receive')),
        discoverChain(apiBase, walletJson, 'change', chainSync(previousSync, 'change')),
    ]);
    return { apiBase, receive, change, binding };
}

/** @param {any} apiBase @param {any} address @param {Set<string> | null} [stopTxIds] */
async function fetchAddressTransactions(apiBase, address, stopTxIds = null) {
    const transactions = [];
    let before = null;
    const seen = new Set();
    for (let page = 0; page < HISTORY_PAGE_CAP; page += 1) {
        const params = new URLSearchParams({ limit: String(HISTORY_PAGE_LIMIT), resolve_previous_outpoints: 'light' });
        if (before) params.set('before', before);
        const result = await fetchText(`${apiBase}/addresses/${address}/full-transactions-page?${params.toString()}`);
        const decoded = JSON.parse(quoteExactAmountFields(result.text));
        if (!Array.isArray(decoded)) throw new Error('history page is not an array');
        transactions.push(...decoded);
        // Full transaction pages are newest-first. On incremental sync, once a
        // known transaction is reached there is no reason to walk older pages.
        if (stopTxIds && decoded.some(transaction => stopTxIds.has(transaction.transaction_id))) break;
        const next = result.headers.get('X-Next-Page-Before');
        if (!next || seen.has(next)) break;
        seen.add(next);
        before = next;
    }
    return transactions;
}

function walletNet(transaction, addresses) {
    let inputs = 0n;
    let outputs = 0n;
    for (const input of transaction.inputs || []) {
        if (addresses.has(input.previous_outpoint_address)) inputs += exactUnsigned(input.previous_outpoint_amount ?? '0', 'wallet history input');
    }
    for (const output of transaction.outputs || []) {
        if (addresses.has(output.script_public_key_address)) outputs += exactUnsigned(output.amount ?? '0', 'wallet history output');
    }
    return outputs - inputs;
}

function transactionFeeSompi(transaction) {
    let inputTotal = 0n;
    let outputTotal = 0n;
    for (const input of transaction.inputs || []) inputTotal += exactUnsigned(input.previous_outpoint_amount ?? '0', 'wallet history fee input');
    for (const output of transaction.outputs || []) outputTotal += exactUnsigned(output.amount ?? '0', 'wallet history fee output');
    return inputTotal >= outputTotal ? inputTotal - outputTotal : 0n;
}

function transactionTimestamp(transaction) {
    const value = transaction.accepting_block_time ?? transaction.block_time ?? 0;
    if (!Number.isSafeInteger(value) || value <= 0) return Date.now();
    return value > 1_000_000_000_000 ? value : value * 1000;
}

function mapImported(transaction, accountId, addresses, prices) {
    const txId = String(transaction.transaction_id || '').trim();
    if (!txId) return null;
    const net = walletNet(transaction, addresses);
    if (net === 0n) return null;
    const timestampMs = transactionTimestamp(transaction);
    const amount = net > 0n ? net : -net;
    const priceMicroUsd = historicalPriceAt(prices, timestampMs);
    const feeMicroUsd = kasValueMicro(transactionFeeSompi(transaction), priceMicroUsd);
    return {
        id: newId(), portfolioId: accountId,
        type: net > 0n ? 'Transfer In' : 'Transfer Out',
        kasSompi: amount.toString(),
        priceMicroUsd: priceMicroUsd.toString(),
        feeMicroUsd: feeMicroUsd.toString(), timestampMs,
        notes: 'Fetched from Kaspa wallet history', sourceTxId: txId, createdAt: Date.now(),
    };
}

function nextSync(previousSync, discovered) {
    const now = Date.now();
    const encodeChain = chain => ({
        addressCount: chain.addresses.length,
        usedIndices: [...chain.usedIndices].sort((left, right) => left - right),
    });
    return {
        version: SYNC_VERSION,
        network: discovered.binding.network,
        kpub: discovered.binding.kpub,
        profileName: discovered.binding.profileName,
        initializedAt: previousSync?.initializedAt || now,
        lastSyncAt: now,
        receive: encodeChain(discovered.receive),
        change: encodeChain(discovered.change),
    };
}

export async function fetchWalletHistory(store, accountId) {
    const account = store.accounts.find(candidate => candidate.id === accountId);
    if (!account) throw new Error('Portfolio not found');
    const previousSync = account.walletHistory?.version === SYNC_VERSION ? account.walletHistory : null;
    const mode = previousSync ? 'incremental' : 'deep';
    const discovered = await discoverHistoricalWalletAddresses(previousSync);
    const receiveUsed = [...discovered.receive.usedIndices].map(index => discovered.receive.addresses[index]).filter(Boolean);
    const changeUsed = [...discovered.change.usedIndices].map(index => discovered.change.addresses[index]).filter(Boolean);
    const addresses = new Set([...discovered.receive.addresses, ...discovered.change.addresses]);
    const active = [...new Set([...receiveUsed, ...changeUsed])];
    const existing = importedTxIds(store, accountId);
    const stopTxIds = mode === 'incremental' ? existing : null;
    const pages = await Promise.all(active.map(address => fetchAddressTransactions(discovered.apiBase, address, stopTxIds)));
    const byId = new Map();
    for (const transaction of pages.flat()) if (transaction.transaction_id) byId.set(transaction.transaction_id, transaction);
    const prices = await loadBundledHistory();
    const entries = [];
    for (const transaction of byId.values()) {
        if (existing.has(transaction.transaction_id)) continue;
        const entry = mapImported(transaction, accountId, addresses, prices);
        if (entry) entries.push(entry);
    }
    return {
        mode,
        entries: entries.sort((left, right) => left.timestampMs - right.timestampMs),
        sync: nextSync(previousSync, discovered),
    };
}

// Kept as an internal compatibility name for focused QA modules. New UI code
// uses Fetch Wallet History terminology and the explicit deep/incremental result.
export async function importWalletHistory(store, accountId) {
    return (await fetchWalletHistory(store, accountId)).entries;
}
