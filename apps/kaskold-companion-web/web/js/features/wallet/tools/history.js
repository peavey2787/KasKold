import { networkState, walletSession, walletState } from '../boundary.js';
import { kaspaRestApiBase } from '../../../core/config/network.js';
import { hideLoading, showLoading, showScreen } from '../../../app/navigation.js';

import { timeAgo } from '../../../core/time.js';
import { byId } from '../../../core/dom.js';
import { formatTransactionTime } from '../../../core/format.js';
import { explorerTransactionUrl } from '../../../core/explorer.js';
import { exactUnsigned } from '../../../core/exact.js';
import { sompiToKasFixed } from '../../../core/amounts.js';
export function showHistory() {
    if (!walletSession.hasWallet()) return;
    showLoading('Loading transaction history...');
    fetchArchivalHistory().then(() => {
        hideLoading();
        renderHistory();
        showScreen('history');
    }).catch(e => {
        hideLoading();
        // Fall back to session-only history
        renderHistory();
        showScreen('history');
        console.log('[Companion] archival history fetch failed, showing session data:', e);
    });
}

async function fetchAddressTransactions(apiBase, addr) {
    const r = await fetch(
        `${apiBase}/addresses/${addr}/full-transactions?resolve_previous_outpoints=light`,
        { signal: AbortSignal.timeout(10000) }
    );
    if (!r.ok) return null;
    const txs = await r.json();
    return Array.isArray(txs) ? txs : null;
}

function sumInputs(inputs, myAddressSet) {
    let fromUs = 0n;
    let total = 0n;
    const senders = [];
    for (const inp of inputs) {
        const amt = exactUnsigned(inp.previous_outpoint_amount ?? 0n, 'history input sompi');
        total += amt;
        const addr = inp.previous_outpoint_address;
        if (!addr) continue;
        if (myAddressSet.has(addr)) fromUs += amt;
        else senders.push(addr);
    }
    return { fromUs, total, senders };
}

function sumOutputs(outputs, myAddressSet) {
    let toUs = 0n;
    let total = 0n;
    const recipients = [];
    for (const out of outputs) {
        const amt = exactUnsigned(out.amount ?? 0n, 'history output sompi');
        total += amt;
        const addr = out.script_public_key_address;
        if (!addr) continue;
        if (myAddressSet.has(addr)) toUs += amt;
        else recipients.push(addr);
    }
    return { toUs, total, recipients };
}

// Direction: if we funded inputs, it's outgoing; otherwise incoming
function describeDirection(ins, outs) {
    if (ins.fromUs > 0n) {
        // We spent — outgoing. Amount = what left our wallet (excluding change back to us)
        return { type: 'out', amount: ins.fromUs - outs.toUs, counterparty: outs.recipients[0] ?? null };
    }
    return { type: 'in', amount: outs.toUs, counterparty: ins.senders[0] ?? null };
}

function buildTxEntry(tx, myAddressSet) {
    const ins = sumInputs(tx.inputs || [], myAddressSet);
    const outs = sumOutputs(tx.outputs || [], myAddressSet);
    const fee = ins.total > 0n ? ins.total - outs.total : 0n;
    const { type, amount, counterparty } = describeDirection(ins, outs);
    return {
        type,
        amount,
        fee,
        tx_id: tx.transaction_id,
        time: tx.block_time || tx.accepting_block_time || 0,
        counterparty,
        is_accepted: tx.is_accepted !== false,
    };
}

function recordAddressTransactions(txs, myAddressSet, txMap) {
    for (const tx of txs) {
        if (txMap.has(tx.transaction_id)) continue;
        txMap.set(tx.transaction_id, buildTxEntry(tx, myAddressSet));
    }
}

async function fetchArchivalHistory() {
    if (!walletSession.hasWallet()) return;
    const wallet = walletSession.current();
    const allAddresses = [...wallet.receive_addresses, ...wallet.change_addresses];
    const myAddressSet = new Set(allAddresses);

    const apiBase = kaspaRestApiBase(networkState.network);

    const txMap = new Map(); // tx_id → processed entry

    // Fetch full-transactions for each address in parallel
    const fetches = allAddresses.map(async (addr) => {
        try {
            const txs = await fetchAddressTransactions(apiBase, addr);
            if (txs) recordAddressTransactions(txs, myAddressSet, txMap);
        } catch { /* One address failing leaves the others in the history. */ }
    });

    await Promise.all(fetches);

    // Merge archival data into historyEntries, replacing session-only entries
    if (txMap.size > 0) {
        // Keep session entries that aren't in archival (very recent, not yet indexed)
        const archivalIds = new Set(txMap.keys());
        const sessionOnly = walletState.historyEntries.filter(h => !archivalIds.has(h.tx_id));

        // Build merged list: archival (sorted by time desc) + session-only at top
        const archival = [...txMap.values()].sort((a, b) => b.time - a.time);
        walletState.historyEntries = [...sessionOnly, ...archival];

        // Cap at 200
        if (walletState.historyEntries.length > 200) walletState.historyEntries.length = 200;
    }
}

function renderHistory() {
    const list = byId('history-list');
    list.replaceChildren();
    byId('screen-history')?.classList.toggle('history-compact', walletState.historyEntries.length <= 4);
    if (walletState.historyEntries.length === 0) {
        byId('history-summary').textContent = 'No transactions found';
        const empty = document.createElement('div');
        empty.className = 'u-align-text-center-text-text-muted-padding-20px';
        empty.textContent = 'No transaction history available';
        list.appendChild(empty);
        return;
    }

    byId('history-summary').textContent = walletState.historyEntries.length + ' transaction' + (walletState.historyEntries.length !== 1 ? 's' : '');
    walletState.historyEntries.forEach(h => {
        const kas = sompiToKasFixed(exactUnsigned(h.amount ?? 0n, 'history amount sompi'), 8);
        const incoming = h.type === 'in';
        const cls = incoming ? 'incoming' : 'outgoing';
        const item = document.createElement('div');
        item.className = 'history-item';

        const icon = document.createElement('div');
        icon.className = `history-icon ${cls}`;
        icon.textContent = incoming ? '↓' : '↑';
        item.appendChild(icon);

        const info = document.createElement('div');
        info.className = 'history-info';
        const amount = document.createElement('div');
        amount.className = `history-amount ${cls}`;
        amount.textContent = `${incoming ? '+' : '-'}${kas} KAS`;
        info.appendChild(amount);

        const time = document.createElement('div');
        time.className = 'history-time';
        const timeStr = h.time > 1e12 ? formatTransactionTime(h.time) : (h.time > 0 ? timeAgo(h.time) : '');
        time.append(document.createTextNode(timeStr));
        if (h.tx_id) {
            time.append(document.createTextNode(' · '));
            const link = document.createElement('a');
            link.className = 'u-text-teal-dim';
            link.href = explorerTransactionUrl(networkState.network, h.tx_id);
            link.target = '_blank';
            link.rel = 'noopener';
            link.textContent = h.tx_id.slice(0, 12) + '…';
            time.appendChild(link);
        }
        info.appendChild(time);

        if (h.counterparty) {
            const counterparty = document.createElement('div');
            counterparty.className = 'history-time';
            counterparty.textContent = `${incoming ? 'from' : 'to'} ${String(h.counterparty).slice(0, 16)}…`;
            info.appendChild(counterparty);
        }
        item.appendChild(info);
        list.appendChild(item);
    });
}

export function clearHistory() {
    if (!confirm('Clear transaction history?')) return;
    walletState.historyEntries = [];
    networkState.utxoSnapshot = null;
    walletState.fundedReceiveIndices = [];
    walletState.fundedChangeIndices = [];
    walletState.usedReceiveIndices = new Set();
    walletState.usedChangeIndices = new Set();
    renderHistory();
}
