import { covenantState, navigationState, networkState } from '../../../../../app/state/index.js';
import { piggyBreakStatus, piggyStatusBanner } from '../../../../../app/events/contracts/covenant_creation/result_actions.js';
import { hideLoading, showLoading } from '../../../../../app/navigation.js';
import { fetchCurrentDaa } from '../../../../../core/node/daa.js';
import { resolveNodeUrl } from '../../../../../core/node/resolver.js';
import { toast } from '../../../../../core/ui/toast.js';
import { getCovFee } from '../../../payload/state.js';
import { pickThread } from '../spend_paths.js';
import { encodeCovenantSpend } from '../spend_document.js';
import { openPsktReview } from '../../../../transactions/pskt_multisig/review.js';
import { create_covenant_owner_spend, create_global_spending_limit_withdraw, fetch_utxos_for_address_js, thread_withdraw_fee } from '../../../../../wasm/api.js';

import { byId } from '../../../../../core/dom.js';
import { formatDaaDuration } from '../../../../../core/format.js';
import { kasToSompi, sompiToKasString } from '../../../../../core/amounts.js';
import { exactJsonStringify, exactUnsigned } from '../../../../../core/exact.js';
import { THREAD_FEE_FLOOR, ceilRateToInteger } from '../../../../../core/fee_math.js';

const AMBIGUOUS_THREAD_MSG = 'Multiple covenant-tagged UTXOs at this address and no known thread id, cannot safely pick the thread.';
const UNAVAILABLE_THREAD_MSG = 'Thread covenant_id unavailable from the node. The continuation must reuse the thread id; the node must serve version-2 UTXO entries.';

// CLTV-only covenant types: the owner path IS the timelock branch.
const CLTV_OWNER_REASONS = new Map([
    ['merkle-whitelist', 'only whitelisted spends are valid'],
    ['commit-reveal', 'only the reveal path is valid'],
    ['payjoin', 'only the joint-spend path is valid'],
    ['oracle-v1', 'only an oracle-attested beneficiary claim is valid'],
]);

const PARTIAL_OWNER_REFUSALS = new Map([
    ['commit-reveal', { message: 'Commit-Reveal owner refund is full-only. Clear the amount to refund the whole commitment. A partial refund would leave a remainder that breaks the reveal and refund paths.', durationMs: 8000 }],
    ['oracle-v1', { message: 'Oracle owner refund is full-only after the timeout. Clear the amount to refund the whole covenant balance.', durationMs: 8000 }],
]);

const REQUIRED_OWNER_FIELDS = [
    ['covAddr', 'Enter covenant P2SH address'],
    ['redeemHex', 'Enter redeem script hex'],
    ['destAddr', 'Enter destination address'],
];

function failSpend(message, durationMs) {
    toast(message, 'error', durationMs);
    hideLoading();
}

function lastCovenantIdHex() {
    return covenantState.lastCovenantResult && covenantState.lastCovenantResult.covenant_id_hex;
}

function isKnownCovenantId(idHex) {
    return !!(idHex && !/^0+$/.test(idHex));
}

function threadSelectionError(pick) {
    if (pick.ambiguous) return AMBIGUOUS_THREAD_MSG;
    if (isKnownCovenantId(lastCovenantIdHex())) {
        return 'Thread closed. The remaining ' + sompiToKasString(pick.externalSompi) + ' KAS is external and cannot be spent through the limit.';
    }
    return UNAVAILABLE_THREAD_MSG;
}

function readSpendCapSompi() {
    const maxWithdraw = covenantState.lastCovenantResult && covenantState.lastCovenantResult.max_withdraw_sompi;
    return maxWithdraw ? BigInt(maxWithdraw) : 0n;
}

// CSV cooldown: returns the remaining ETA string while the thread UTXO is still
// aging, or null when a withdrawal may proceed.
async function cooldownEtaIfActive(thread) {
    const cooldownDaa = exactUnsigned(covenantState.lastCovenantResult?.cooldown_daa ?? 0n, 'cooldown DAA');
    if (!(cooldownDaa > 0n)) return null;
    const threadDaa = exactUnsigned(thread.block_daa_score ?? 0n, 'thread DAA');
    if (!(threadDaa > 0n)) return null;
    const curDaa = await fetchCurrentDaa();
    const matureDaa = threadDaa + cooldownDaa;
    if (!(curDaa > 0n && curDaa < matureDaa)) return null;
    return formatDaaDuration(matureDaa - curDaa);
}

function capExceededMessage(withdrawSompi, threadAmt, capSompi) {
    const capK = sompiToKasString(capSompi);
    // A sweep-all (close) is valid only when the whole balance is <= cap.
    if (withdrawSompi >= threadAmt) {
        return 'Balance (' + sompiToKasString(threadAmt) + ' KAS) is over the per-spend cap of ' + capK + ' KAS, so it cannot be swept in one TX. Withdraw ' + capK + ' KAS or less.';
    }
    return 'Per-spend cap is ' + capK + ' KAS. Withdraw that or less.';
}

function withdrawalAmountError(withdrawSompi, threadAmt, capSompi) {
    if (withdrawSompi > threadAmt) {
        return { message: 'Amount exceeds the thread balance (' + sompiToKasString(threadAmt) + ' KAS).', durationMs: undefined };
    }
    if (capSompi > 0n && withdrawSompi > capSompi) {
        return { message: capExceededMessage(withdrawSompi, threadAmt, capSompi), durationMs: 5000 };
    }
    return null;
}

// Returns the withdrawal fee, or null when the remainder is too small (storage mass).
function computeWithdrawFee(threadAmt, withdrawSompi) {
    const feeRate = ceilRateToInteger(networkState.lastFeeEstimate?.normal_sompi_per_gram || 1);
    try {
        return thread_withdraw_fee(threadAmt, withdrawSompi, THREAD_FEE_FLOOR, feeRate);
    } catch {
        return null;
    }
}

async function buildGlobalSpendingLimitOwnerSpend(request) {
    const { covAddr, redeemHex, destAddr, amountStr, isPartial, wsUrl } = request;
    // Single-thread global limit. Empty amount = sweep/close the whole thread
    // (allowed only when balance <= cap, enforced on-chain by the script's ELSE
    // branch); otherwise a capped partial withdrawal leaving a continuation.
    // The continuation reuses the thread's own covenant id (G), read from the node.
    const threadRaw = JSON.parse(await fetch_utxos_for_address_js(covAddr, wsUrl));
    if (!threadRaw.length) { failSpend('No UTXO at the covenant address'); return; }
    const pick = pickThread(threadRaw, lastCovenantIdHex());
    const thread = pick.thread; // the tagged thread, selected by covenant_id (not size)
    if (!thread) { failSpend(threadSelectionError(pick), 6500); return; }
    const threadAmt = BigInt(thread.amount);
    const gId = thread.covenant_id || ''; // thread's own covenant id (G)
    const cooldownEta = await cooldownEtaIfActive(thread);
    if (cooldownEta !== null) {
        hideLoading();
        toast('Cooldown not elapsed. Next withdrawal in ~' + cooldownEta + '. An early spend is rejected by the node.', 'error', 5000);
        return;
    }
    // Empty -> sweep the whole balance (close); otherwise the entered amount.
    const withdrawSompi = isPartial ? kasToSompi(amountStr) : threadAmt;
    const amountError = withdrawalAmountError(withdrawSompi, threadAmt, readSpendCapSompi());
    if (amountError) { failSpend(amountError.message, amountError.durationMs); return; }
    const glFee = computeWithdrawFee(threadAmt, withdrawSompi);
    if (glFee === null) {
        failSpend('That withdrawal leaves too small a remainder (storage mass). Pick a different amount.');
        return;
    }
    console.log('[Companion] Global limit withdraw: thread ' + thread.tx_id.substring(0, 16) + ':' + thread.index + ' = ' + sompiToKasString(threadAmt) + ' KAS, withdraw=' + sompiToKasString(withdrawSompi) + ' KAS, fee=' + glFee.toString());
    return await create_global_spending_limit_withdraw(covAddr, destAddr, redeemHex, gId, withdrawSompi, glFee, exactJsonStringify([thread]));
}

// Current DAA from the node, falling back to the last known DAA when the node is unreachable.
async function currentDaaOrLastKnown() {
    let daa = 0n;
    try { daa = await fetchCurrentDaa(); } catch { /* With no node answer the DAA stays at zero, so time locks read as still locked. */ }
    if (daa === 0n && typeof covenantState._lastKnownDaa !== 'undefined') {
        daa = exactUnsigned(covenantState._lastKnownDaa ?? 0n, 'last known DAA');
    }
    return daa;
}

// Returns { eta, reason } when a CLTV-only owner reclaim is still timelocked, else null.
async function timelockBlockFor(covType) {
    const reason = CLTV_OWNER_REASONS.get(covType);
    if (!reason) return null;
    const locktimeDaa = exactUnsigned(covenantState.lastCovenantResult?.locktime_daa ?? 0n, 'owner locktime DAA');
    if (!(locktimeDaa > 0n)) return null;
    const daa = await currentDaaOrLastKnown();
    if (!(daa > 0n && daa < locktimeDaa)) return null;
    return { eta: formatDaaDuration(locktimeDaa - daa), reason };
}

function reportTimelockedReclaim({ eta, reason }) {
    try {
        piggyStatusBanner({
            text: 'Owner reclaim NOT available yet: timelock matures in ~' + eta +
                  '. Until then ' + reason + ' — a reclaim TX would be rejected on-chain.',
            color: 'var(--error, #f44336)'
        });
    } catch { /* The toast below reports the same refusal. */ }
    hideLoading();
    toast('Owner reclaim is timelocked for ~' + eta + ' more. The node would reject this TX.', 'error', 7500);
}

async function checkPiggyBreak(utxos, sweepFee) {
    const totalCheck = utxos.reduce((s, u) => s + BigInt(u.amount), 0n);
    const st = await piggyBreakStatus(totalCheck, sweepFee);
    try { piggyStatusBanner(st); } catch { /* The banner is advisory; the status is returned to the caller. */ }
    return st;
}

async function buildOwnerSweep(request) {
    const { covAddr, redeemHex, destAddr, covType, fee, wsUrl } = request;
    // Sweep all — use existing WASM function. Scale the fee to the UTXO
    // count so a multi-UTXO sweep (e.g. a vault/DMS funded several times)
    // is not rejected for compute mass.
    let branch = CLTV_OWNER_REASONS.has(covType) ? 'owner-time' : '';
    // Block the doomed TX with a banner before the CLTV-only timelock matures.
    const timelockBlock = await timelockBlockFor(covType);
    if (timelockBlock) { reportTimelockedReclaim(timelockBlock); return; }
    let sweepFee = fee;
    try {
        const wsCheck = await resolveNodeUrl();
        const utxosCheck = JSON.parse(await fetch_utxos_for_address_js(covAddr, wsCheck));
        sweepFee = getCovFee(utxosCheck.length || 1);
        // Piggy break gate: refuse to build a TX that cannot pass on-chain.
        if (covType === 'additive' && covenantState.lastCovenantResult) {
            const st = await checkPiggyBreak(utxosCheck, sweepFee);
            if (!st.canBreak) {
                hideLoading();
                toast(st.text, 'error', 7500);
                return;
            }
            if (!st.goalMet && st.deadlinePassed) {
                branch = 'owner-time';
                console.log('[Companion] Piggy break: using deadline (time) path');
            }
        }
    } catch { /* With no status the default branch is built and the node judges the spend. */ }
    return await create_covenant_owner_spend(covAddr, destAddr, redeemHex, sweepFee, wsUrl, branch);
}

async function buildPartialOwnerSpend(request) {
    const { covAddr, redeemHex, destAddr, amountStr, fee, wsUrl } = request;
    // Partial spend — build PSKB in JS with change back to covenant
    const sendSompi = kasToSompi(amountStr);
    const utxosJson = await fetch_utxos_for_address_js(covAddr, wsUrl);
    const utxos = JSON.parse(utxosJson);
    if (!utxos.length) throw 'No UTXOs at covenant address';

    const total = utxos.reduce((s, u) => s + BigInt(u.amount), 0n);
    if (total < sendSompi + fee) throw 'Balance too low: ' + sompiToKasString(total) + ' KAS available';

    const change = total - sendSompi - fee;
    const outputs = [{ address: destAddr, amount: String(sendSompi) }];
    if (change > 0n) outputs.push({ address: covAddr, amount: String(change) });

    // Partial owner reclaim always spends the immediate branch (the
    // time-locked owner break is full-sweep only), so the TX must be
    // final: locktime 0. The script's CLTV lives in the beneficiary
    // branch; stamping it here would make the node reject the TX as
    // "input #0 is not finalized" before the timeout.
    return encodeCovenantSpend({ covAddr, redeemHex, path: 'owner', utxos, outputs, txVersion: 0 });
}

function readOwnerSpendInputs() {
    return {
        covAddr: byId('cov-owner-addr').value.trim(),
        redeemHex: byId('cov-owner-script').value.trim(),
        destAddr: byId('cov-owner-dest').value.trim(),
        amountStr: byId('cov-owner-amount').value.trim(),
    };
}

function missingOwnerFieldMessage(inputs) {
    const missing = REQUIRED_OWNER_FIELDS.find(([key]) => !inputs[key]);
    return missing ? missing[1] : null;
}

// Returns false for an empty amount (sweep), true for a positive partial,
// or null when the amount does not parse.
function parseIsPartial(amountStr) {
    if (!amountStr) return false;
    try {
        return kasToSompi(amountStr) > 0n;
    } catch {
        return null;
    }
}

function readCovOwnerType() {
    const panel = byId('cov-owner-panel');
    return panel ? (panel.dataset.covOwnerType || '') : '';
}

function partialRefusalFor(covType, isPartial) {
    if (!isPartial) return null;
    return PARTIAL_OWNER_REFUSALS.get(covType) ?? null;
}

function buildOwnerSpendPskb(covType, isPartial, request) {
    if (covType === 'global-spending-limit') return buildGlobalSpendingLimitOwnerSpend(request);
    if (!isPartial) return buildOwnerSweep(request);
    return buildPartialOwnerSpend(request);
}

export async function handleCovOwnerSpend() {
    covenantState._covPayloadHex = '';
    const inputs = readOwnerSpendInputs();
    const fieldError = missingOwnerFieldMessage(inputs);
    if (fieldError) { toast(fieldError, 'error'); return; }

    const isPartial = parseIsPartial(inputs.amountStr);
    if (isPartial === null) { toast('Enter a valid KAS amount with at most 8 decimal places', 'error'); return; }
    const covType = readCovOwnerType();
    const refusal = partialRefusalFor(covType, isPartial);
    if (refusal) { toast(refusal.message, 'error', refusal.durationMs); return; }

    showLoading('Building owner-spend PSKB...');
    try {
        const fee = getCovFee();
        const wsUrl = await resolveNodeUrl();
        const request = { ...inputs, isPartial, covType, fee, wsUrl };
        const pskbHex = await buildOwnerSpendPskb(covType, isPartial, request);
        if (!pskbHex) return;

        hideLoading();
        console.log('[Companion] Covenant owner-spend PSKB: ' + pskbHex.length + ' hex chars' + (isPartial ? ' (partial)' : ' (sweep)'));
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Owner spend failed: ' + e, 'error', 5000);
        console.error('[Companion] Owner spend error:', e);
    }
}
