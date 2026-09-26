import { networkState, scannerState, uiState, walletSession, walletState } from '../../../app/state/index.js';
import { setStatus } from '../../../app/navigation.js';
import { fetchCurrentDaa } from '../../../core/node/daa.js';
import { resolvePublicNode } from '../../../core/node/resolver.js';
import { toast } from '../../../core/ui/toast.js';
import { renderNodeUnavailable } from '../../../core/ui/connectivity_status.js';
import { expandAddressesIfNeeded, reconcileStandardChangeReservations } from './address_state.js';
import { fetchAddressHistory } from './history.js';
import { trackUtxoChangesAndUsed } from '../tools.js';
import { fetch_balance, fetch_utxos } from '../../../wasm/api.js';

import { byId } from '../../../core/dom.js';
import { exactUnsigned } from '../../../core/exact.js';
import { formatSompiGrouped, sompiToKasRoundedGrouped } from '../../../core/amounts.js';

const BALANCE_RECONNECT_ATTEMPTS = 3;
const NODE_RETRY_DELAYS_MS = Object.freeze([500, 1_000]);
let nodeFailureNotified = false;
let balanceDisplayUnit = 'kas';
let postBroadcastRefreshGeneration = 0;

function sleep(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
}


export async function refreshBalanceAfterBroadcast() {
    const generation = ++postBroadcastRefreshGeneration;
    // A node can acknowledge broadcast before its UTXO index reflects the new
    // change output. Poll briefly so balance metadata converges without making
    // the user mash Refresh or wait for the normal auto-refresh interval.
    for (const delay of [250, 1000, 2500, 5000, 8000, 15000, 30000]) {
        await sleep(delay);
        if (generation !== postBroadcastRefreshGeneration || !walletSession.hasWallet()) return;
        await refreshBalance();
    }
}

export function isRetryableNodeError(error) {
    const message = String(error).toLowerCase();
    return [
        'websocket',
        'timeout',
        'connection',
        'network',
        'resolver',
        'failed to fetch',
        'send failed',
        'invalid url',
        'node unavailable',
    ].some(fragment => message.includes(fragment));
}

function nodeUrlForAttempt(attempt, maxRetries) {
    if (networkState.customNodeUrl && attempt < maxRetries) {
        return Promise.resolve(networkState.customNodeUrl);
    }
    return resolvePublicNode();
}

export async function withNodeRetry(fn, maxRetries = 3, onRetry = null) {
    const attempts = Math.max(1, Number(maxRetries) || 1);
    let lastError;

    for (let attempt = 1; attempt <= attempts; attempt++) {
        try {
            const wsUrl = await nodeUrlForAttempt(attempt, attempts);
            return await fn(wsUrl);
        } catch (error) {
            lastError = error;
            if (!isRetryableNodeError(error) || attempt >= attempts) throw error;
            onRetry?.({ attempt, maxRetries: attempts, error });
            await sleep(NODE_RETRY_DELAYS_MS[Math.min(attempt - 1, NODE_RETRY_DELAYS_MS.length - 1)]);
        }
    }

    throw lastError;
}

function renderStoredBalance() {
    const amount = byId('balance-kas');
    const unit = byId('btn-balance-unit-toggle');
    if (!amount || !unit) return;

    const raw = amount.dataset.balanceSompi;
    if (!raw) {
        unit.textContent = balanceDisplayUnit === 'kas' ? 'KAS' : 'Sompi';
        return;
    }

    const totalSompi = exactUnsigned(raw, 'wallet balance');
    const showingKas = balanceDisplayUnit === 'kas';
    amount.textContent = showingKas
        ? sompiToKasRoundedGrouped(totalSompi, 2)
        : formatSompiGrouped(totalSompi);
    unit.textContent = showingKas ? 'KAS' : 'Sompi';
    const alternate = showingKas ? 'Sompi' : 'KAS';
    unit.setAttribute('aria-label', `Show balance in ${alternate}`);
    unit.setAttribute('title', `Show balance in ${alternate}`);
}

export function toggleBalanceDisplayUnit() {
    const amount = byId('balance-kas');
    if (!amount?.dataset.balanceSompi) return;
    balanceDisplayUnit = balanceDisplayUnit === 'kas' ? 'sompi' : 'kas';
    renderStoredBalance();
}

export function resetBalanceDisplayUnit() {
    balanceDisplayUnit = 'kas';
    renderStoredBalance();
}

function renderBalance(result) {
    const totalSompi = exactUnsigned(result.total_sompi, 'wallet total balance');
    const amount = byId('balance-kas');
    amount.dataset.balanceSompi = totalSompi.toString();
    renderStoredBalance();
    byId('balance-info').textContent =
        `${result.utxo_count} UTXO${result.utxo_count !== 1 ? 's' : ''} across ${result.funded_addresses} address${result.funded_addresses !== 1 ? 'es' : ''}`;
}

function noteReconnectAttempt({ attempt, maxRetries }) {
    setStatus('connecting', `Reconnecting ${attempt}/${maxRetries}`);
}

function showNodeFailureAfterRetries() {
    renderNodeUnavailable();
    const balance = byId('balance-kas');
    const hasLastKnownBalance = Boolean(balance?.dataset.balanceSompi);
    if (!hasLastKnownBalance && balance) {
        balance.textContent = 'Unavailable';
        balance.dataset.balanceSompi = '';
    }
    byId('balance-info').textContent = hasLastKnownBalance
        ? `Unable to reconnect after ${BALANCE_RECONNECT_ATTEMPTS} attempts. Last known balance shown.`
        : `Unable to reconnect after ${BALANCE_RECONNECT_ATTEMPTS} attempts.`;
    if (!nodeFailureNotified) {
        toast(`Unable to reconnect to a Kaspa node after ${BALANCE_RECONNECT_ATTEMPTS} attempts`, 'error', 5000);
        nodeFailureNotified = true;
    }
}

export async function refreshBalance() {
    if (!walletSession.hasWallet() || scannerState.refreshing) return;
    scannerState.refreshing = true;

    try {
        const resultJson = await withNodeRetry(
            wsUrl => fetch_balance(walletSession.json(), wsUrl),
            BALANCE_RECONNECT_ATTEMPTS,
            noteReconnectAttempt,
        );
        const result = JSON.parse(resultJson);

        nodeFailureNotified = false;
        setStatus('online', 'Connected');
        renderBalance(result);

        walletState.fundedReceiveIndices = result.funded_receive_indices || [];
        walletState.fundedChangeIndices = result.funded_change_indices || [];
        reconcileStandardChangeReservations();

        // History tracking is supplemental. A transient node timeout must not
        // erase or replace a successfully fetched zero balance.
        try {
            const utxosJson = await withNodeRetry(
                wsUrl => fetch_utxos(walletSession.json(), wsUrl),
                BALANCE_RECONNECT_ATTEMPTS,
                noteReconnectAttempt,
            );
            const currentUtxos = JSON.parse(utxosJson);
            trackUtxoChangesAndUsed(currentUtxos);
            setStatus('online', 'Connected');
        } catch (_) {
            setStatus('online', 'Connected');
            // Keep the authoritative balance already rendered above. The next
            // background refresh will retry history without blocking the UI.
        }

        // Detect used addresses via api.kaspa.org (or custom REST server)
        // then expand gap limit if all addresses are occupied.
        //
        // If expansion actually added addresses, fetch balance again so
        // funds at the new indices (e.g. user's wallet has activity at
        // index 25+ but we only derived 0-19 initially) show up without
        // requiring a manual refresh. Cap the chain at 3 cycles total
        // to bound the work for very deep wallets.
        fetchAddressHistory().then(() => {
            reconcileStandardChangeReservations();
            const expanded = expandAddressesIfNeeded();
            if (expanded && (uiState._refreshExpansionDepth || 0) < 3) {
                uiState._refreshExpansionDepth = (uiState._refreshExpansionDepth || 0) + 1;
                refreshBalance().finally(() => {
                    // Reset depth once the chain settles
                    if (!scannerState.refreshing) uiState._refreshExpansionDepth = 0;
                });
            } else {
                uiState._refreshExpansionDepth = 0;
            }
        });

        // Fetch current DAA from node and display on dashboard
        try {
            const daa = await fetchCurrentDaa();
            if (daa > 0 && byId('balance-daa')) {
                byId('balance-daa').textContent = 'DAA ' + daa.toLocaleString();
            }
        } catch (_) {}
    } catch (_) {
        showNodeFailureAfterRetries();
    } finally {
        scannerState.refreshing = false;
    }
}
