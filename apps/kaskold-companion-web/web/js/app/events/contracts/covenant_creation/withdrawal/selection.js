// @ts-check
import { checkboxesIn, elementById } from '../../../../../core/dom.js';
import { exactJsonStringify, exactUnsigned } from '../../../../../core/exact.js';

export function selectedUtxos() {
    const list = elementById('cov-consol-list');
    const utxos = JSON.parse(list.dataset.utxos || '[]');
    return Array.from(checkboxesIn(list))
        .filter(checkbox => checkbox.checked)
        .map(checkbox => utxos[Number.parseInt(String(checkbox.dataset.utxoIdx), 10)]);
}

export function setAllSelected(selected) {
    checkboxesIn(elementById('cov-consol-list'))
        .forEach(checkbox => { checkbox.checked = selected; });
}

export function selectedUtxosJson(utxos) {
    return exactJsonStringify(utxos.map(utxo => ({
        tx_id: utxo.tx_id,
        index: utxo.index,
        amount: exactUnsigned(utxo.amount, 'selected UTXO sompi'),
        block_daa_score: exactUnsigned(utxo.block_daa_score ?? 0n, 'selected UTXO DAA'),
    })));
}
