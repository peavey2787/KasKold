import { el, setContent } from '../../../../core/ui/elements.js';
import { covenantState } from '../../../state/index.js';
import { resolveNodeUrl } from '../../../../core/node/resolver.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../features/covenants/generation/panel_keys.js';
import { fetch_utxos_for_address_js } from '../../../../wasm/api.js';
// KasKold Companion Web — covenant UTXO picker and selection summary.

import { byId } from '../../../../core/dom.js';
import { sortUtxosLargestFirst } from '../../../../core/utxo.js';
import { sompiToKasFixed } from '../../../../core/amounts.js';
import { exactJsonStringify } from '../../../../core/exact.js';
const PIGGY_BREAK_LABELS = {
    title: 'Break Piggy Bank',
    desc: 'Sweep the piggy to your address. All UTXOs are selected. Deselect some to break in smaller batches if the QR is too large for your KasKold. Owner signature required.',
};
const STANDARD_PICKER_LABELS = {
    title: 'Select UTXOs',
    desc: 'Select UTXOs to consolidate or withdraw. Owner signature required.',
};

function applyPickerLabels(isPiggyBreak) {
    const labels = isPiggyBreak ? PIGGY_BREAK_LABELS : STANDARD_PICKER_LABELS;
    const titleEl = byId('cov-consol-title');
    const descEl = byId('cov-consol-desc');
    if (titleEl) titleEl.textContent = labels.title;
    if (descEl) descEl.textContent = labels.desc;
}

function buildUtxoRow(utxo, idx) {
    const kas = sompiToKasFixed(utxo.amount, 4);
    const txShort = utxo.tx_id.substring(0, 8) + '...' + utxo.tx_id.substring(utxo.tx_id.length - 6);
    const row = document.createElement('label');
    row.classList.add('covenant-utxo-option');
    row.append(
        el('input', { className: 'covenant-utxo-checkbox', attrs: { type: 'checkbox' }, checked: true, dataset: { utxoIdx: idx } }),
        el('div', { className: 'u-grow' },
            el('div', { className: 'covenant-utxo-amount' }, `${kas} KAS`),
            el('div', { className: 'covenant-utxo-outpoint' }, `${txShort}:${utxo.index}`)));
    return row;
}

function renderUtxoRows(listEl, utxos) {
    utxos.forEach((utxo, idx) => listEl.appendChild(buildUtxoRow(utxo, idx)));
}

function bindSummaryOnce(listEl) {
    // Reopening the picker must not stack another summary listener.
    if (listEl.dataset.summaryBound) return;
    listEl.dataset.summaryBound = 'true';
    listEl.addEventListener('change', () => updateConsolSummary());
}

export async function openUtxoPicker(defaultDest, beneClaim = null) {
        if (!covenantState.lastCovenantResult) { toast('No covenant loaded', 'error'); return; }
        covenantState._pickerBeneClaim = beneClaim || null; // beneficiary timeout claim vs owner sweep
        const covAddr = covenantState.lastCovenantResult.address;
        const listEl = byId('cov-consol-list');
        setContent(listEl, el('div', { className: 'covenant-utxo-loading' }, 'Loading UTXOs...'));
        byId('cov-consol-dest').value = defaultDest || covAddr;
        // Additive piggy break: this is a withdrawal (sweep to your address), not a
        // consolidation. Default-select all, and explain that deselecting lets you
        // break in smaller batches if a full sweep makes too large a QR for KasKold.
        const _isPiggyBreak = (covenantState.lastCovenantResult.type === 'additive') && defaultDest && defaultDest !== covAddr;
        applyPickerLabels(_isPiggyBreak);
        covShowPanel('consolidate');
        try {
            const wsUrl = await resolveNodeUrl();
            const utxosJson = await fetch_utxos_for_address_js(covAddr, wsUrl);
            const utxos = JSON.parse(utxosJson);
            if (utxos.length < 1) { toast('No UTXOs at covenant address', 'error'); covShowPanel('result'); return; }
            listEl.replaceChildren();
            sortUtxosLargestFirst(utxos);
            renderUtxoRows(listEl, utxos);
            listEl.dataset.utxos = exactJsonStringify(utxos);
            updateConsolSummary();
            bindSummaryOnce(listEl);
        } catch (e) {
            toast('Error loading UTXOs: ' + e, 'error');
            covShowPanel('result');
        }
    }

export function updateConsolSummary(_context) {
        const listEl = byId('cov-consol-list');
        const checks = listEl.querySelectorAll('input[type="checkbox"]');
        let count = 0, total = 0n;
        const utxos = JSON.parse(listEl.dataset.utxos || '[]');
        checks.forEach(cb => {
            if (cb.checked) {
                count++;
                const idx = parseInt(cb.dataset.utxoIdx);
                total += BigInt(utxos[idx].amount);
            }
        });
        const kas = sompiToKasFixed(total, 4);
        byId('cov-consol-summary').textContent = count + ' UTXO' + (count !== 1 ? 's' : '') + ' selected: ' + kas + ' KAS';
    }
