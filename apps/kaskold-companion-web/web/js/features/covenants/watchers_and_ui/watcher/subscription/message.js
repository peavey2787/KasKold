import { bytesToHex } from '../../../../../core/bytes.js';
import { covenant_spend_path } from '../../../../../wasm/api.js';

// owner, heir or unknown: Kaspa Portal reads the branch selector.
export function detectSpendPath(signature, redeemScriptHex) {
    const path = covenant_spend_path(bytesToHex(signature), redeemScriptHex);
    console.log(`[Companion] Covenant spend path: ${path}`);
    return path;
}
