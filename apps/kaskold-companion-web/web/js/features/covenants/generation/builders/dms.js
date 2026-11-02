import { networkState } from '../../../../app/state/index.js';
import { toast } from '../../../../core/ui/toast.js';
import { covenant_dms, decode_address } from '../../../../wasm/api.js';
// dms covenant builders.

import { byId } from '../../../../core/dom.js';
import { exactJsonStringify, exactUnsigned } from '../../../../core/exact.js';

// Decodes a Kaspa address to its x-only pubkey. Returns null after toasting on failure.
function decodeHeirAddress(address) {
    try {
        const decoded = JSON.parse(decode_address(address));
        if (decoded.version !== 0) {
            toast('Heir must be a standard address (P2PK), not a script address', 'error'); return null;
        }
        if (!decoded.payload || decoded.payload.length !== 64) {
            toast('Could not read pubkey from that address', 'error'); return null;
        }
        byId('cov-dms2-heir-pk').value = decoded.payload;
        return decoded.payload;
    } catch (e) {
        toast('Invalid address: ' + e, 'error'); return null;
    }
}

// Heir is given as a single Kaspa address. Returns the x-only pubkey, or null after toasting.
function resolveHeirPk(raw) {
    if (raw.startsWith('kpub1:')) {
        toast('Paste the heir address, not a kpub', 'error'); return null;
    }
    if (raw.includes(':')) return decodeHeirAddress(raw); // an address; Portal decodes and validates it
    return raw;
}

// Converts an inactivity duration in seconds to DAA units (10 BPS).
function parseInactivityDaa() {
    let durationSec;
    try { durationSec = exactUnsigned(byId('cov-dms2-duration').value.trim() || '0', 'inactivity seconds'); }
    catch { durationSec = 0n; }
    return durationSec;
}

// Injects inactivity_daa into the result for storage.
function withInactivityDaa(resultJson, inactivityDaa) {
    const dmsResult = JSON.parse(resultJson);
    dmsResult.inactivity_daa = inactivityDaa;
    return exactJsonStringify(dmsResult);
}

export async function buildDms(ownerPk) {
    const heirPk = resolveHeirPk(byId('cov-dms2-heir-pk').value.trim());
    if (heirPk === null) return;
    const durationSec = parseInactivityDaa();
    if (durationSec <= 0n) { toast('Set an inactivity period', 'error'); return; }
    const inactivityDaa = durationSec * 10n;
    if (!ownerPk) { toast('Load wallet first', 'error'); return; }
    if (!heirPk || heirPk.length !== 64) { toast('Enter the heir Kaspa address', 'error'); return; }
    const resultJson = withInactivityDaa(covenant_dms(ownerPk, heirPk, inactivityDaa, networkState.network), inactivityDaa);
    return { resultJson, extra: { heir_pubkey_hex: heirPk } };
}
