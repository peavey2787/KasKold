import { covenantState, navigationState, networkState } from '../../../../../app/state/index.js';
import { hideLoading, showLoading } from '../../../../../app/navigation.js';
import { fetchCurrentDaa } from '../../../../../core/node/daa.js';
import { resolveNodeUrl } from '../../../../../core/node/resolver.js';
import { toast } from '../../../../../core/ui/toast.js';
import { getCovFee, savingsClaimKeyPosition } from '../../../payload/state.js';
import { pickThread } from '../spend_paths.js';
import { openPsktReview } from '../../../../transactions/pskt_multisig/review.js';
import { create_covenant_beneficiary_spend, create_covenant_timelocked_savings_claim, create_global_allowance_withdraw, fetch_utxos_for_address_js, thread_withdraw_fee } from '../../../../../wasm/api.js';

import { byId } from '../../../../../core/dom.js';
import { formatDaaDuration } from '../../../../../core/format.js';
import { kasToSompi, sompiToKasString } from '../../../../../core/amounts.js';
import { exactJsonStringify, exactUnsigned } from '../../../../../core/exact.js';
import { THREAD_FEE_FLOOR, ceilRateToInteger } from '../../../../../core/fee_math.js';

function readBeneficiaryFields() {
    const fields = {
        type: byId('cov-beneficiary-panel').dataset.covBeneType || '',
        covenantAddress: byId('cov-bene-addr').value.trim(),
        redeemScriptHex: byId('cov-bene-script').value.trim(),
        destinationAddress: byId('cov-bene-dest').value.trim(),
    };
    if (!fields.covenantAddress) throw new Error('Enter covenant P2SH address');
    if (!fields.redeemScriptHex) throw new Error('Enter redeem script hex');
    if (!fields.destinationAddress) throw new Error('Enter destination address');
    return fields;
}

function selectAllowanceThread(utxos) {
    const knownId = covenantState.lastCovenantResult?.covenant_id_hex;
    const selection = pickThread(utxos, knownId);
    if (selection.thread) return selection.thread;
    const hasKnownId = !!(knownId && !/^0+$/.test(knownId));
    if (selection.ambiguous) {
        throw new Error('Multiple covenant-tagged UTXOs and no known thread id, cannot safely pick the thread.');
    }
    if (hasKnownId) {
        throw new Error('Thread closed. The remaining ' + sompiToKasString(selection.externalSompi) +
            ' KAS is external; the owner can reclaim it, the beneficiary cannot withdraw it.');
    }
    throw new Error('Thread covenant_id unavailable from the node (need version-2 UTXO entries).');
}

function assertAllowanceStarted(startDaa, currentDaa) {
    if (startDaa > 0n && currentDaa < startDaa) {
        const eta = formatDaaDuration(startDaa - currentDaa);
        throw new Error('Not started yet. Withdrawals begin in ~' + eta + '. An early spend is rejected by the node.');
    }
}

function assertAllowanceCooldownElapsed(cooldownDaa, threadDaa, currentDaa) {
    if (cooldownDaa > 0n && threadDaa > 0n && currentDaa < threadDaa + cooldownDaa) {
        const eta = formatDaaDuration(threadDaa + cooldownDaa - currentDaa);
        throw new Error('Cooldown not elapsed. Next withdrawal in ~' + eta + '. An early spend is rejected by the node.');
    }
}

async function enforceAllowanceTiming(thread) {
    const startDaa = exactUnsigned(covenantState.lastCovenantResult?.start_daa ?? 0n, 'start DAA');
    const cooldownDaa = exactUnsigned(covenantState.lastCovenantResult?.cooldown_daa ?? 0n, 'cooldown DAA');
    if (startDaa === 0n && cooldownDaa === 0n) return;
    const currentDaa = await fetchCurrentDaa();
    if (currentDaa <= 0n) return;
    assertAllowanceStarted(startDaa, currentDaa);
    const threadDaa = exactUnsigned(thread.block_daa_score ?? 0n, 'thread DAA');
    assertAllowanceCooldownElapsed(cooldownDaa, threadDaa, currentDaa);
}

function calculateAllowanceWithdrawal(threadAmount) {
    const amountText = byId('cov-bene-amount')?.value.trim() || '';
    let withdrawSompi = threadAmount;
    if (amountText) {
        withdrawSompi = kasToSompi(amountText);
        if (withdrawSompi <= 0n) throw new Error('Enter a positive amount');
    }
    const capSompi = BigInt(covenantState.lastCovenantResult?.max_withdraw_sompi || 0);
    if (withdrawSompi > threadAmount) {
        throw new Error('Amount exceeds the thread balance (' + sompiToKasString(threadAmount) + ' KAS).');
    }
    if (capSompi > 0n && withdrawSompi > capSompi) {
        const capKas = sompiToKasString(capSompi);
        const message = withdrawSompi >= threadAmount
            ? 'Balance (' + sompiToKasString(threadAmount) + ' KAS) is over the per-spend cap of ' +
                capKas + ' KAS, so it cannot be swept in one TX. Withdraw ' + capKas + ' KAS or less.'
            : 'Per-spend cap is ' + capKas + ' KAS. Withdraw that or less.';
        throw new Error(message);
    }
    return withdrawSompi;
}

function calculateAllowanceFee(threadAmount, withdrawSompi) {
    const feeRate = ceilRateToInteger(networkState.lastFeeEstimate?.normal_sompi_per_gram || 1);
    return thread_withdraw_fee(threadAmount, withdrawSompi, THREAD_FEE_FLOOR, feeRate);
}

async function handleGlobalAllowanceWithdrawal(fields) {
    showLoading('Building global allowance withdraw PSKB...');
    try {
        const websocketUrl = await resolveNodeUrl();
        const utxos = JSON.parse(await fetch_utxos_for_address_js(fields.covenantAddress, websocketUrl));
        if (!utxos.length) throw new Error('No UTXO at the covenant address');
        const thread = selectAllowanceThread(utxos);
        await enforceAllowanceTiming(thread);
        const threadAmount = BigInt(thread.amount);
        const withdrawSompi = calculateAllowanceWithdrawal(threadAmount);
        const fee = calculateAllowanceFee(threadAmount, withdrawSompi);
        const pskbHex = await create_global_allowance_withdraw(
            fields.covenantAddress,
            fields.destinationAddress,
            fields.redeemScriptHex,
            thread.covenant_id || '',
            withdrawSompi,
            fee,
            exactJsonStringify([thread]),
        );
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } catch (error) {
        toast('Global allowance withdraw failed: ' + error, 'error', 5000);
    } finally {
        hideLoading();
    }
}

async function ensureStandardClaimUnlocked(type, locktime, utxos) {
    const currentDaa = await fetchCurrentDaa();
    if (currentDaa <= 0n) return;
    if (type === 'timelocked-savings') {
        const unlockDaa = exactUnsigned(locktime || '0', 'claim locktime DAA');
        if (unlockDaa > 0n && currentDaa < unlockDaa) {
            const eta = formatDaaDuration(unlockDaa - currentDaa);
            throw new Error('Still locked. Unlocks in ~' + eta + '. An early claim is rejected by the node.');
        }
        return;
    }
    if (covenantState.lastCovenantResult?.type !== 'dms') return;
    const inactivityDaa = exactUnsigned(covenantState.lastCovenantResult.inactivity_daa ?? 0n, 'inactivity DAA');
    if (inactivityDaa === 0n || !utxos.length) return;
    const newestDaa = utxos.reduce((latest, utxo) => {
        const daa = exactUnsigned(utxo.block_daa_score ?? 0n, 'UTXO DAA');
        return daa > latest ? daa : latest;
    }, 0n);
    const unlockDaa = newestDaa + inactivityDaa;
    if (currentDaa < unlockDaa) {
        const eta = formatDaaDuration(unlockDaa - currentDaa);
        throw new Error('Still locked. The inactivity period has not elapsed for all vault UTXOs. ' +
            'The heir can sweep everything in ~' + eta + '. An early claim is rejected by the node.');
    }
}

async function handleStandardBeneficiarySpend(fields) {
    const locktime = byId('cov-bene-locktime').value.trim();
    const isDms = covenantState.lastCovenantResult?.type === 'dms';
    if (!isDms && (!/^\d+$/.test(locktime) || exactUnsigned(locktime, 'claim locktime DAA') === 0n)) {
        throw new Error('Enter locktime DAA score');
    }
    showLoading('Building beneficiary-spend PSKB...');
    try {
        const websocketUrl = await resolveNodeUrl();
        let utxos = [];
        try {
            utxos = JSON.parse(await fetch_utxos_for_address_js(fields.covenantAddress, websocketUrl));
        } catch { /* The claim build below reads the UTXOs itself and reports a failure. */ }
        const fee = getCovFee(utxos.length || 1);
        await ensureStandardClaimUnlocked(fields.type, locktime, utxos);
        const pskbHex = fields.type === 'timelocked-savings'
            ? await create_covenant_timelocked_savings_claim(
                fields.covenantAddress,
                fields.destinationAddress,
                fields.redeemScriptHex,
                BigInt(locktime),
                savingsClaimKeyPosition(),
                fee,
                websocketUrl,
            )
            : await create_covenant_beneficiary_spend(
                fields.covenantAddress,
                fields.destinationAddress,
                fields.redeemScriptHex,
                BigInt(locktime),
                fee,
                websocketUrl,
            );
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } finally {
        hideLoading();
    }
}

export async function handleCovBeneficiarySpend() {
    covenantState._covPayloadHex = '';
    let fields;
    try {
        fields = readBeneficiaryFields();
    } catch (error) {
        toast(error.message, 'error');
        return;
    }
    if (fields.type === 'global-allowance') {
        await handleGlobalAllowanceWithdrawal(fields);
        return;
    }
    try {
        await handleStandardBeneficiarySpend(fields);
    } catch (error) {
        toast('Beneficiary spend failed: ' + error, 'error', 5000);
        console.error('[Companion] Beneficiary spend error:', error);
    }
}
