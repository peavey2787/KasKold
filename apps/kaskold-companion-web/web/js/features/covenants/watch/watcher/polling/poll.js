import { covenantState, covenantWatcherState } from '../../../../../app/state/index.js';
import { sompiToKasFixed, sompiToKasString } from '../../../../../core/amounts.js';
import { exactUnsigned } from '../../../../../core/exact.js';
import { fetchCurrentDaa } from '../../../../../core/node/daa.js';
import { resolveNodeUrl } from '../../../../../core/node/resolver.js';
import { covSaveActive } from '../../../recovery/active.js';
import { pickThread } from '../../../spending/standard/spend_paths.js';
import { covWatcherStop } from './lifecycle.js';
import { isWatchedCovenantType } from '../types.js';
import { fetch_utxos_for_address_js } from '../../../../../wasm/api.js';
import { byId } from '../../../../../core/dom.js';
import { pollCovenantType } from './covenant_pollers.js';

function isGovernedType(type) {
    return type === 'global-spending-limit' || type === 'global-allowance';
}

function isNonZeroCovenantId(value) {
    return !!value && !/^0+$/.test(String(value));
}

function utxoCountLabel(count) {
    return count + ' UTXO' + (count !== 1 ? 's' : '');
}

function needsThreadIdentity(type) {
    return isGovernedType(type) && !isNonZeroCovenantId(covenantState.lastCovenantResult.covenant_id_hex);
}

function adoptCovenantId(covenantId) {
    covenantState.lastCovenantResult.covenant_id_hex = covenantId;
    const entry = covenantState.activeCovenants.find(c => c.address === covenantState.lastCovenantResult.address);
    if (entry && entry.covenant_id_hex !== covenantId) {
        entry.covenant_id_hex = covenantId;
        covSaveActive();
    }
}

function captureThreadIdentity(type, utxos) {
    if (!needsThreadIdentity(type)) return;
    const tagged = utxos.filter(u => u && isNonZeroCovenantId(u.covenant_id));
    if (tagged.length === 1) adoptCovenantId(String(tagged[0].covenant_id));
}

function plainBalanceText(utxos, total) {
    return sompiToKasString(total) + ' KAS (' + utxoCountLabel(utxos.length) + ')';
}

function governedBalanceText(type, utxos) {
    const picked = pickThread(utxos, covenantState.lastCovenantResult?.covenant_id_hex);
    const governed = picked.thread ? exactUnsigned(picked.thread.amount, 'thread amount') : 0n;
    let text = sompiToKasString(governed) + ' KAS';
    if (picked.externalSompi > 0n) {
        const word = type === 'global-spending-limit' ? 'stuck' : 'owner-reclaimable';
        text += ' (+' + sompiToKasString(picked.externalSompi) + ' KAS external, ' + word + ')';
    }
    return text;
}

function updateWatcherBalance(type, utxos, total) {
    const balance = byId('cov-result-balance');
    if (!balance) return;
    balance.textContent = plainBalanceText(utxos, total);
    if (isGovernedType(type)) balance.textContent = governedBalanceText(type, utxos);
    balance.style.display = '';
}

function activeWatchedType() {
    if (!covenantState.lastCovenantResult) return null;
    const type = covenantState.lastCovenantResult.type || '';
    return isWatchedCovenantType(type) ? type : null;
}

async function fetchWatchedUtxos() {
    const wsUrl = await resolveNodeUrl();
    const utxosJson = await fetch_utxos_for_address_js(covenantState.lastCovenantResult.address, wsUrl);
    return JSON.parse(utxosJson);
}

function sumUtxoAmounts(utxos) {
    return utxos.reduce((sum, utxo) => sum + exactUnsigned(utxo.amount, 'UTXO amount'), 0n);
}

function isWatcherActive(status) {
    return !!status && !!covenantWatcherState._covWatcherTimer;
}

async function buildPollState(type, total, status, utxos) {
    const locktime = exactUnsigned(covenantState.lastCovenantResult.locktime_daa ?? 0n, 'locktime DAA');
    const currentDaa = await fetchCurrentDaa();
    if (currentDaa > 0n) covenantState._lastKnownDaa = currentDaa;
    return { t: type, total, kas: sompiToKasFixed(total, 2), st: status, locktime, currentDaa, utxos };
}

function rememberOutpoint(utxos, total) {
    if (covenantWatcherState._covWatcherLastBalance === null && utxos.length > 0 && utxos[0].tx_id) {
        covenantWatcherState._covWatcherOutpoint = { txid: utxos[0].tx_id, index: utxos[0].index || 0 };
    }
    covenantWatcherState._covWatcherLastBalance = total;
}

async function pollWatchedType(type) {
    const utxos = await fetchWatchedUtxos();
    const total = sumUtxoAmounts(utxos);
    captureThreadIdentity(type, utxos);
    updateWatcherBalance(type, utxos, total);

    const status = byId('cov-watcher-status');
    if (!isWatcherActive(status)) return;
    const state = await buildPollState(type, total, status, utxos);
    if (await pollCovenantType(state)) return;
    rememberOutpoint(utxos, total);
}

export async function covWatcherPoll() {
    const type = activeWatchedType();
    if (type === null) { covWatcherStop(); return; }

    try {
        await pollWatchedType(type);
    } catch (_) {
        // Best-effort watcher poll. The next interval retries.
    }
}
