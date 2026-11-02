import { networkState, transactionState, walletSession, walletState } from '../../../app/state/index.js';
import { hideLoading, showLoading } from '../../../app/navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { openPsktReview } from '../../transactions/pskt_multisig/review.js';
import { getNextReceiveIndex, walletWithFreshIndices, withNodeRetry } from '../core.js';
import { consolidationFee } from '../../transactions/send/compose/fees.js';
import { create_consolidate_pskb, create_send_pskb_selected, decode_address } from '../../../wasm/api.js';
import { openSendScreenWithSelectedUtxos } from '../../transactions/send/compose/send_form.js';
import { utxoId } from '../../transactions/shared/utxo_selection.js';

import { hexToBytes } from '../../../core/bytes.js';
import { byId } from '../../../core/dom.js';
import { exactUnsigned } from '../../../core/exact.js';
transactionState.consolidateSelection = new Set();
export function updateConsolidateButtons(totalCount) {
    const n = transactionState.consolidateSelection.size;
    const btnAll = byId('btn-consolidate');
    const btnSel = byId('btn-consolidate-selected');
    const btnSend = byId('btn-send-selected-utxos');
    btnSend.style.display = totalCount >= 1 ? 'block' : 'none';
    btnSend.textContent = n >= 1 ? `Send with ${n} Selected` : 'Select UTXOs for Send';
    if (totalCount <= 1) {
        btnAll.style.display = 'none';
        btnSel.style.display = 'none';
    } else if (n >= 2) {
        btnAll.style.display = 'none';
        btnSel.style.display = '';
        btnSel.textContent = `Consolidate ${n} Selected`;
    } else {
        btnAll.style.display = '';
        btnSel.style.display = 'none';
    }
}

export async function handleSendSelectedUtxos() {
    if (!walletSession.hasWallet()) return;
    if (transactionState.consolidateSelection.size < 1) {
        toast('Tap one or more UTXOs below to choose the exact inputs for this send', 'info', 2800);
        return;
    }
    const ids = [...transactionState.consolidateSelection]
        .sort((a, b) => a - b)
        .map(index => networkState.cachedUtxos?.[index])
        .filter(Boolean)
        .map(utxoId);
    if (!ids.length) {
        toast('Selected UTXOs are no longer available', 'error');
        return;
    }
    await openSendScreenWithSelectedUtxos(ids);
}

export async function handleConsolidate() {
    if (!walletSession.hasWallet()) return;
    // Builder takes up to 5 largest UTXOs; size the fee to that actual count.
    const fee = consolidationFee(Math.min(5, (networkState.cachedUtxos && networkState.cachedUtxos.length) || 5));

    showLoading('Building consolidation TX...');
    try {
        const pskbHex = await withNodeRetry(wsUrl =>
            create_consolidate_pskb(walletWithFreshIndices(), fee, wsUrl)
        );
        hideLoading();
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Consolidation failed: ' + e, 'error', 5000);
    }
}

export async function handleConsolidateSelected() {
    if (!walletSession.hasWallet() || transactionState.consolidateSelection.size < 2) return;
    const wallet = walletSession.current();
    const fee = consolidationFee(transactionState.consolidateSelection.size);
    const indices = [...transactionState.consolidateSelection].sort((a, b) => a - b);
    const indicesCsv = indices.join(',');

    // Calculate total of selected UTXOs
    let totalSelected = 0n;
    for (const idx of indices) {
        if (networkState.cachedUtxos && idx < networkState.cachedUtxos.length) {
            totalSelected += exactUnsigned(networkState.cachedUtxos[idx].amount, 'UTXO amount');
        }
    }
    const sendSompi = totalSelected - fee;
    if (sendSompi <= 0n) {
        toast('Selected UTXOs too small to cover fee', 'error');
        return;
    }

    showLoading(`Consolidating ${indices.length} UTXOs...`);
    try {
        const destAddr = wallet.receive_addresses[getNextReceiveIndex()];
        const pskbHex = await withNodeRetry(wsUrl =>
            create_send_pskb_selected(walletWithFreshIndices(), destAddr, sendSompi, fee, indicesCsv, wsUrl)
        );
        hideLoading();
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Consolidation failed: ' + e, 'error', 5000);
    }
}

function utxoKey(u) {
    return u.tx_id + ':' + u.index;
}

function makeHistoryEntry(type, u, now) {
    return { type, amount: u.amount, tx_id: u.tx_id, index: u.index, time: now };
}

function trimHistoryEntries() {
    if (walletState.historyEntries.length > 100) walletState.historyEntries.length = 100;
}

function scriptMatchesAddress(address, spkJson) {
    try {
        const decoded = JSON.parse(decode_address(address));
        // P2PK script: [0x20, ...32 bytes..., 0xAC]
        const spk = [0x20, ...Array.from(hexToBytes(decoded.payload)), 0xAC];
        return JSON.stringify(spk) === spkJson;
    } catch {
        return false;
    }
}

function markSpentAddressUsed(wallet, u) {
    const spkJson = JSON.stringify(u.script_public_key);
    const receiveIdx = wallet.receive_addresses.findIndex(addr => scriptMatchesAddress(addr, spkJson));
    if (receiveIdx >= 0) walletState.usedReceiveIndices.add(receiveIdx);
    const changeIdx = wallet.change_addresses.findIndex(addr => scriptMatchesAddress(addr, spkJson));
    if (changeIdx >= 0) walletState.usedChangeIndices.add(changeIdx);
}

function recordFirstSnapshot(currentUtxos, now) {
    // First snapshot — record all existing UTXOs as initial balance
    for (const u of currentUtxos) {
        walletState.historyEntries.push(makeHistoryEntry('in', u, now));
    }
}

function recordIncoming(currentUtxos, prevKeys, now) {
    for (const u of currentUtxos) {
        if (!prevKeys.has(utxoKey(u))) {
            walletState.historyEntries.unshift(makeHistoryEntry('in', u, now));
        }
    }
}

function recordSpent(prevUtxos, currKeys, now, wallet) {
    // Gone UTXOs = spent (outgoing) — also mark the address as "used"
    for (const u of prevUtxos) {
        if (!currKeys.has(utxoKey(u))) {
            walletState.historyEntries.unshift(makeHistoryEntry('out', u, now));
            if (wallet) markSpentAddressUsed(wallet, u);
        }
    }
}

export function trackUtxoChangesAndUsed(currentUtxos) {
    const now = Date.now();

    if (!networkState.utxoSnapshot) {
        recordFirstSnapshot(currentUtxos, now);
    } else {
        const prevKeys = new Set(networkState.utxoSnapshot.map(utxoKey));
        const currKeys = new Set(currentUtxos.map(utxoKey));
        recordIncoming(currentUtxos, prevKeys, now);
        const wallet = walletSession.hasWallet() ? walletSession.current() : null;
        recordSpent(networkState.utxoSnapshot, currKeys, now, wallet);
    }

    trimHistoryEntries();
    networkState.utxoSnapshot = currentUtxos;
}
