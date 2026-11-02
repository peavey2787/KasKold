// @ts-check
import { covenantState } from '../../../state/index.js';
import { fetchCurrentDaa } from '../../../../core/node/daa.js';
import { resolveNodeUrl } from '../../../../core/node/resolver.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../features/covenants/generation/panel_keys.js';
import { ownerReceiveAddr } from '../../../../features/covenants/payload/state.js';
import { covScanAddress } from '../../../../features/covenants/scanning.js';
import { handleCovBeneficiarySpend } from '../../../../features/covenants/spending/standard/spend_paths.js';
import { applyScriptTerms } from '../../../../features/covenants/model/script_terms.js';
import { fetch_utxos_for_address_js } from '../../../../wasm/api.js';
// KasKold Companion Web — app/events/contracts/covenant_creation/claim_sharing
import { byId, elementById, inputById } from '../../../../core/dom.js';
import { bindDurationInputs } from '../../../../core/forms/duration.js';
import { openUtxoPicker } from './utxo_picker.js';
import { registerInviteSharingActions } from './invite_sharing.js';

import { formatDaaDuration } from '../../../../core/format.js';
import { exactUnsigned } from '../../../../core/exact.js';








// Savings pre-flight: returns the ETA when a timelocked-savings claim is still locked, else null.
// Uses a LIVE DAA fetch (the picker confirm is sync, so the cached _lastKnownDaa is unreliable there).
async function savingsLockEta(lt) {
    const lockN = exactUnsigned(lt || '0', 'savings locktime DAA');
    if (lockN <= 0n) return null;
    const curDaa = await fetchCurrentDaa();
    return curDaa > 0n && curDaa < lockN ? formatDaaDuration(lockN - curDaa) : null;
}

async function fetchVaultUtxos(address) {
    try { return JSON.parse(await fetch_utxos_for_address_js(address, await resolveNodeUrl())); } catch { return []; }
}

function oldestUtxoDaa(utxos) {
    let oldest = null;
    for (const u of utxos) {
        const d = exactUnsigned(u.block_daa_score ?? 0n, 'UTXO DAA');
        if (d > 0n && (oldest === null || d < oldest)) oldest = d;
    }
    return oldest;
}

// DMS heir claim is gated by CSV (per-UTXO age). The OLDEST UTXO ages first, so return the ETA
// only when not even that one has cleared the inactivity period. Once at least one has aged,
// the heir may batch-claim the aged UTXOs in the picker.
async function dmsClaimEta(result) {
    const inact = exactUnsigned(result.inactivity_daa ?? 0n, 'inactivity DAA');
    if (inact <= 0n) return null;
    const curDaa = await fetchCurrentDaa();
    const utxos = await fetchVaultUtxos(covenantState.lastCovenantResult.address);
    if (curDaa <= 0n || !utxos.length) return null;
    const oldest = oldestUtxoDaa(utxos);
    return oldest !== null && curDaa < oldest + inact ? formatDaaDuration(oldest + inact - curDaa) : null;
}

// Returns { message, ms } when the selected beneficiary claim is still time-locked, else null.
async function beneficiaryClaimBlock(lt) {
    const result = covenantState.lastCovenantResult;
    const type = result && result.type;
    if (type === 'timelocked-savings') {
        const eta = await savingsLockEta(lt);
        return eta === null ? null : { message: 'Still locked. Unlocks in ~' + eta + '. An early claim is rejected by the node.', ms: 5000 };
    }
    if (type === 'dms') {
        const eta = await dmsClaimEta(result);
        return eta === null ? null : { message: 'Still locked. No vault UTXO has aged past the inactivity period yet. The heir can claim in ~' + eta + '. The node rejects an early claim.', ms: 6000 };
    }
    return null;
}

function registerSpendPanelActions() {
    if (byId('btn-cov-owner-spend')) elementById('btn-cov-owner-spend').onclick = () => {
        covShowPanel('owner');
        if (covenantState.lastCovenantResult) {
            inputById('cov-owner-addr').value = covenantState.lastCovenantResult.address || '';
            inputById('cov-owner-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
            // DMS heartbeat: send back to same covenant address to reset CSV timer
            if (covenantState.lastCovenantResult.type === 'dms') {
                inputById('cov-owner-dest').value = covenantState.lastCovenantResult.address || '';
            }
        }
    };
    elementById('btn-cov-owner-back').onclick = () => {
        covShowPanel(covenantState.lastCovenantResult ? 'result' : 'menu');
    };
    if (byId('btn-cov-beneficiary-spend')) elementById('btn-cov-beneficiary-spend').onclick = () => {
        covShowPanel('beneficiary');
        if (covenantState.lastCovenantResult) {
            inputById('cov-bene-addr').value = covenantState.lastCovenantResult.address || '';
            inputById('cov-bene-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
            if (covenantState.lastCovenantResult.locktime_daa) {
                inputById('cov-bene-locktime').value = covenantState.lastCovenantResult.locktime_daa;
            }
            // Escrow: the seller claims to the buyer address the script pays to.
            if (covenantState.lastCovenantResult.type === 'escrow') {
                try {
                    inputById('cov-bene-dest').value = applyScriptTerms(covenantState.lastCovenantResult).alice_address;
                } catch (e) {
                    toast('Escrow script rejected: ' + (e.message || e), 'error');
                }
            }
        }
    };
    elementById('btn-cov-bene-back').onclick = () => covShowPanel('menu');
    elementById('btn-cov-bene-create').onclick = () => handleCovBeneficiarySpend();
    if (byId('btn-cov-bene-pick')) {
        elementById('btn-cov-bene-pick').onclick = async () => {
            // Claim only the selected UTXOs (batched). Dest is the claiming wallet's
            // address; locktime comes from the claim setup.
            const dest = (byId('cov-bene-dest') ? inputById('cov-bene-dest').value.trim() : '') || ownerReceiveAddr();
            const lt = byId('cov-bene-locktime') ? (inputById('cov-bene-locktime').value.trim() || '0') : '0';
            const block = await beneficiaryClaimBlock(lt);
            if (block) {
                toast(block.message, 'error', block.ms);
                return;
            }
            openUtxoPicker(dest, { locktime: lt });
        };
    }
}

function registerClaimAndPresetActions() {
    if (byId('btn-cov-scan-dms2-heir')) elementById('btn-cov-scan-dms2-heir').onclick = () => covScanAddress('cov-dms2-heir-pk', 'Scan heir address', true);
    if (byId('cov-dms2-preset')) {
        bindDurationInputs({ prefix: 'cov-dms2', outputId: 'cov-dms2-duration' });
        elementById('cov-dms2-preset').onchange = () => {
            const v = inputById('cov-dms2-preset').value;
            const customWrap = byId('cov-dms2-custom-wrap');
            if (customWrap) customWrap.classList.toggle('hidden', v !== 'custom');
            if (v !== 'custom') inputById('cov-dms2-duration').value = v;
        };
        // Custom is default; no duration pre-fill. User picks a preset or opens Custom rolling inputs.
        if (inputById('cov-dms2-preset').value !== 'custom') {
            inputById('cov-dms2-duration').value = inputById('cov-dms2-preset').value;
        }
    }
}

export function registerClaimSharing() {
    registerInviteSharingActions();
    registerSpendPanelActions();
    registerClaimAndPresetActions();
}
