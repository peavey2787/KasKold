import { applyScriptTerms } from '../../../../model/script_terms.js';
import { setContent, span } from '../../../../../../core/ui/elements.js';
import { covenantState, covenantWatcherState } from '../../../../../../app/state/index.js';
import { exactUnsigned } from '../../../../../../core/exact.js';
import { formatDaaDuration } from '../../../../../../core/format.js';
import { sompiToKasFixed, sompiToKasString } from '../../../../../../core/amounts.js';
import { pickThread } from '../../../../spending/standard/spend_paths.js';
import { covWatcherStop } from '../lifecycle.js';
import { isFundsSpent } from './status_text.js';

function governedThreadState(utxos) {
    const picked = pickThread(
        utxos,
        covenantState.lastCovenantResult?.covenant_id_hex,
    );
    const threadSompi = picked.thread ? exactUnsigned(picked.thread.amount, 'thread amount') : 0n;
    const threadDaa = picked.thread
        ? exactUnsigned(picked.thread.block_daa_score ?? 0n, 'thread DAA score')
        : 0n;
    return { picked, threadSompi, threadDaa };
}

function externalFundsNote(sompi, disposition) {
    const exact = exactUnsigned(sompi ?? 0n, 'external amount');
    return exact > 0n
        ? [' ', span('u-text-warning', `(+${sompiToKasFixed(exact, 2)} KAS external, ${disposition})`)]
        : null;
}

function readyLabel(canDrain, governedKas, maxStr) {
    return span('u-text-teal', canDrain ? `✅ Ready to drain all ${governedKas} KAS` : `✅ Ready to withdraw${maxStr}`);
}

function withdrawalLimitLabel(maxSompi, canDrain) {
    if (canDrain) return ' (full drain)';
    return maxSompi > 0n ? ` (max ${sompiToKasString(maxSompi)} KAS)` : '';
}

function hasCooldownClock(cooldown, currentDaa) {
    return cooldown > 0n && currentDaa > 0n;
}

function notFundedStatus() {
    return '👁 0 KAS | Not funded';
}

function watchingStatus(governedKas, maxStr) {
    return `👁 ${governedKas} KAS | Watching...${maxStr}`;
}

function cooldownStatus({ threadDaa, cooldown, currentDaa, readyCanDrain, governedKas, maxStr }) {
    const remaining = threadDaa + cooldown - currentDaa;
    return remaining <= 0n
        ? readyLabel(readyCanDrain, governedKas, maxStr)
        : `👁 ~${formatDaaDuration(remaining)} until next withdraw${maxStr}`;
}

function isMatureThread(picked, cooldown, currentDaa, threadDaa) {
    return !!picked.thread && (cooldown === 0n || currentDaa === 0n || currentDaa >= threadDaa + cooldown);
}

function spendingLimitStatus(ctx) {
    const { threadSompi, matureSompi, canDrain, governedKas, maxStr, cooldown, currentDaa, threadDaa } = ctx;
    if (threadSompi === 0n) return notFundedStatus();
    if (matureSompi > 0n) return readyLabel(canDrain, governedKas, maxStr);
    if (hasCooldownClock(cooldown, currentDaa)) {
        return cooldownStatus({ threadDaa, cooldown, currentDaa, readyCanDrain: false, governedKas, maxStr });
    }
    return watchingStatus(governedKas, maxStr);
}

function allowanceStatus(ctx) {
    const { threadSompi, iAmOwner, startDaa, currentDaa, cooldown, threadDaa, canDrain, governedKas, maxStr } = ctx;
    if (threadSompi === 0n) return notFundedStatus();
    if (iAmOwner) return `👁 ${governedKas} KAS | Owner can reclaim anytime`;
    if (startDaa > 0n && currentDaa > 0n && currentDaa < startDaa) {
        return `👁 ${governedKas} KAS | Locked, ~${formatDaaDuration(startDaa - currentDaa)} until start`;
    }
    if (hasCooldownClock(cooldown, currentDaa)) {
        return cooldownStatus({ threadDaa, cooldown, currentDaa, readyCanDrain: canDrain, governedKas, maxStr });
    }
    return watchingStatus(governedKas, maxStr);
}

function spentFundsStatus(spender) {
    if (spender === 'heir') return span('u-text-teal', '✅ Beneficiary withdrew.');
    if (spender === 'owner') return span('u-text-warning', '⚠ Owner reclaimed all funds.');
    return null;
}

function reportFundsSpent(st) {
    const spender = covenantWatcherState._covWatcherSpendPath || 'unknown';
    const status = spentFundsStatus(spender);
    if (status) setContent(st, status);
    else st.textContent = '👁 Funds spent (0 KAS)';
    covWatcherStop();
    if (st) st.style.display = '';
}

export async function pollGlobalSpendingLimit(state) {
    const { st, currentDaa, utxos } = state;
    applyScriptTerms(covenantState.lastCovenantResult);
    const cooldown = exactUnsigned(covenantState.lastCovenantResult.cooldown_daa ?? 0n, 'cooldown DAA');
    const maxSompi = exactUnsigned(covenantState.lastCovenantResult.max_withdraw_sompi ?? 0n, 'withdrawal limit');
    const { picked, threadSompi, threadDaa } = governedThreadState(utxos);
    const mature = isMatureThread(picked, cooldown, currentDaa, threadDaa);
    const matureSompi = mature ? threadSompi : 0n;
    const canDrain = maxSompi > 0n && matureSompi > 0n && matureSompi <= maxSompi;
    const maxStr = withdrawalLimitLabel(maxSompi, canDrain);
    const extNote = externalFundsNote(picked.externalSompi, 'stuck');
    const governedKas = sompiToKasFixed(threadSompi, 2);

    st.style.color = '';
    const text = spendingLimitStatus({
        threadSompi, matureSompi, canDrain, governedKas, maxStr, cooldown, currentDaa, threadDaa,
    });
    setContent(st, text, extNote);
    return false;
}

export async function pollGlobalAllowance(state) {
    const { total, st, currentDaa, utxos } = state;
    applyScriptTerms(covenantState.lastCovenantResult);
    const result = covenantState.lastCovenantResult;
    const iAmOwner = result.role !== 'beneficiary';
    const cooldown = exactUnsigned(result.cooldown_daa ?? result.min_sequence ?? 0n, 'cooldown DAA');
    const maxSompi = exactUnsigned(result.max_withdraw_sompi ?? 0n, 'withdrawal limit');
    const startDaa = exactUnsigned(result.start_daa ?? 0n, 'start DAA');
    const { picked, threadSompi, threadDaa } = governedThreadState(utxos);
    const canDrain = maxSompi > 0n && threadSompi > 0n && threadSompi <= maxSompi;
    const maxStr = withdrawalLimitLabel(maxSompi, canDrain);
    const extNote = externalFundsNote(picked.externalSompi, 'owner-reclaimable');
    const governedKas = sompiToKasFixed(threadSompi, 2);

    if (isFundsSpent(total)) {
        reportFundsSpent(st);
        return true;
    }

    st.style.color = '';
    const text = allowanceStatus({
        threadSompi, iAmOwner, startDaa, currentDaa, cooldown, threadDaa, canDrain, governedKas, maxStr,
    });
    setContent(st, text, extNote);
    return false;
}
