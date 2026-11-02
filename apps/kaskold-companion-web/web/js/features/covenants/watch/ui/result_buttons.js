import { covenantState } from '../../../../app/state/index.js';
// KasKold Companion Web — covenant result-button façade.
import { byId } from '../../../../core/dom.js';
import { configurePrimaryCoreActions } from './result_buttons/primary_core.js';
import { configurePrimaryAdvancedActions } from './result_buttons/primary_advanced.js';
import { configureAuxiliaryResultActions } from './result_buttons/auxiliary.js';


function configurePrimaryResultActions(state) {
    if (!configurePrimaryCoreActions(state)) {
        configurePrimaryAdvancedActions(state);
    }
}

const HIDDEN_RESULT_PANEL_IDS = [
    'btn-cov-res-oracle-v1-attest',
    'btn-cov-res-oracle-v1-bind',
    'btn-cov-res-oracle-v1-scan-binding',
    'crowdfund-result-panel',
];

const TIMELOCK_RESULT_TYPES = ['dms', 'escrow', 'timelocked-escrow', 'global-allowance'];

function hideStaleResultPanels() {
    for (const id of HIDDEN_RESULT_PANEL_IDS) {
        byId(id)?.classList.add('hidden');
    }
}

function resetResultButtonVisibility({ beneBtn, ownerBtn, consolBtn, fundBtn }) {
    if (fundBtn) fundBtn.style.display = '';
    beneBtn.style.display = '';
    ownerBtn.style.display = '';
    if (consolBtn) consolBtn.style.display = 'none';
}

function readLastCovenantResultContext() {
    const lastResult = covenantState.lastCovenantResult;
    const isLoaded = lastResult && lastResult.loaded;
    const covRole = lastResult && lastResult.role;
    const isBeneficiary = isLoaded && covRole === 'beneficiary';
    return { isLoaded, covRole, isBeneficiary };
}

function hideOwnerFundingForBeneficiary({ ownerBtn, fundBtn }) {
    ownerBtn.style.display = 'none';
    if (fundBtn) fundBtn.style.display = 'none';
}

export function covUpdateResultButtons(type) {
    const beneBtn = byId('btn-cov-res-bene');
    const ownerBtn = byId('btn-cov-res-owner');
    const consolBtn = byId('btn-cov-res-consolidate');
    const fundBtn = byId('btn-cov-fund');
    if (!beneBtn) return;
    hideStaleResultPanels();
    resetResultButtonVisibility({ beneBtn, ownerBtn, consolBtn, fundBtn });

    const { isLoaded, covRole, isBeneficiary } = readLastCovenantResultContext();
    if (isBeneficiary && TIMELOCK_RESULT_TYPES.includes(type)) {
        hideOwnerFundingForBeneficiary({ ownerBtn, fundBtn });
    }

    const state = { type, beneBtn, ownerBtn, consolBtn, fundBtn, isLoaded, covRole, isBeneficiary };
    configurePrimaryResultActions(state);
    configureAuxiliaryResultActions(state);
}
