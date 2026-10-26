import { walletMatchesPk } from '../../generation/ui_and_keys.js';

// The key that makes this wallet the owner of a two-party covenant.
const OWNER_KEY = Object.freeze({
    dms: 'owner_pubkey_hex',
    'timelocked-savings': 'wallet1_pubkey_hex',
    'global-allowance': 'owner_pubkey_hex',
    payjoin: 'owner_pubkey_hex',
});

function escrowRole(entry) {
    if (walletMatchesPk(entry.alice_pk)) return 'owner';
    if (walletMatchesPk(entry.arbiter_pk)) return 'arbiter';
    return 'beneficiary';
}

function oracleRole(entry) {
    if (walletMatchesPk(entry.beneficiary_pubkey_hex)) return 'beneficiary';
    if (walletMatchesPk(entry.owner_pubkey_hex)) return 'owner';
    return 'observer';
}

// This wallet's role in a covenant whose keys were read from its script.
export function inferCovenantRole(entry) {
    if (entry.type === 'escrow') return escrowRole(entry);
    if (entry.type === 'oracle-v1') return oracleRole(entry);
    const ownerKey = OWNER_KEY[entry.type];
    if (!ownerKey) return 'owner';
    return walletMatchesPk(entry[ownerKey]) ? 'owner' : 'beneficiary';
}
