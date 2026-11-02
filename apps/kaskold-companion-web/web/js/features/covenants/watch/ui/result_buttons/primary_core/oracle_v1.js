import { covenantState } from '../../../../../../app/state/index.js';
import { byId } from '../../../../../../core/dom.js';

const BINDING_TOKEN_HEX_PATTERN = /^[0-9a-f]{64}$/;

function readOracleActionButtons() {
    return {
        attestBtn: byId('btn-cov-res-oracle-v1-attest'),
        bindBtn: byId('btn-cov-res-oracle-v1-bind'),
        scanBindingBtn: byId('btn-cov-res-oracle-v1-scan-binding'),
    };
}

function applyOracleDefaultLabels({ beneBtn, ownerBtn, fundBtn }) {
    if (fundBtn) fundBtn.textContent = 'Covenant Deposit';
    beneBtn.textContent = 'Claim with Oracle Attestation';
    ownerBtn.textContent = 'Owner Refund (after timeout)';
    beneBtn.style.display = 'none';
    ownerBtn.style.display = '';
}

function hideOracleActionButtons({ attestBtn, bindBtn, scanBindingBtn }) {
    if (attestBtn) attestBtn.classList.add('hidden');
    bindBtn?.classList.add('hidden');
    scanBindingBtn?.classList.add('hidden');
}

function applyUnboundLayout({ fundBtn, isLoaded }, { bindBtn, scanBindingBtn }) {
    if (fundBtn) fundBtn.style.display = 'none';
    if (!isLoaded) {
        bindBtn?.classList.remove('hidden');
        scanBindingBtn?.classList.remove('hidden');
    }
}

function hideFundingAndOwnerButtons({ beneBtn, ownerBtn, fundBtn }) {
    beneBtn.style.display = 'none';
    ownerBtn.style.display = 'none';
    if (fundBtn) fundBtn.style.display = 'none';
}

function applyBeneficiaryLayout({ beneBtn, ownerBtn, fundBtn }) {
    beneBtn.style.display = '';
    ownerBtn.style.display = 'none';
    if (fundBtn) fundBtn.style.display = 'none';
}

function applyNonOwnerLayout(buttons, { attestBtn }) {
    hideFundingAndOwnerButtons(buttons);
    if (attestBtn) attestBtn.classList.remove('hidden');
}

function applyBoundLayout(buttons, { isLoaded, covRole }, actionButtons) {
    if (!isLoaded) return;
    if (covRole === 'beneficiary') {
        applyBeneficiaryLayout(buttons);
    } else if (covRole !== 'owner') {
        applyNonOwnerLayout(buttons, actionButtons);
    }
}

export function configureOracleV1({ beneBtn, ownerBtn, fundBtn, isLoaded, covRole }) {
    const actionButtons = readOracleActionButtons();
    const result = covenantState.lastCovenantResult || {};
    const bound = BINDING_TOKEN_HEX_PATTERN.test(result.oracle_covenant_binding_token_hex || '');
    const buttons = { beneBtn, ownerBtn, fundBtn };
    applyOracleDefaultLabels(buttons);
    hideOracleActionButtons(actionButtons);

    if (!bound) {
        applyUnboundLayout({ fundBtn, isLoaded }, actionButtons);
        return;
    }
    applyBoundLayout(buttons, { isLoaded, covRole }, actionButtons);
}
