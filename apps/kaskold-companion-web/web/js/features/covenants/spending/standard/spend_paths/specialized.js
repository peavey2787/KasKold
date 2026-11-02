// @ts-check
import { toast } from '../../../../../core/ui/toast.js';
import { getCovFee } from '../../../payload/state.js';
import { create_covenant_payjoin_claim } from '../../../boundary.js';

import { inputById } from '../../../../../core/dom.js';
import { runCovenantClaim } from './claim_controller.js';

export async function handleCovPayjoinClaim() {
    const covAddr = inputById('cov-payjoin-claim-addr').value.trim();
    const redeemHex = inputById('cov-payjoin-claim-script').value.trim();
    const mixAddr = inputById('cov-payjoin-claim-mix-addr').value.trim();
    const destAddr = inputById('cov-payjoin-claim-dest').value.trim();

    if (!covAddr) { toast('Enter covenant P2SH address', 'error'); return; }
    if (!redeemHex) { toast('Enter redeem script hex', 'error'); return; }
    if (!mixAddr) { toast('Enter your mixing address (must have UTXOs)', 'error'); return; }
    if (!destAddr) { toast('Enter destination address', 'error'); return; }

    const fee = getCovFee();
    await runCovenantClaim({
        loadingMessage: 'Building PayJoin claim PSKB...',
        errorLabel: 'PayJoin claim failed',
        logLabel: 'PayJoin claim PSKB',
        build: websocketUrl => create_covenant_payjoin_claim(
            covAddr,
            destAddr,
            redeemHex,
            mixAddr,
            fee,
            websocketUrl,
        ),
    });
}

