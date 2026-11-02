import { networkState } from '../../../../app/state/index.js';
import { resolveFutureDaa } from '../../../../core/node/future_daa.js';
import { toast } from '../../../../core/ui/toast.js';
import { covenant_additive_address, covenant_timelocked_savings, decode_address } from '../../../../wasm/api.js';
// savings covenant builders.

import { byId } from '../../../../core/dom.js';
import { kasToSompi } from '../../../../core/amounts.js';
import { exactUnsigned } from '../../../../core/exact.js';
export async function buildAdditive(ownerPk) {
    let resultJson;
    let extra = {};
    const goalStr = byId('cov-piggy-goal').value.trim();
    let sompi = 0n;
    if (goalStr) {
        try {
            sompi = kasToSompi(goalStr);
            if (sompi <= 0n) throw new Error('Goal must be positive');
        } catch (_) {
            toast('Enter a valid positive savings goal in KAS', 'error');
            return;
        }
    }
    // Deadline: date picker to DAA
    let deadlineDaa = 0n;
    const dateVal = byId('cov-piggy-deadline') ? byId('cov-piggy-deadline').value : '';
    if (dateVal) {
        try {
            deadlineDaa = (await resolveFutureDaa(dateVal)).daa;
        } catch (error) {
            toast(error.message, 'error');
            return;
        }
    }
    resultJson = covenant_additive_address(ownerPk, sompi, deadlineDaa, networkState.network);
    if (dateVal) extra.deadline_date_iso = new Date(dateVal).toISOString();
    return { resultJson, extra };
}

// Recovery wallet resolution. Blank reuses the loaded wallet (no separate backup).
function checkRecoveryPkLength(pk) {
    if (pk.length !== 64) { toast('Recovery wallet pubkey must be 64 hex chars (or leave blank)', 'error'); return null; }
    return pk;
}

// An address is decoded and validated by Portal; returns its pubkey hex or null.
function decodeRecoveryAddressPk(address) {
    try {
        const decoded = JSON.parse(decode_address(address));
        if (decoded.version !== 0) { toast('Recovery wallet must be a standard address (P2PK)', 'error'); return null; }
        if (!decoded.payload || decoded.payload.length !== 64) { toast('Could not read pubkey from that address', 'error'); return null; }
        return decoded.payload;
    } catch (e) { toast('Invalid address: ' + e, 'error'); return null; }
}

function parseRecoveryInput(raw) {
    if (raw.startsWith('kpub1:')) { toast('Paste the recovery wallet address, not a kpub', 'error'); return null; }
    return raw.includes(':') ? decodeRecoveryAddressPk(raw) : raw;
}

function resolveSavingsRecoveryPk(ownerPk) {
    const field = byId('cov-savings-recovery-pk');
    const raw = field ? field.value.trim() : '';
    if (!raw) return checkRecoveryPkLength(ownerPk);
    const pk = parseRecoveryInput(raw);
    if (!pk) return null;
    if (pk !== raw) field.value = pk;
    return checkRecoveryPkLength(pk);
}

// Datetime-to-DAA (10 BPS), same conversion as the vault. Returns BigInt DAA or null.
async function resolveSavingsLocktimeDaa() {
    const field = byId('cov-savings-locktime');
    let locktime = field ? field.value.trim() : '';
    const dtEl = byId('cov-savings-datetime');
    if (dtEl && dtEl.value && !locktime) {
        try {
            locktime = String((await resolveFutureDaa(dtEl.value)).daa);
            if (field) field.value = locktime;
        } catch (error) {
            toast(error.message, 'error');
            return null;
        }
    }
    let locktimeDaa;
    try { locktimeDaa = exactUnsigned(locktime, 'unlock DAA'); } catch (_) { locktimeDaa = 0n; }
    if (locktimeDaa <= 0n) { toast('Set an unlock date (or DAA score)', 'error'); return null; }
    return locktimeDaa;
}

// Deposit-and-lock savings. wallet1 = the loaded wallet (primary).
// wallet2 = optional independent recovery wallet. No owner-spend-anytime
// branch: frozen for everyone until the date, then 1-of-2 claim.
export async function buildTimelockedSavings(ownerPk) {
    if (!ownerPk) { toast('Load wallet first', 'error'); return; }
    const recoveryPk = resolveSavingsRecoveryPk(ownerPk);
    if (!recoveryPk) return;
    const locktimeDaa = await resolveSavingsLocktimeDaa();
    if (locktimeDaa === null) return;
    const resultJson = covenant_timelocked_savings(ownerPk, recoveryPk, locktimeDaa, networkState.network);
    const extra = { wallet1_pubkey_hex: ownerPk, wallet2_pubkey_hex: recoveryPk };
    const sDtEl = byId('cov-savings-datetime');
    const sDtVal = sDtEl ? sDtEl.value : '';
    if (sDtVal) extra.locktime_date_iso = new Date(sDtVal).toISOString();
    return { resultJson, extra };
}
