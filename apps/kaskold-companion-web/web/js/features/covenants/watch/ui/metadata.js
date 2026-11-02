import { covenantState } from '../../../../app/state/index.js';
import { fetchCurrentDaa } from '../../../../core/node/daa.js';
// KasKold Companion Web — features/covenants/watch/ui/metadata
import { byId } from '../../../../core/dom.js';
import { exactUnsigned } from '../../../../core/exact.js';
import { sompiToKasString } from '../../../../core/amounts.js';
import { formatDaaDuration, formatStartDate } from '../../../../core/format.js';

// ── Covenant result meta line ───────────────────────────────────────────────
// Single source of truth for the "Type: ... | ..." summary on the covenant
// result panel. All four entry points (creation, active-list reload,
// post-funding return, invite load) call covRenderMetaLine, so a per-type
// display change is made here once.
//
// covMetaLine(c): pure function -> string. Timed fields go through
// formatStartDate (exact date if *_date_iso present, else ~estimate from
// covenantState._lastKnownDaa, else raw 'DAA N').

function refundText(c) {
    return formatStartDate({ locktime_daa: c.locktime_daa, start_date_iso: c.locktime_date_iso }, covenantState._lastKnownDaa);
}

function dmsMetaLine(c) {
    if (!c.inactivity_daa) return null;
    return `Type: Dead Man's Switch | Inactivity: ${formatDaaDuration(exactUnsigned(c.inactivity_daa, 'inactivity DAA'))}`;
}

function spendingLimitLabel(t) {
    return t === 'global-spending-limit' ? 'Global Spending Limit | Limit' : 'Global Allowance | Max';
}

function spendingLimitCooldownText(cd) {
    return cd > 0n ? ` | Cooldown: ${formatDaaDuration(cd)}` : '';
}

function spendingLimitStartText(t, c) {
    return t === 'global-allowance' && c.start_date_iso
        ? ` | Start: ${formatStartDate(c, covenantState._lastKnownDaa)}`
        : '';
}

function spendingLimitMetaLine(c, t) {
    const mw = exactUnsigned(c.max_withdraw_sompi ?? 0n, 'withdrawal limit');
    const cd = exactUnsigned(c.cooldown_daa ?? 0n, 'cooldown DAA');
    if (!(mw > 0n)) return null;
    const cdStr = spendingLimitCooldownText(cd);
    const startStr = spendingLimitStartText(t, c);
    return `Type: ${spendingLimitLabel(t)}: ${sompiToKasString(mw)} KAS/spend${cdStr}${startStr}`;
}

function piggyBankMetaLine(c) {
    const threshold = exactUnsigned(c.threshold_sompi ?? 0n, 'savings threshold');
    const deadline = exactUnsigned(c.deadline_daa ?? 0n, 'deadline DAA');
    let line = 'Type: Piggy Bank';
    if (threshold > 0n) line += ` | Goal: ${sompiToKasString(threshold)} KAS`;
    if (deadline > 0n && c.deadline_date_iso) {
        line += ' | Deadline: ' + formatStartDate({ start_date_iso: c.deadline_date_iso, start_daa: deadline }, covenantState._lastKnownDaa);
    }
    if (threshold === 0n && deadline === 0n) line += ' | No conditions (break anytime)';
    return line;
}

function merkleAddressCount(json) {
    try { return JSON.parse(json || '[]').length; } catch { return 0; }
}

function merkleWhitelistMetaLine(c) {
    return 'Type: Merkle Whitelist | ' + merkleAddressCount(c.merkle_addresses_json) + ' addresses | Refund: ' + refundText(c);
}

function crowdfundMetaLine(c) {
    const roleLabel = c.crowdfund_role === 'organizer' ? 'Organizer' : 'Contributor';
    return `Type: ZK Crowdfunding | Goal: ${sompiToKasString(c.goal_sompi ?? 0n)} KAS | ${roleLabel} | Refund: ${refundText(c)}`;
}

// Types with a bespoke summary. A builder returns null when the covenant does
// not qualify for its summary, in which case the generic line is used.
const META_LINE_BUILDERS = new Map([
    ['dms', dmsMetaLine],
    ['global-spending-limit', spendingLimitMetaLine],
    ['global-allowance', spendingLimitMetaLine],
    ['additive', piggyBankMetaLine],
    ['merkle-whitelist', merkleWhitelistMetaLine],
    ['payjoin', (c) => 'Type: PayJoin | Refund timeout: ' + refundText(c)],
    ['commit-reveal', (c) => 'Type: Commit-Reveal | Refund timeout: ' + refundText(c)],
    ['oracle-v1', (c) => 'Type: Oracle | Attestation-bound release | Owner refund: ' + refundText(c)],
    ['crowdfund', crowdfundMetaLine],
    ['private-swap', () => 'Type: Private Swap (adaptor signature)'],
]);

function genericMetaLine(t, c) {
    return 'Type: ' + t + (c.locktime_daa ? ' | Locktime: ' + refundText(c) : '');
}

export function covMetaLine(c) {
    const t = (c && c.type) || '';
    const builder = META_LINE_BUILDERS.get(t);
    const line = builder ? builder(c, t) : null;
    return line ?? genericMetaLine(t, c);
}
// field could only resolve to a raw DAA (no *_date_iso and no cached DAA),
// fetch the current DAA once and re-render so it shows an estimated date.
export function covRenderMetaLine(c) {
    const node = byId('cov-result-extra');
    if (!node || !c) return;
    node.textContent = covMetaLine(c);
    const timed = c.locktime_daa || c.deadline_daa || c.start_daa;
    const hasIso = c.locktime_date_iso || c.deadline_date_iso || c.start_date_iso;
    if (timed && !hasIso && exactUnsigned(covenantState._lastKnownDaa ?? 0n, 'last known DAA') === 0n) {
        fetchCurrentDaa().then(daa => {
            if (daa > 0n) { covenantState._lastKnownDaa = daa; node.textContent = covMetaLine(c); }
        }).catch(() => {});
    }
}
