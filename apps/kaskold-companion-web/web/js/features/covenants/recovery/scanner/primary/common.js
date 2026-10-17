import { networkState } from '../../../../../app/state/index.js';
import { redeem_script_address } from '../../../../../wasm/api.js';
import { readLen } from '../payload_reader.js';

export function readStoredScript(params) {
    const { len, endOff } = readLen(params, 0);
    const redeemScriptHex = params.slice(endOff, endOff + len * 2);
    if (!redeemScriptHex) throw new Error('Recovered covenant has an empty redeem script');
    return { redeemScriptHex, offset: endOff + len * 2 };
}

function addressForScript(redeemScriptHex) {
    return redeem_script_address(redeemScriptHex, networkState.network);
}

export function baseRecoveredRecord(type, redeemScriptHex, role = 'owner') {
    return {
        type,
        address: addressForScript(redeemScriptHex),
        redeem_script_hex: redeemScriptHex,
        loaded: true,
        role,
    };
}

export function normalizedCovenantId(value) {
    return value && !/^0+$/.test(value) ? value : '';
}
