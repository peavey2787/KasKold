import { networkState } from '../../../app/state/index.js';
import { covenant_script_terms } from '../../../wasm/api.js';
import { parseCovenantJson } from './exact_fields.js';

// Covenant families whose terms are read from the redeem script in WASM.
// Any record of these types shows only what its script proves.
export const SCRIPT_TERM_TYPES = Object.freeze(new Set([
    'dms', 'timelocked-savings', 'additive', 'escrow',
    'global-allowance', 'global-spending-limit', 'payjoin', 'oracle-v1',
]));

// Terms of a covenant record as proven by its script; throws when the script
// is not that family's, does not hash to the record's address on the current
// network, or (oracle-v1) the statement is not the one it commits to.
export function verifiedCovenantTerms(record) {
    return parseCovenantJson(covenant_script_terms(
        record.type,
        record.redeem_script_hex,
        record.address,
        networkState.network,
        record.attestation_statement || '',
    ));
}

// Overwrite a record's terms with the ones its script proves.
export function applyScriptTerms(record) {
    if (!record || !SCRIPT_TERM_TYPES.has(record.type)) return record;
    return Object.assign(record, verifiedCovenantTerms(record));
}
