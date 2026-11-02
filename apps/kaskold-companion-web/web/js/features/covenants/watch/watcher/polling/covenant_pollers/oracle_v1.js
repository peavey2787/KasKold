// @ts-check
import {
    covenantState,
    covenantWatcherState,
    networkState,
    verify_oracle_v1_attestation,
} from '../../../../boundary.js';
import { hexToBytes } from '../../../../../../core/bytes.js';
import { kaspaRestApiBase } from '../../../../../../core/config/network.js';
import { formatDaaDuration } from '../../../../../../core/format.js';
import { el, setContent, span } from '../../../../../../core/ui/elements.js';
import { oracleV1MessageCommitment } from '../../../../../oracle/v1/attestation.js';
import { covSaveActive } from '../../../../recovery/active.js';

const MAGIC = '4f525631';
const HEADER_HEX = 8 + 128 + 64;
const MAX_CHECKED_TXIDS = 64;
const TXID_PATTERN = /^[0-9a-f]{64}$/i;
const SIGNATURE_HEX_PATTERN = /^[0-9a-f]{128}$/;
const COMMITMENT_HEX_PATTERN = /^[0-9a-f]{64}$/;

function checkedTxids(result) {
    return Array.isArray(result._oracle_v1_checked_txids)
        ? result._oracle_v1_checked_txids.filter(txid => TXID_PATTERN.test(txid)).slice(-MAX_CHECKED_TXIDS)
        : [];
}

function markChecked(result, transactionId) {
    const txids = checkedTxids(result).filter(txid => txid !== transactionId);
    txids.push(transactionId);
    result._oracle_v1_checked_txids = txids.slice(-MAX_CHECKED_TXIDS);
}

function isCandidateTxid(transactionId, result) {
    return TXID_PATTERN.test(transactionId || '') && !checkedTxids(result).includes(transactionId);
}

async function fetchTransactionPayload(transactionId) {
    const response = await fetch(`${kaspaRestApiBase(networkState.network)}/transactions/${transactionId}`, {
        signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return null;
    return String((await response.json()).payload || '').toLowerCase();
}

function decodeStatementHex(textHex) {
    if (textHex.length % 2 !== 0) return null;
    return new TextDecoder('utf-8', { fatal: true }).decode(hexToBytes(textHex));
}

async function parseBeaconPayload(payload, result) {
    if (!payload.startsWith(MAGIC) || payload.length < HEADER_HEX) return null;
    const signature = payload.slice(8, 136);
    const commitment = payload.slice(136, 200);
    const text = decodeStatementHex(payload.slice(200));
    if (text === null) return null;
    if (text !== (result.attestation_statement || '')) return null;
    if (commitment !== (result.message_commitment_hex || '').toLowerCase()) return null;
    if (await oracleV1MessageCommitment(text) !== commitment) return null;
    if (!verify_oracle_v1_attestation(result.oracle_pubkey_hex || '', signature, commitment)) return null;
    return { signature, commitment, text };
}

async function fetchBeacon(transactionId, result) {
    if (!isCandidateTxid(transactionId, result)) return null;
    try {
        const payload = await fetchTransactionPayload(transactionId);
        if (payload === null) return null;
        markChecked(result, transactionId);
        const parsed = await parseBeaconPayload(payload, result);
        return parsed ? { ...parsed, transactionId } : null;
    } catch {
        return null;
    }
}

async function hasValidSavedAttestation(result) {
    const signature = String(result.oracle_attestation_signature || '').toLowerCase();
    const commitment = String(result.oracle_attestation_commitment || '').toLowerCase();
    const expected = String(result.message_commitment_hex || '').toLowerCase();
    const statement = String(result.attestation_statement || '');
    if (!SIGNATURE_HEX_PATTERN.test(signature) || !COMMITMENT_HEX_PATTERN.test(commitment)) return false;
    if (commitment !== expected || await oracleV1MessageCommitment(statement) !== commitment) return false;
    return verify_oracle_v1_attestation(result.oracle_pubkey_hex || '', signature, commitment);
}

function saveBeacon(result, beacon) {
    result.oracle_attestation_signature = beacon.signature;
    result.oracle_attestation_commitment = beacon.commitment;
    result.oracle_attestation_text = beacon.text;
    result.oracle_attestation_txid = beacon.transactionId;
    const active = covenantState.activeCovenants.find(item => item.address === result.address);
    if (active) {
        active.oracle_attestation_signature = beacon.signature;
        active.oracle_attestation_commitment = beacon.commitment;
        active.oracle_attestation_text = beacon.text;
        active.oracle_attestation_txid = beacon.transactionId;
        active._oracle_v1_checked_txids = [beacon.transactionId];
        covSaveActive();
    }
}

// If persisted attestation fields were corrupted, permit its known txid to
// be fetched again instead of trusting or permanently blacklisting state.
function forgetCheckedAttestationTxid(result) {
    if (!result.oracle_attestation_txid) return;
    result._oracle_v1_checked_txids = checkedTxids(result)
        .filter(txid => txid !== result.oracle_attestation_txid);
}

async function scanUtxosForBeacon(utxos, result) {
    for (const utxo of utxos) {
        const beacon = await fetchBeacon(String(utxo?.tx_id || '').toLowerCase(), result);
        if (beacon) {
            saveBeacon(result, beacon);
            return;
        }
    }
}

async function resolveAttestation(result, utxos) {
    if (await hasValidSavedAttestation(result)) return true;
    forgetCheckedAttestationTxid(result);
    await scanUtxosForBeacon(utxos, result);
    return hasValidSavedAttestation(result);
}

function isSpentCovenant(total) {
    const lastBalance = covenantWatcherState._covWatcherLastBalance;
    return total === 0n && lastBalance !== null && lastBalance > 0n;
}

function reportSpent(st) {
    setContent(st, span('u-text-teal', '✅ Oracle covenant spent.'));
    st.style.display = '';
}

function reportNotFunded(st) {
    st.textContent = '👁 0 KAS | Not funded';
    st.style.color = '';
}

function refundLabel(locktime, currentDaa) {
    if (!(locktime > 0n && currentDaa > 0n)) return 'Owner refund timer active';
    return currentDaa >= locktime
        ? 'Owner refund available now'
        : `Owner refund in ~${formatDaaDuration(locktime - currentDaa)}`;
}

function renderOracleStatus(st, { attested, kas, refund }) {
    if (attested) {
        setContent(st, span('u-text-teal', `✅ ${kas} KAS | Oracle attested | Beneficiary claim available`), el('br'), refund);
    } else {
        st.textContent = `👁 ${kas} KAS | Waiting for oracle attestation | ${refund}`;
        st.style.color = '';
    }
    st.style.display = '';
}

export async function pollOracleV1(state) {
    const { total, kas, st, locktime, currentDaa, utxos } = state;
    const result = covenantState.lastCovenantResult;
    if (isSpentCovenant(total)) {
        reportSpent(st);
        return true;
    }
    if (total === 0n) {
        reportNotFunded(st);
        return false;
    }

    const attested = await resolveAttestation(result, utxos);
    const refund = refundLabel(locktime, currentDaa);
    renderOracleStatus(st, { attested, kas, refund });
    return false;
}
