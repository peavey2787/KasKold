import { covenantState } from '../../../../state/index.js';
import { fetchCurrentDaa } from '../../../../../core/node/daa.js';
// KasKold Companion Web — focused covenant result action registration.

import { byId } from '../../../../../core/dom.js';
import { formatDaaDuration } from '../../../../../core/format.js';
import { sompiToKasFixed, sompiToKasString } from '../../../../../core/amounts.js';
import { exactUnsigned } from '../../../../../core/exact.js';
function readPiggyTerms() {
    const result = covenantState.lastCovenantResult;
    return {
        thr: result.threshold_sompi ? BigInt(result.threshold_sompi) : 0n,
        dl: result.deadline_daa ? BigInt(result.deadline_daa) : 0n,
    };
}

async function currentDaaForPiggy() {
    let curDaa = 0n;
    try { curDaa = await fetchCurrentDaa(); } catch (_) {}
    if (curDaa === 0n && typeof covenantState._lastKnownDaa !== 'undefined') curDaa = exactUnsigned(covenantState._lastKnownDaa ?? 0n, 'last known DAA');
    return curDaa;
}

function deadlineEtaText(dl, curDaa) {
    return curDaa > 0n && dl > curDaa ? '~' + formatDaaDuration(dl - curDaa) : 'unknown';
}

function unmetPiggyTerms(totalSompi, thr, dl, curDaa) {
    const parts = [];
    if (thr > 0n) parts.push('goal ' + sompiToKasString(thr) + ' KAS not reached (have ' + sompiToKasFixed(totalSompi, 4) + ')');
    if (dl > 0n) parts.push('deadline not passed (' + deadlineEtaText(dl, curDaa) + ' left)');
    return parts.join(' and ');
}

export function createPiggyHelpers() {
    // Evaluate whether a piggy (additive) can be broken RIGHT NOW.
    // totalSompi/feeSompi as BigInt. Returns {canBreak, goalMet, deadlinePassed,
    // text, color}: goalMet checks output[0] (total - fee) >= threshold; the
    // deadline path needs current DAA >= deadline. No conditions set = breakable.
    const piggyBreakStatus = async function (totalSompi, feeSompi) {
        const { thr, dl } = readPiggyTerms();
        if (thr === 0n && dl === 0n) {
            return { canBreak: true, goalMet: true, deadlinePassed: true,
                     text: 'No conditions set — breakable anytime.', color: 'var(--accent, #4caf50)' };
        }
        const curDaa = await currentDaaForPiggy();
        const goalMet = thr > 0n && (totalSompi - feeSompi) >= thr;
        const deadlinePassed = dl > 0n && curDaa > 0n && curDaa >= dl;
        if (goalMet || deadlinePassed) {
            const why = goalMet ? 'goal reached' : 'deadline passed';
            return { canBreak: true, goalMet, deadlinePassed,
                     text: 'Breakable now (' + why + ').', color: 'var(--accent, #4caf50)' };
        }
        return { canBreak: false, goalMet, deadlinePassed,
                 text: 'NOT breakable yet: ' + unmetPiggyTerms(totalSompi, thr, dl, curDaa) + '. A break TX would be rejected on-chain.',
                 color: 'var(--error, #f44336)' };
    };

    // Insert/update the piggy status banner above the owner help text.
    const piggyStatusBanner = function (status) {
        let b = byId('cov-piggy-status-banner');
        if (!b) {
            b = document.createElement('div');
            b.id = 'cov-piggy-status-banner';
            b.classList.add('piggy-result-action');
            const help = byId('cov-owner-help');
            if (help && help.parentNode) help.parentNode.insertBefore(b, help);
        }
        b.style.color = status.color;
        b.style.borderColor = status.color;
        b.textContent = status.text;
        b.classList.remove('hidden');
        return b;
    };
    return { piggyBreakStatus, piggyStatusBanner };
}
