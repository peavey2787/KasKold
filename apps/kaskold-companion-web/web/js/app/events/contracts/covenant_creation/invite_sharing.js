import { covenantState } from '../../../state/index.js';
import { toast } from '../../../../core/ui/toast.js';
import { showStaticPayloadQr } from '../../../../features/qr_display/static_payload.js';
import { el } from '../../../../core/ui/elements.js';
// Focused covenant invite sharing event registration.

import { byId } from '../../../../core/dom.js';
import { buildCovenantInvite } from '../../../../features/covenants/recovery/export/beneficiary_payload.js';

function registerCovenantInviteShare() {
    const button = byId('btn-cov-res-share-cov');
    if (!button) return;
    button.onclick = () => {
        const result = covenantState.lastCovenantResult;
        if (!result) return;
        try {
            if (result.type === 'additive') {
                const address = result.address || '';
                const details = el('div', { className: 'covenant-invite-details' },
                    'Piggy Bank Address', el('br'),
                    el('span', { className: 'u-text-10px-break-all' }, address), el('br'),
                    el('span', { className: 'u-text-10px-text-text-muted' }, 'Anyone can send KAS to this address'));
                showStaticPayloadQr(address, 'Share Piggy Bank Address', details);
                return;
            }
            showStaticPayloadQr(JSON.stringify(buildCovenantInvite(result)), 'Covenant Invite QR');
        } catch (error) {
            toast('QR generation failed: ' + error, 'error');
        }
    };
}

function registerGenericInviteActions() {
    registerCovenantInviteShare();
}

export function registerInviteSharingActions() {
    registerGenericInviteActions();
}
