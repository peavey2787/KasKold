import { setContent, span } from '../../../../../../../../core/ui/elements.js';
import { covenantState } from '../../../../../../../../app/state/index.js';

export function renderEscrowResolved(status) {
    setContent(status, span('u-text-teal', '✅ Escrow resolved. Funds released.'));
    status.style.display = '';
}

export function renderEscrowEmpty(status) {
    if (covenantState.lastCovenantResult._escrowResolved) {
        renderEscrowResolved(status);
    } else {
        status.textContent = '⚖️ Awaiting deposit.';
        status.style.color = '';
    }
}

export function renderEscrowFunded(status, kas) {
    if (covenantState.lastCovenantResult._escrowDisputed) {
        const role = covenantState.lastCovenantResult._escrowDisputeRole || 'party';
        setContent(status, span('u-text-warning', `⚖️ Arbitration requested by ${role}. ${kas.toFixed(2)} KAS locked.`));
    } else {
        status.textContent = `⚖️ ${kas.toFixed(2)} KAS locked. Awaiting resolution.`;
        status.style.color = '';
    }
    status.style.display = '';
}
