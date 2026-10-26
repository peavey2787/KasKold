import { covenantState } from '../../../../app/state/index.js';
import { byId } from '../../../../core/dom.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../generation/ui_and_keys.js';
import { covRenderMetaLine } from '../../watchers_and_ui/ui/metadata.js';
import { covUpdateResultButtons } from '../../watchers_and_ui/ui/result_buttons.js';
import { ACTIVE_METADATA_FIELDS, copyDefinedFields, saveActiveRecords } from './repository.js';
import { stringifyCovenantJson } from '../../model/exact_fields.js';
import { SCRIPT_TERM_TYPES, applyScriptTerms } from '../../model/script_terms.js';
import { inferCovenantRole } from '../import/role.js';
import { restorePrivateSwapState } from '../../private_swap/state.js';

export function openActiveCovenant(covenant) {
    let result;
    try {
        result = applyScriptTerms(restoreActiveResult(covenant));
    } catch (error) {
        toast('Covenant rejected: ' + (error.message || error), 'error');
        return;
    }
    covenantState.lastCovenantResult = result;
    detectStoredRole(covenant);
    persistLastResult();
    showResultPanel(covenant);
}

function restoreActiveResult(covenant) {
    const result = {
        address: covenant.address,
        redeem_script_hex: covenant.redeem_script_hex,
        locktime_daa: covenant.locktime_daa,
        type: covenant.type,
        loaded: covenant.loaded || false,
    };
    copyDefinedFields(covenant, result, ACTIVE_METADATA_FIELDS);
    if (covenant.covenant_id_hex) result.covenant_id_hex = covenant.covenant_id_hex;
    return result;
}

// The role follows the keys the script proves, so a stored role is refreshed.
function detectStoredRole(covenant) {
    if (!SCRIPT_TERM_TYPES.has(covenant.type)) return;
    const role = inferCovenantRole(covenantState.lastCovenantResult);
    covenantState.lastCovenantResult.role = role;
    covenant.role = role;
    saveActiveRecords();
}

function persistLastResult() {
    try {
        sessionStorage.setItem('lastCovenantResult', stringifyCovenantJson(covenantState.lastCovenantResult));
    } catch (_) {
        // Session persistence is optional.
    }
}

function showResultPanel(covenant) {
    if (covenant.type === 'private-swap') {
        if (!covenant.private_swap_recovery_json) throw new Error('Private Swap recovery transcript is missing');
        restorePrivateSwapState(covenant.private_swap_recovery_json);
        covShowPanel('private-swap');
        toast('Loaded Private Swap protocol state', 'ok', 1500);
        return;
    }
    covShowPanel('result');
    covUpdateResultButtons(covenant.type);
    byId('cov-result-addr').textContent = covenant.address;
    byId('cov-result-script').textContent = covenant.redeem_script_hex;
    covRenderMetaLine(covenantState.lastCovenantResult);
    const balance = byId('cov-result-balance');
    balance.textContent = 'Loading...';
    balance.style.display = '';
    toast(`Loaded: ${covenant.label} covenant`, 'ok', 1500);
    setTimeout(() => byId('btn-cov-res-balance')?.click(), 300);
}
