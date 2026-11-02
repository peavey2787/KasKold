// @ts-check
import { oracleState } from '../../../boundary.js';
import { byId, inputById } from '../../../../../core/dom.js';

export function createOracleAskUi() {
    const status = byId('oracle-mb-ask-status');
    const button = inputById('btn-oracle-mb-ask');
    return {
        show(message, color = 'var(--teal)') {
            if (!status) return;
            status.style.display = 'block';
            status.textContent = message;
            status.style.color = color;
        },
        begin() {
            if (oracleState._oracleMbAskBusy) return false;
            oracleState._oracleMbAskBusy = true;
            if (button) { button.disabled = true; button.style.opacity = '0.6'; }
            return true;
        },
        finish() {
            oracleState._oracleMbAskBusy = false;
            if (button) { button.disabled = false; button.style.opacity = ''; }
        },
    };
}
