import { networkState, walletSession } from '../../../app/state/index.js';
import { kaspaRestApiBase } from '../../../core/config/network.js';
import { toast } from '../../../core/ui/toast.js';
import { getAccountPubkeyHex } from '../generation/panel_keys.js';
import { decryptCovenantPayload } from '../payload/payload.js';
import { covRenderActive, covSaveActive } from './active.js';
// KasKold Companion Web — features/covenants/recovery/scanner
import { rebuildCovenant } from './scanner/rebuild.js';
export { rebuildCovenant } from './scanner/rebuild.js';


// ─── Covenant Recovery Scanner ───
// Scans TX history for all wallet addresses, finds TXs with payloads,
// decrypts with kpub-derived key, rebuilds covenant addresses, checks balances.

const EMPTY_PAYLOAD_HEX = '0000000000000000';
const MIN_PAYLOAD_HEX = 60; // min 30 bytes = 60 hex

function resolveApiBase() {
    try { return kaspaRestApiBase(networkState.network); }
    catch (error) { toast(error.message, 'error'); return null; }
}

async function fetchAddressTxs(apiBase, addr) {
    const r = await fetch(
        `${apiBase}/addresses/${addr}/full-transactions?resolve_previous_outpoints=light&limit=50`,
        { signal: AbortSignal.timeout(10000) }
    );
    if (!r.ok) return [];
    const txs = await r.json();
    return Array.isArray(txs) ? txs : [];
}

// full-transactions may not include the payload, so fetch the individual TX.
async function fetchTxPayload(apiBase, txId) {
    try {
        const txr = await fetch(`${apiBase}/transactions/${txId}`, { signal: AbortSignal.timeout(5000) });
        if (!txr.ok) return undefined;
        return (await txr.json()).payload;
    } catch (_) {
        return undefined;
    }
}

async function resolvePayloadHex(apiBase, tx) {
    if (tx.payload && tx.payload !== EMPTY_PAYLOAD_HEX) return tx.payload;
    return fetchTxPayload(apiBase, tx.transaction_id);
}

// TN10 API doesn't reliably populate previous_outpoint_address, so there is no fromUs check;
// decrypt failure is the filter: only payloads encrypted with our key will decrypt.
async function tryRecoverTx(apiBase, tx, ownerPk) {
    const payloadHex = await resolvePayloadHex(apiBase, tx);
    if (!payloadHex || payloadHex.length < MIN_PAYLOAD_HEX) return false;
    const decrypted = await decryptCovenantPayload(payloadHex);
    if (!decrypted) return false;
    return Boolean(await rebuildCovenant(decrypted, ownerPk));
}

async function scanAddressTransactions(apiBase, addr, ownerPk, seenTxIds) {
    const tally = { scanned: 0, found: 0 };
    try {
        for (const tx of await fetchAddressTxs(apiBase, addr)) {
            if (seenTxIds.has(tx.transaction_id)) continue;
            seenTxIds.add(tx.transaction_id);
            tally.scanned++;
            if (await tryRecoverTx(apiBase, tx, ownerPk)) tally.found++;
        }
    } catch (e) {
        console.log('[Companion] Recovery: error scanning', addr, e);
    }
    return tally;
}

function reportRecoveryResult(found) {
    if (found > 0) {
        toast('Recovered ' + found + ' covenant(s) from chain', 'ok', 4000);
        covSaveActive();
        covRenderActive();
    } else {
        toast('No covenant payloads found on chain', 'info', 3000);
    }
}

export async function recoverCovenants() {
    if (!walletSession.hasWallet()) { toast('Load wallet first', 'error'); return; }
    const wallet = walletSession.current();
    const apiBase = resolveApiBase();
    if (!apiBase) return;

    const ownerPk = getAccountPubkeyHex();
    const allAddresses = [...(wallet.receive_addresses || []), ...(wallet.change_addresses || [])];

    toast('Scanning chain for covenant payloads...', 'info', 3000);
    console.log('[Companion] Recovery: scanning', allAddresses.length, 'addresses');

    let found = 0;
    let scanned = 0;
    const seenTxIds = new Set();
    for (const addr of allAddresses) {
        const tally = await scanAddressTransactions(apiBase, addr, ownerPk, seenTxIds);
        scanned += tally.scanned;
        found += tally.found;
    }

    console.log('[Companion] Recovery complete:', scanned, 'TXs scanned,', found, 'covenants recovered');
    reportRecoveryResult(found);
}
