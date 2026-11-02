// @ts-check
import { setContent, span } from '../../../../../../core/ui/elements.js';
import { covenantState, covenantWatcherState } from '../../../../boundary.js';
import { exactUnsigned } from '../../../../../../core/exact.js';
import { formatDaaDuration } from '../../../../../../core/format.js';
import { sompiToKasString } from '../../../../../../core/amounts.js';
import { covWatcherStop } from '../lifecycle.js';
import { pollTimedBalance } from './timed.js';
import { isFundsSpent, setPlainStatus } from './status_text.js';

export async function pollTimelockedSavings(state) {
    return pollTimedBalance(state, {
        spent: () => 'Claimed.',
        onSpent: covWatcherStop,
        availableClass: 'u-text-teal',
        available: kas => `✅ Unlocked. ${kas} KAS claimable.`,
        unlockingClass: 'u-text-warning',
        unlocking: kas => `⏳ Unlocking... claim available shortly. ${kas} KAS`,
        locked: (kas, timeStr) => `👁 ${kas} KAS | Locked, unlocks in ~${timeStr}`,
        watching: kas => `👁 ${kas} KAS | Watching...`,
    });
}

function reportDmsSpent(st) {
    const spender = covenantWatcherState._covWatcherSpendPath || 'unknown';
    if (spender === 'heir') {
        setContent(st, span('u-text-warning', '⚠ Heir claimed the funds.'));
        covWatcherStop();
        if (st) st.style.display = '';
    } else if (spender === 'owner') {
        setContent(st, span('u-text-text-muted', '⏳ Owner spent. Checking...'));
    } else {
        st.textContent = '👁 Funds spent (0 KAS)';
    }
}

function newestUtxoDaa(utxos) {
    let newest = 0n;
    for (const utxo of utxos) {
        const daa = exactUnsigned(utxo.block_daa_score ?? 0n, 'UTXO DAA score');
        if (daa > newest) newest = daa;
    }
    return newest;
}

function renderInactivityStatus(st, kas, remaining) {
    if (remaining <= -300n) {
        setContent(st, span('u-text-warning', `⚠ Inactivity period passed. Heir can claim. ${kas} KAS`));
    } else if (remaining <= 0n) {
        setContent(st, span('u-text-warning', `⏳ Inactivity period ending... Heir claim available shortly. ${kas} KAS`));
    } else {
        setPlainStatus(st, `👁 ${kas} KAS | ~${formatDaaDuration(remaining)} until heir can claim`);
    }
}

export async function pollDms(state) {
    const { total, kas, st, currentDaa, utxos } = state;
    const inactivity = exactUnsigned(covenantState.lastCovenantResult.inactivity_daa ?? 0n, 'inactivity DAA');
    if (isFundsSpent(total)) {
        reportDmsSpent(st);
        return true;
    }
    if (total === 0n) {
        setPlainStatus(st, '👁 0 KAS | Not funded');
    } else if (inactivity > 0n && currentDaa > 0n) {
        renderInactivityStatus(st, kas, newestUtxoDaa(utxos) + inactivity - currentDaa);
    } else {
        setPlainStatus(st, `👁 ${kas} KAS | Watching...`);
    }
    return false;
}

function reportPiggyBroken(st) {
    setContent(st, span('u-text-teal', '✅ Piggy bank broken! Funds withdrawn.'));
    covWatcherStop();
    if (st) st.style.display = '';
}

function thresholdParts(total, threshold, kas) {
    if (!(threshold > 0n)) return [`${kas} KAS`];
    const roundedPercent = (total * 100n + threshold / 2n) / threshold;
    const percent = roundedPercent > 100n ? 100n : roundedPercent;
    const parts = [`${kas} / ${sompiToKasString(threshold)} KAS (${percent}%)`];
    if (total >= threshold) parts.push('✅ Goal reached!');
    return parts;
}

function deadlineParts(currentDaa, deadlineDaa) {
    if (!(deadlineDaa > 0n && currentDaa > 0n)) return [];
    if (currentDaa >= deadlineDaa) return ['⏰ Deadline passed'];
    return [`~${formatDaaDuration(deadlineDaa - currentDaa)} until deadline`];
}

function isPiggyBreakable({ total, threshold, deadlineDaa, currentDaa }) {
    const canBreakGoal = threshold > 0n && total >= threshold;
    const canBreakTime = deadlineDaa > 0n && currentDaa > 0n && currentDaa >= deadlineDaa;
    const noConditions = threshold === 0n && deadlineDaa === 0n;
    return canBreakGoal || canBreakTime || noConditions;
}

function renderAdditiveStatus(st, parts, breakable) {
    const text = parts.join(' | ');
    if (breakable) setContent(st, span('u-text-teal', `👁 ${text}`));
    else setPlainStatus(st, `👁 ${text}`);
}

export async function pollAdditive(state) {
    const { total, kas, st, currentDaa } = state;
    const result = covenantState.lastCovenantResult;
    const threshold = exactUnsigned(result.threshold_sompi ?? 0n, 'savings threshold');
    const deadlineDaa = exactUnsigned(result.deadline_daa ?? 0n, 'deadline DAA');

    if (isFundsSpent(total)) {
        reportPiggyBroken(st);
        return true;
    }
    if (total === 0n) {
        setPlainStatus(st, '👁 0 KAS | Not funded');
        return false;
    }

    const parts = [...thresholdParts(total, threshold, kas), ...deadlineParts(currentDaa, deadlineDaa)];
    renderAdditiveStatus(st, parts, isPiggyBreakable({ total, threshold, deadlineDaa, currentDaa }));
    return false;
}
