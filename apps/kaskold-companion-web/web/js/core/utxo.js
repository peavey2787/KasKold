// @ts-check
import { exactU64, exactUnsigned } from './exact.js';

/**
 * @param {Record<string, any>} utxo
 */
export function utxoTransactionId(utxo) {
    return utxo.tx_id
        ?? utxo.transactionId
        ?? utxo.outpoint?.transactionId
        ?? utxo.previousOutpoint?.transactionId
        ?? null;
}

/**
 * @param {Record<string, any>} utxo
 */
export function normalizeUtxo(utxo) {
    return {
        ...utxo,
        amount: exactU64(utxo.amount, 'UTXO amount'),
        block_daa_score: exactU64(utxo.block_daa_score ?? 0n, 'UTXO DAA score'),
    };
}

/**
 * @param {Array<Record<string, any>>} utxos
 */
export function normalizeUtxos(utxos) {
    return utxos.map(normalizeUtxo);
}

/**
 * @param {string} json
 */
export function parseUtxosJson(json) {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) throw new Error('UTXO response must be an array');
    return normalizeUtxos(parsed);
}

/**
 * @param {Array<Record<string, any>>} utxos
 */
export function sortUtxosLargestFirst(utxos) {
    utxos.sort((left, right) => {
        const leftAmount = exactUnsigned(left.amount, 'UTXO amount');
        const rightAmount = exactUnsigned(right.amount, 'UTXO amount');
        if (leftAmount === rightAmount) {
            const txOrder = String(left.tx_id ?? '').localeCompare(String(right.tx_id ?? ''));
            return txOrder || Number(left.index ?? 0) - Number(right.index ?? 0);
        }
        return leftAmount > rightAmount ? -1 : 1;
    });
    return utxos;
}
