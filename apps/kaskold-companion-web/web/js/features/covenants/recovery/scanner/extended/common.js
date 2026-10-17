import { redeem_script_address } from '../../../../../wasm/api.js';

export function recoveredFromRedeem(type, redeemScriptHex, network, fields = {}) {
    if (!redeemScriptHex) return null;
    return {
        type,
        address: redeem_script_address(redeemScriptHex, network),
        redeem_script_hex: redeemScriptHex,
        loaded: true,
        ...fields,
    };
}
