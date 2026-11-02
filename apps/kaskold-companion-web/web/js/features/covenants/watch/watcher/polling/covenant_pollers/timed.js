import { setContent, span } from '../../../../../../core/ui/elements.js';
import { covenantWatcherState } from '../../../../boundary.js';
import { formatDaaDuration } from '../../../../../../core/format.js';
import { isFundsSpent, setPlainStatus } from './status_text.js';

function isClockKnown(locktime, currentDaa) {
    return locktime > 0n && currentDaa > 0n;
}

function reportTimedSpent(st, labels) {
    const spender = covenantWatcherState._covWatcherSpendPath || 'unknown';
    setContent(st, span('u-text-teal', '✅ ' + labels.spent(spender)));
    if (labels.onSpent) labels.onSpent();
    if (st) st.style.display = '';
}

function renderTimedStatus(st, labels, { kas, locktime, currentDaa }) {
    if (!isClockKnown(locktime, currentDaa)) {
        setPlainStatus(st, labels.watching(kas));
    } else if (currentDaa >= locktime + 300n) {
        setContent(st, span(labels.availableClass, labels.available(kas)));
    } else if (currentDaa >= locktime) {
        setContent(st, span(labels.unlockingClass, labels.unlocking(kas)));
    } else {
        setPlainStatus(st, labels.locked(kas, formatDaaDuration(locktime - currentDaa)));
    }
}

export async function pollTimedBalance(state, labels) {
    const { total, st } = state;
    if (isFundsSpent(total)) {
        reportTimedSpent(st, labels);
        return true;
    }
    if (total === 0n) setPlainStatus(st, '👁 0 KAS | Not funded');
    else renderTimedStatus(st, labels, state);
    if (st) st.style.display = '';
    return false;
}

function claimLabels({ ownerSpent, claimSpent, claimAvailable, refundAvailable }) {
    return {
        spent: spender => spender === 'owner' ? ownerSpent : claimSpent,
        availableClass: 'u-text-warning',
        available: refundAvailable,
        unlockingClass: 'u-text-warning',
        unlocking: kas => '⏳ ' + kas + ' KAS | Timeout passing...',
        locked: (kas, timeStr) => '👁 ' + kas + ' KAS | Refund in ~' + timeStr + ' | ' + claimAvailable,
        watching: kas => '👁 ' + kas + ' KAS | Watching...',
    };
}

export async function pollMerkleWhitelist(state) {
    return pollTimedBalance(state, claimLabels({
        ownerSpent: 'Owner refunded.',
        claimSpent: 'Spent to whitelisted address.',
        claimAvailable: 'Whitelisted spend available now',
        refundAvailable: kas => '✅ ' + kas + ' KAS | Refund available now',
    }));
}

export async function pollPayjoin(state) {
    return pollTimedBalance(state, claimLabels({
        ownerSpent: 'Owner refunded.',
        claimSpent: 'PayJoin claimed.',
        claimAvailable: 'Claim available now',
        refundAvailable: kas => '⚠ ' + kas + ' KAS | Refund available. Claim still open.',
    }));
}

export async function pollCommitReveal(state) {
    return pollTimedBalance(state, claimLabels({
        ownerSpent: 'Owner refunded.',
        claimSpent: 'Preimage revealed. Funds spent.',
        claimAvailable: 'Reveal available',
        refundAvailable: kas => '⚠ ' + kas + ' KAS | Refund available. Reveal still open.',
    }));
}
