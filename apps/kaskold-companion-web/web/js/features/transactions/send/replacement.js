// @ts-check
// KasKold Companion Web — features/transactions/send/replacement
import { pskt_finalize_and_broadcast, pskt_finalize_and_replace } from '../boundary.js';

/** The node already holds an unconfirmed transaction spending one of these coins. */
const PENDING_DOUBLE_SPEND = /already spent by transaction [0-9a-f]{64} in the mempool/i;

/**
 * Finalize and broadcast a signed PSKB. When an unconfirmed transaction
 * already spends one of its coins and `approveReplacement` agrees, submit it
 * in that transaction's place instead; the node accepts it only at a higher
 * feerate.
 * @param {string} wireHex
 * @param {string} wsUrl
 * @param {() => boolean} approveReplacement
 * @returns {Promise<string>} the transaction id
 */
export async function broadcastOrReplace(wireHex, wsUrl, approveReplacement) {
    try {
        return await pskt_finalize_and_broadcast(wireHex, wsUrl);
    } catch (error) {
        if (!PENDING_DOUBLE_SPEND.test(String(error?.message ?? error)) || !approveReplacement()) throw error;
        return pskt_finalize_and_replace(wireHex, wsUrl);
    }
}

/** Ask first: the transaction a replacement displaces is never sent. */
export function confirmReplacement() {
    return confirm(
        'An earlier transaction that has not confirmed yet already spends some of these coins. '
        + 'Replace it with this one? The earlier transaction will then never be sent.',
    );
}
