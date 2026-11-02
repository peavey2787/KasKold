import { networkState, covenant_payjoin } from '../../../boundary.js';
import { resolveFutureDaa } from '../../../../../core/node/future_daa.js';
import { toast } from '../../../../../core/ui/toast.js';
// payjoin covenant builder.

import { byId } from '../../../../../core/dom.js';
import { exactUnsigned } from '../../../../../core/exact.js';
import { addressToXOnly } from '../../../../../core/address.js';

async function resolvePayjoinLocktime(pjDatetime) {
    const locktime = byId('cov-payjoin-locktime').value.trim();
    if (!pjDatetime || locktime) return locktime;
    try {
        const resolved = String((await resolveFutureDaa(pjDatetime)).daa);
        byId('cov-payjoin-locktime').value = resolved;
        return resolved;
    } catch (error) {
        toast(error.message, 'error');
        return null;
    }
}

function parseLocktimeDaa(locktime) {
    try { return exactUnsigned(locktime, 'refund timeout DAA'); } catch { return 0n; }
}

function payjoinExtra(benePk, minInputs, minOutputs, pjDatetime) {
    const extra = {};
    extra.beneficiary_pubkey_hex = benePk;
    extra.min_inputs = parseInt(minInputs);
    extra.min_outputs = parseInt(minOutputs);
    if (pjDatetime) extra.locktime_date_iso = new Date(pjDatetime).toISOString();
    return extra;
}

export async function buildPayjoin(ownerPk) {
    const benePk = addressToXOnly(byId('cov-payjoin-bene-pk').value);
    const pjDatetime = byId('cov-payjoin-datetime') ? byId('cov-payjoin-datetime').value : '';
    const locktime = await resolvePayjoinLocktime(pjDatetime);
    if (locktime === null) return;
    const minInputs = byId('cov-payjoin-min-inputs').value.trim() || '2';
    const minOutputs = byId('cov-payjoin-min-outputs').value.trim() || '2';
    if (!ownerPk) { toast('Load wallet first', 'error'); return; }
    if (!benePk || benePk.length !== 64) { toast('Enter beneficiary pubkey (64 hex chars)', 'error'); return; }
    const locktimeDaa = parseLocktimeDaa(locktime);
    if (locktimeDaa <= 0n) { toast('Pick a refund timeout date', 'error'); return; }
    const resultJson = covenant_payjoin(ownerPk, benePk, locktimeDaa, BigInt(minInputs), BigInt(minOutputs), networkState.network);
    return { resultJson, extra: payjoinExtra(benePk, minInputs, minOutputs, pjDatetime) };
}
