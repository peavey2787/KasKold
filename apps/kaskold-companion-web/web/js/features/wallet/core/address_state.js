import { networkState, walletSession, walletState } from '../../../app/state/index.js';
import { GAP_EXPAND_CHANGE, GAP_EXPAND_RECEIVE } from '../../../core/config/network.js';
import { extend_addresses } from '../../../wasm/api.js';

function standardChangeReservations() {
    if (!(walletState.standardChangeReservations instanceof Map)) {
        walletState.standardChangeReservations = new Map();
    }
    return walletState.standardChangeReservations;
}

function reservedChangeIndices() {
    return standardChangeReservations().keys();
}

function observedChangeIndices() {
    return new Set([
        ...(walletState.fundedChangeIndices || []),
        ...(walletState.usedChangeIndices || []),
    ]);
}

function outputMatchesReservation(output, index, address) {
    if (!output || output.address !== address) return false;
    if (Number(output.derivation_branch) !== 1 || Number(output.derivation_index) !== index) return false;
    try { return BigInt(output.amount_sompi) > 0n; } catch (_) { return false; }
}

export function reserveStandardChangeFromSummary(walletJson, summary) {
    let wallet;
    try { wallet = JSON.parse(walletJson); } catch (_) { return null; }
    const index = Number(wallet?.next_change_index);
    if (!Number.isSafeInteger(index) || index < 0) return null;
    const address = wallet?.change_addresses?.[index];
    if (!address) return null;
    const hasChange = (summary?.outputs || []).some(output => outputMatchesReservation(output, index, address));
    if (!hasChange) return null;
    standardChangeReservations().set(index, { address, status: 'pending' });
    return index;
}

export function standardChangeReservationMatchesSummary(index, summary) {
    if (!Number.isSafeInteger(index) || index < 0) return false;
    const reservation = standardChangeReservations().get(index);
    if (!reservation) return false;
    return (summary?.outputs || []).some(output => outputMatchesReservation(output, index, reservation.address));
}

export function releasePendingStandardChange(index) {
    if (!Number.isSafeInteger(index) || index < 0) return false;
    const reservations = standardChangeReservations();
    const reservation = reservations.get(index);
    if (!reservation || reservation.status !== 'pending') return false;
    reservations.delete(index);
    return true;
}

export function markStandardChangeBroadcast(index) {
    if (!Number.isSafeInteger(index) || index < 0) return false;
    const reservation = standardChangeReservations().get(index);
    if (!reservation) return false;
    reservation.status = 'broadcast';
    return true;
}

export function reconcileStandardChangeReservations() {
    const observed = observedChangeIndices();
    const reservations = standardChangeReservations();
    for (const index of [...reservations.keys()]) {
        if (observed.has(index)) reservations.delete(index);
    }
}

export function clearStandardChangeReservations() {
    standardChangeReservations().clear();
}

function receiveSkipSet() {
    return new Set([...(walletState.fundedReceiveIndices || []), ...(walletState.usedReceiveIndices || [])]);
}

function changeSkipSet() {
    return new Set([
        ...(walletState.fundedChangeIndices || []),
        ...(walletState.usedChangeIndices || []),
        ...reservedChangeIndices(),
    ]);
}

// First index in the address list that is not skipped, or -1 when all are skipped.
function firstUnskippedIndex(addresses, skipSet) {
    for (let i = 0; i < addresses.length; i++) {
        if (!skipSet.has(i)) return i;
    }
    return -1;
}

function nextUnusedIndex(addresses, skipSet) {
    const i = firstUnskippedIndex(addresses, skipSet);
    return i === -1 ? addresses.length - 1 : i;
}

function addressListGrew(addresses, before, extra) {
    return extra === 0 || addresses.length >= before + extra;
}

function applyAddressExpansion(beforeReceive, beforeChange, extraRcv, extraChg) {
    walletSession.replace(extend_addresses(walletSession.json(), extraRcv, extraChg, networkState.network));
    const expandedWallet = walletSession.current();
    const complete = addressListGrew(expandedWallet.receive_addresses, beforeReceive, extraRcv)
        && addressListGrew(expandedWallet.change_addresses, beforeChange, extraChg);
    if (!complete) {
        console.warn('[Companion] Address expansion returned unchanged or incomplete address lists');
        return false;
    }
    console.log(`[Companion] Gap expanded: +${extraRcv} receive, +${extraChg} change`);
    return true;
}

export function expandAddressesIfNeeded() {
    if (!walletSession.hasWallet()) return false;
    const wallet = walletSession.current();
    const needReceive = firstUnskippedIndex(wallet.receive_addresses, receiveSkipSet()) === -1;
    const needChange = firstUnskippedIndex(wallet.change_addresses, changeSkipSet()) === -1;
    if (!needReceive && !needChange) return false;

    const extraRcv = needReceive ? GAP_EXPAND_RECEIVE : 0;
    const extraChg = needChange ? GAP_EXPAND_CHANGE : 0;
    try {
        return applyAddressExpansion(
            wallet.receive_addresses.length, wallet.change_addresses.length, extraRcv, extraChg);
    } catch (e) {
        console.error('[Companion] Address expansion failed:', e);
        return false;
    }
}

function getNextChangeIndex() {
    if (!walletSession.hasWallet()) return 0;
    expandAddressesIfNeeded();
    const wallet = walletSession.current();
    return nextUnusedIndex(wallet.change_addresses, changeSkipSet());
}

export function getNextReceiveIndex() {
    if (!walletSession.hasWallet()) return 0;
    expandAddressesIfNeeded();
    const wallet = walletSession.current();
    return nextUnusedIndex(wallet.receive_addresses, receiveSkipSet());
}

export function walletWithFreshIndices() {
    if (!walletSession.hasWallet()) return walletSession.json();
    expandAddressesIfNeeded();
    const w = { ...walletSession.current() };
    w.next_change_index = getNextChangeIndex();
    w.next_receive_index = getNextReceiveIndex();
    return JSON.stringify(w);
}
