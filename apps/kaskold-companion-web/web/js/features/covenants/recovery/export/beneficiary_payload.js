import { bytesToHex } from '../../../../core/bytes.js';

const HEX_32 = /^[0-9a-f]{64}$/;

function addOracleBinding(invite, covenant) {
    if (!HEX_32.test(covenant.oracle_covenant_binding_token_hex || '')) {
        throw new Error('Bind the Oracle covenant key before sharing this covenant');
    }
    invite.okid = covenant.oracle_covenant_key_id_hex;
    invite.obt = covenant.oracle_covenant_binding_token_hex;
    invite.oas = covenant.attestation_statement;
}

// A covenant invite carries the script and address, which prove every term,
// plus only what no script can prove: calendar hints and the oracle's binding
// record and statement.
export function buildCovenantInvite(covenant) {
    const invite = {
        v: 1,
        t: 'cov-invite',
        ct: covenant.type || '',
        addr: covenant.address || '',
        rs: covenant.redeem_script_hex || '',
    };
    if (covenant.locktime_date_iso) invite.ldi = covenant.locktime_date_iso;
    if (covenant.type === 'global-allowance' && covenant.start_date_iso) invite.sdi = covenant.start_date_iso;
    if (covenant.type === 'oracle-v1') addOracleBinding(invite, covenant);
    return invite;
}

export function buildBeneficiaryExport(covenant) {
    const jsonBytes = new TextEncoder().encode(JSON.stringify(buildCovenantInvite(covenant)));
    const bytes = new Uint8Array(4 + jsonBytes.length);
    bytes.set(new TextEncoder().encode('COVI'), 0);
    bytes.set(jsonBytes, 4);
    return Object.freeze({ bytes, hex: bytesToHex(bytes), encrypted: false, extension: '.cov' });
}
