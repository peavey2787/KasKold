// @ts-check
import { covShowPanel } from '../../../../features/covenants/generation/panel_keys.js';
import { covScanAddress, covScanPubkey } from '../../../../features/covenants/scanning.js';
import { handleShipEscrowSpend } from '../../../../features/covenants/spending/standard/shipment.js';
import { byId } from '../../../../core/dom.js';

const spendClick = (spendKind) => () => handleShipEscrowSpend(spendKind);
const scanPubkeyClick = (inputId, label) => () => covScanPubkey(inputId, label);

// Shipment-escrow operate + create scan buttons, bound in this order.
/** @type {[string, () => void][]} */
const SHIPMENT_CLICK_BINDINGS = [
    ['btn-cov-ship-back', () => covShowPanel('result')],
    ['btn-cov-ship-pickup', spendClick('pickup')],
    ['btn-cov-ship-s0-arb', spendClick('state0-arb-refund')],
    ['btn-cov-ship-s0-timeout', spendClick('state0-timeout')],
    ['btn-cov-ship-delivery', spendClick('delivery')],
    ['btn-cov-ship-s1-award', spendClick('state1-arb-award')],
    ['btn-cov-ship-s1-arb-refund', spendClick('state1-arb-refund')],
    ['btn-cov-ship-s1-timeout', spendClick('state1-timeout')],
    ['btn-cov-scan-ship-seller', scanPubkeyClick('cov-ship-seller-pk', 'Scan seller pubkey')],
    ['btn-cov-scan-ship-deliverer', scanPubkeyClick('cov-ship-deliverer-pk', 'Scan deliverer pubkey')],
    ['btn-cov-scan-ship-arbiter', scanPubkeyClick('cov-ship-arbiter-pk', 'Scan arbiter pubkey')],
    ['btn-cov-scan-ship-addr', () => covScanAddress('cov-ship-addr', 'Scan covenant address')],
];

export function bindShipmentEscrowEvents() {
    for (const [buttonId, onClick] of SHIPMENT_CLICK_BINDINGS) {
        const button = byId(buttonId);
        if (button) button.onclick = onClick;
    }
}
