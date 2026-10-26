import { covenantState } from '../../../../app/state/index.js';
import { fetchCurrentDaa } from '../../../../core/node/daa.js';
// KasKold Companion Web — features/covenants/watchers_and_ui/ui/metadata
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
export function covMetaLine(c) {
    const t = (c && c.type) || '';
    const refund = () => formatStartDate({ locktime_daa: c.locktime_daa, start_date_iso: c.locktime_date_iso }, covenantState._lastKnownDaa);
    if (t === 'dms' && c.inactivity_daa) {
        return `Type: Dead Man's Switch | Inactivity: ${formatDaaDuration(exactUnsigned(c.inactivity_daa, 'inactivity DAA'))}`;
    }
    if (t === 'global-spending-limit' || t === 'global-allowance') {
        const mw = exactUnsigned(c.max_withdraw_sompi ?? 0n, 'withdrawal limit');
        const cd = exactUnsigned(c.cooldown_daa ?? 0n, 'cooldown DAA');
        if (mw > 0n) {
            const cdStr = cd > 0n ? ` | Cooldown: ${formatDaaDuration(cd)}` : '';
            const label = t === 'global-spending-limit' ? 'Global Spending Limit | Limit' : 'Global Allowance | Max';
            const startStr = t === 'global-allowance' && c.start_date_iso
                ? ` | Start: ${formatStartDate(c, covenantState._lastKnownDaa)}`
                : '';
            return `Type: ${label}: ${sompiToKasString(mw)} KAS/spend${cdStr}${startStr}`;
        }
    }
    if (t === 'additive') {
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
    if (t === 'merkle-whitelist') {
        let n = 0;
        try { n = JSON.parse(c.merkle_addresses_json || '[]').length; } catch (_) {}
        return 'Type: Merkle Whitelist | ' + n + ' addresses | Refund: ' + refund();
    }
    if (t === 'payjoin') return 'Type: PayJoin | Refund timeout: ' + refund();
    if (t === 'commit-reveal') return 'Type: Commit-Reveal | Refund timeout: ' + refund();
    if (t === 'oracle-v1') return 'Type: Oracle | Attestation-bound release | Owner refund: ' + refund();
    if (t === 'crowdfund') return `Type: ZK Crowdfunding | Goal: ${sompiToKasString(c.goal_sompi ?? 0n)} KAS | ${c.crowdfund_role === 'organizer' ? 'Organizer' : 'Contributor'} | Refund: ${refund()}`;
    if (t === 'private-swap') return 'Type: Private Swap (adaptor signature)';
    return 'Type: ' + t + (c.locktime_daa ? ' | Locktime: ' + refund() : '');
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
