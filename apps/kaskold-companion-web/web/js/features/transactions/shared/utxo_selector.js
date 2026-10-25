import { el, setContent } from '../../../core/ui/elements.js';
import { toast } from '../../../core/ui/toast.js';
import { sompiToKasFixed } from '../../../core/amounts.js';
import { utxoId } from './utxo_selection.js';

const SORT_MODES = new Set(['amount-desc', 'amount-asc', 'daa-desc', 'daa-asc']);

export function normalizeUtxoSortMode(value) {
    if (value === 'desc') return 'amount-desc';
    if (value === 'asc') return 'amount-asc';
    return SORT_MODES.has(value) ? value : 'amount-desc';
}

function compareExact(left, right, direction) {
    if (left === right) return 0;
    const ascending = left < right ? -1 : 1;
    return direction === 'asc' ? ascending : -ascending;
}

function compareEntries(mode) {
    const normalized = normalizeUtxoSortMode(mode);
    const [field, direction] = normalized.split('-');
    return (left, right) => {
        const leftValue = field === 'daa' ? (left.utxo.block_daa_score ?? 0n) : left.utxo.amount;
        const rightValue = field === 'daa' ? (right.utxo.block_daa_score ?? 0n) : right.utxo.amount;
        const primary = compareExact(leftValue, rightValue, direction);
        return primary || utxoId(left.utxo).localeCompare(utxoId(right.utxo));
    };
}

export function orderedUtxoEntries(utxos, mode) {
    return utxos.map((utxo, index) => ({ utxo, index })).sort(compareEntries(mode));
}

function renderRow(entry, selected) {
    const { utxo, index } = entry;
    const address = utxo.address || utxo.source_address || '';
    return el('div', { className: 'utxo-item', dataset: { idx: index, id: utxoId(utxo) } },
        el('span', { className: 'utxo-check' }, selected ? '☑' : '☐'),
        el('div', { className: 'u-grow' },
            el('div', { className: 'utxo-amount u-text-13px' }, `${sompiToKasFixed(utxo.amount)} KAS`),
            el('div', { className: 'utxo-detail' }, `${utxo.tx_id}:${utxo.index}`),
            el('div', { className: 'utxo-detail' }, `DAA ${utxo.block_daa_score ?? '—'}${address ? ` · ${address}` : ''}`)));
}

function heading(text, extraClass = '') {
    return el('div', { className: `utxo-detail${extraClass}` }, el('strong', {}, text));
}

export function renderUtxoSelector(list, utxos, selectedIds, options, onChange) {
    const selected = new Set(selectedIds || []);
    const limit = Math.max(1, Number(options?.limit) || 8);
    const mode = normalizeUtxoSortMode(options?.sort);
    const entries = orderedUtxoEntries(utxos, mode);
    const chosen = entries.filter(entry => selected.has(utxoId(entry.utxo)));
    const available = entries.filter(entry => !selected.has(utxoId(entry.utxo)));

    setContent(list,
        heading('SELECTED UTXOs'),
        chosen.length
            ? chosen.map(entry => renderRow(entry, true))
            : el('div', { className: 'utxo-detail' }, 'None selected — automatic selection will be used'),
        heading('AVAILABLE UTXOs', ' utxo-section-heading'),
        available.map(entry => renderRow(entry, false)));

    list.querySelectorAll('.utxo-item').forEach(item => {
        item.onclick = () => {
            const id = item.dataset.id;
            if (selected.has(id)) selected.delete(id);
            else if (selected.size >= limit) {
                toast(`Current manual-selection limit is ${limit} UTXOs`, 'info', 1800);
                return;
            } else selected.add(id);
            const ids = [...selected];
            onChange?.(ids);
            renderUtxoSelector(list, utxos, ids, options, onChange);
        };
    });
    list.style.display = '';
    list.classList.remove('hidden');
}
