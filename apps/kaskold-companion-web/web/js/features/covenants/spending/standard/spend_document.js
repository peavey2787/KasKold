// @ts-check
import { exactJsonStringify } from '../../../../core/exact.js';
import { covenant_spend_document } from '../../boundary.js';

// KasKold Companion Web — one covenant spend the Companion shapes itself. Kaspa
// Portal encodes it, so it always names a path the script can run.
export function encodeCovenantSpend({ covAddr, redeemHex, path, utxos, outputs, txVersion = 1, lockTime = 0n, payloadHex = '' }) {
    return covenant_spend_document(exactJsonStringify({
        covenant_address: covAddr,
        redeem_script_hex: redeemHex,
        path,
        tx_version: txVersion,
        lock_time: lockTime,
        payload_hex: payloadHex,
        utxos: utxos.map(({ tx_id, index, amount }) => ({ tx_id, index, amount })),
        outputs,
    }));
}
