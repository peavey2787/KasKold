import { walletSession } from '../../state/index.js';
import { oracleMbAskForNew, oracleMbSetFee } from '../../../features/oracle/model_b/controller.js';
import { navigateBack, showScreen } from '../../navigation.js';
import { covShowPanel } from '../../../features/covenants/generation/panel_keys.js';
import { covScanAddress } from '../../../features/covenants/scanning/pubkeys.js';
import {
    buildOracleV1Claim, openOracleV1Attest, publishOracleV1Beacon,
    scanOracleV1BindingResponse, scanOracleV1ClaimAttestation, scanOracleV1KeyResponse, scanOracleV1CovenantSignResponse,
    showOracleV1AttestationQr, showOracleV1BindingRequest, showOracleV1KeyRequest, showOracleV1SignRequest,
} from '../../../features/oracle/v1/controller.js';
// KasKold Companion Web — app/events/contracts/oracle
// Binds covenant navigation and oracle-model-B controls.

import { byId } from '../../../core/dom.js';


const ORACLE_MB_CLICK_BINDINGS = [
    ['btn-oracle-mb-back', () => covShowPanel('menu')],
    ['btn-oracle-mb-ask', () => oracleMbAskForNew()],
];

const ORACLE_V1_CLICK_BINDINGS = [
    ['btn-cov-scan-oracle-v1-bene', () => covScanAddress('cov-oracle-v1-bene', 'Scan beneficiary address', true)],
    ['btn-cov-oracle-v1-key-request', () => showOracleV1KeyRequest()],
    ['btn-cov-scan-oracle-v1-key', () => scanOracleV1KeyResponse()],
    ['btn-cov-res-oracle-v1-bind', () => showOracleV1BindingRequest()],
    ['btn-cov-res-oracle-v1-scan-binding', () => scanOracleV1BindingResponse()],
    ['btn-cov-res-oracle-v1-attest', () => openOracleV1Attest()],
    ['btn-cov-oracle-v1-scan-attestation', () => scanOracleV1ClaimAttestation()],
    ['btn-cov-scan-oracle-v1-claim-dest', () => covScanAddress('cov-oracle-v1-claim-dest', 'Scan claim destination', true)],
    ['btn-cov-oracle-v1-claim-create', () => buildOracleV1Claim()],
    ['btn-cov-oracle-v1-claim-back', () => covShowPanel('result')],
    ['btn-cov-oracle-v1-sign-request', () => showOracleV1SignRequest()],
    ['btn-cov-oracle-v1-scan-signed', () => scanOracleV1CovenantSignResponse()],
    ['btn-cov-oracle-v1-beacon', () => publishOracleV1Beacon()],
    ['btn-cov-oracle-v1-share', () => showOracleV1AttestationQr()],
    ['btn-cov-oracle-v1-attest-back', () => covShowPanel('result')],
];

function bindGuardedClicks(bindings) {
    for (const [buttonId, onClick] of bindings) {
        const button = byId(buttonId);
        if (button) button.onclick = onClick;
    }
}

function onOracleFeeCustomInput(input) {
    const value = input.value.trim();
    if (value === '') { oracleMbSetFee(1, false); return; }
    oracleMbSetFee(value, true);
}

// Oracle roll fee selector: presets (1/2/3 KAS) + a custom amount (min 1). The chosen total is the
// miner fee plus the 0.3 service fee; a bigger total raises the feerate so the roll clears a busy mempool.
function bindOracleFeeSelector() {
    document.querySelectorAll('.omb-fee-btn').forEach(b => {
        b.onclick = () => { const ci = byId('input-omb-fee-custom'); if (ci) ci.value = ''; oracleMbSetFee(b.getAttribute('data-omb-fee'), false); };
    });
    const customInput = byId('input-omb-fee-custom');
    if (customInput) customInput.oninput = () => onOracleFeeCustomInput(customInput);
    oracleMbSetFee(1, false);
}

export function bindOracleEvents() {
    // ─── Covenant++ handlers ───
    byId('btn-covenant').onclick = () => {
        document.querySelectorAll('#cov-menu .cov-category').forEach(category => category.classList.add('collapsed'));
        covShowPanel('menu');
        showScreen('covenant');
    };
    byId('btn-cov-back').onclick = () => navigateBack(walletSession.hasWallet() ? 'dashboard' : 'welcome');
    bindGuardedClicks(ORACLE_MB_CLICK_BINDINGS);
    bindOracleFeeSelector();
    bindGuardedClicks(ORACLE_V1_CLICK_BINDINGS);
}
