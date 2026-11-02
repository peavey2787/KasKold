import { networkState } from '../../../../../app/state/index.js';
import { resolveFutureDaa } from '../../../../../core/node/future_daa.js';
import { toast } from '../../../../../core/ui/toast.js';
import { covenant_commit_reveal } from '../../../../../wasm/api.js';
// commit-reveal covenant builder.

import { byId } from '../../../../../core/dom.js';
import { exactUnsigned } from '../../../../../core/exact.js';

function readCommitHash() {
    const hashRaw = byId('cov-cr-hash-display').textContent.trim();
    return hashRaw.startsWith('BLAKE2B: ') ? hashRaw.slice(9) : hashRaw;
}

async function resolveLocktime(crDatetime) {
    const locktimeInput = byId('cov-cr-locktime');
    let locktime = locktimeInput.value.trim();
    if (!crDatetime || locktime) return locktime;
    try {
        locktime = String((await resolveFutureDaa(crDatetime)).daa);
        locktimeInput.value = locktime;
        return locktime;
    } catch (error) {
        toast(error.message, 'error');
        return null;
    }
}

function parseLocktimeDaa(locktime) {
    try { return exactUnsigned(locktime, 'refund timeout DAA'); } catch { return 0n; }
}

function buildCommitExtra(hashDisplay, crDatetime) {
    const extra = { commit_hash: hashDisplay };
    // Store ECIES ciphertext only (parts are never persisted in browser)
    const ctHex = byId('cov-cr-ciphertext-hex') ? byId('cov-cr-ciphertext-hex').value : '';
    if (ctHex) extra.cr_ciphertext_hex = ctHex;
    if (crDatetime) extra.locktime_date_iso = new Date(crDatetime).toISOString();
    return extra;
}

export async function buildCommitReveal(ownerPk) {
    if (!ownerPk) { toast('Load wallet first', 'error'); return; }
    const hashDisplay = readCommitHash();
    if (!hashDisplay || hashDisplay.length !== 64) { toast('Scan commitment from KasKold first', 'error'); return; }
    const crDatetime = byId('cov-cr-datetime') ? byId('cov-cr-datetime').value : '';
    const locktime = await resolveLocktime(crDatetime);
    if (locktime === null) return;
    const locktimeDaa = parseLocktimeDaa(locktime);
    if (locktimeDaa <= 0n) { toast('Pick a refund timeout date', 'error'); return; }
    const resultJson = covenant_commit_reveal(ownerPk, hashDisplay, locktimeDaa, networkState.network);
    return { resultJson, extra: buildCommitExtra(hashDisplay, crDatetime) };
}
