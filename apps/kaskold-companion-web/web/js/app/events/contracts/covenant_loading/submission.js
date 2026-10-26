import { covenantRecoveryState, covenantState } from '../../../state/index.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../features/covenants/generation/ui_and_keys.js';
import { covAddActive } from '../../../../features/covenants/recovery/active.js';
import { inferCovenantRole } from '../../../../features/covenants/recovery/import/role.js';
import { SCRIPT_TERM_TYPES, verifiedCovenantTerms } from '../../../../features/covenants/model/script_terms.js';
import { covRenderMetaLine } from '../../../../features/covenants/watchers_and_ui/ui/metadata.js';
import { covUpdateResultButtons } from '../../../../features/covenants/watchers_and_ui/ui/result_buttons.js';
import { byId } from '../../../../core/dom.js';

// A loaded covenant of a known family shows only what its script proves;
// other types keep the address and script the user entered.
function loadedCovenant(address, script, type) {
    const claimed = { type, address, redeem_script_hex: script };
    if (!SCRIPT_TERM_TYPES.has(type)) return { ...claimed, loaded: true };
    const result = { ...verifiedCovenantTerms(claimed), loaded: true };
    result.role = inferCovenantRole(result);
    return result;
}

function takeInviteDate(result) {
    if (covenantRecoveryState._covLoadedLdi) result.locktime_date_iso = covenantRecoveryState._covLoadedLdi;
    covenantRecoveryState._covLoadedLdi = null;
}

export function bindLoadSubmissionAction() {
    byId('btn-cov-load-submit').onclick = () => {
        const addr = byId('cov-load-addr').value.trim();
        const script = byId('cov-load-script').value.trim();
        const type = byId('cov-load-type').value;
        if (!addr) { toast('Enter covenant address', 'error'); return; }
        if (!script) { toast('Enter redeem script hex', 'error'); return; }
        let result;
        try {
            result = loadedCovenant(addr, script, type);
        } catch (error) {
            toast('Covenant rejected: ' + (error.message || error), 'error');
            return;
        }
        takeInviteDate(result);
        covenantState.lastCovenantResult = result;
        try { sessionStorage.setItem('lastCovenantResult', JSON.stringify(result)); } catch (_) {}
        covAddActive(type, result);
        covShowPanel('result');
        covUpdateResultButtons(type);
        byId('cov-result-addr').textContent = result.address;
        byId('cov-result-script').textContent = result.redeem_script_hex;
        covRenderMetaLine(result);
        byId('cov-result-balance').style.display = 'none';
        toast('Covenant loaded', 'ok', 2000);
    };
}
