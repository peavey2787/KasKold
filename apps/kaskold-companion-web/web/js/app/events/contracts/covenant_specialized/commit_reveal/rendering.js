// @ts-check
import { byId, elementById, inputById } from '../../../../../core/dom.js';

const RESULT_FIELDS = [
    'cov-cr-verify-preimage',
    'cov-cr-verify-hash',
    'cov-cr-verify-computed',
    'cov-cr-verify-match',
    'cov-cr-verify-time',
];

export function clearVerificationResult() {
    for (const id of RESULT_FIELDS) {
        const element = byId(id);
        if (element) element.textContent = '';
    }
    const result = byId('cov-cr-verify-result');
    if (result) result.style.display = 'none';
    const txid = inputById('cov-cr-verify-txid');
    if (txid) txid.value = '';
}

export function renderVerificationResult(result) {
    elementById('cov-cr-verify-result').style.display = '';
    elementById('cov-cr-verify-preimage').textContent = result.preimageText;
    elementById('cov-cr-verify-hash').textContent = result.committedHash;
    elementById('cov-cr-verify-computed').textContent = result.computedHash;
    const match = elementById('cov-cr-verify-match');
    match.textContent = result.matches
        ? '✅ HASH MATCH — Commitment verified'
        : '❌ HASH MISMATCH — Invalid revelation';
    match.style.background = result.matches ? 'rgba(78,205,196,0.15)' : 'rgba(255,82,82,0.15)';
    match.style.color = result.matches ? 'var(--teal)' : '#ff5252';
    elementById('cov-cr-verify-time').textContent = result.timestamp;
}
