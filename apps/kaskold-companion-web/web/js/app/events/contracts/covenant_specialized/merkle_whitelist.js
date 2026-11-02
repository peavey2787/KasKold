// @ts-check
import { covenantState } from '../../../state/index.js';
import { hideLoading, showLoading } from '../../../navigation.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../features/covenants/generation/panel_keys.js';
import { covScanAddress, covScanAddressAppend } from '../../../../features/covenants/scanning.js';
import { handleCovMwSpend, mwMaxSompi } from '../../../../features/covenants/spending/advanced.js';
import { byId, elementById, inputById } from '../../../../core/dom.js';
import { sompiToKasString } from '../../../../core/amounts.js';

export function bindMerkleWhitelistEvents() {
    // Merkle whitelist panel wiring
    if (byId('btn-cov-mw-never')) elementById('btn-cov-mw-never').onclick = () => {
        const d = new Date();
        d.setFullYear(d.getFullYear() + 100);
        const pad = (n) => String(n).padStart(2, '0');
        const v = d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + 'T' + pad(d.getHours()) + ':' + pad(d.getMinutes());
        if (byId('cov-mw-datetime')) inputById('cov-mw-datetime').value = v;
        if (byId('cov-mw-locktime')) inputById('cov-mw-locktime').value = ''; // force recompute from the new far-future date
        toast('Refund blocked ~100 years out. Whitelist is now permanent.', 'ok', 2500);
    };
    if (byId('btn-cov-mw-spend')) elementById('btn-cov-mw-spend').onclick = async () => {
        // Pre-fill spend panel from active covenant data
        if (covenantState.lastCovenantResult) {
            inputById('cov-mw-addr').value = covenantState.lastCovenantResult.address || '';
            inputById('cov-mw-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
            // Restore whitelist addresses from active entry
            const activeEntry = covenantState.activeCovenants.find(c => c.address === covenantState.lastCovenantResult.address);
            const addrJson = (activeEntry && activeEntry.merkle_addresses_json) || covenantState.lastCovenantResult.merkle_addresses_json || '';
            if (addrJson) {
                try { inputById('cov-mw-spend-addresses').value = JSON.parse(addrJson).join('\n'); } catch { /* An unreadable saved list leaves the field for the user to fill. */ }
            }
        }
        covShowPanel('mw-spend');
        // Show the spendable max as the grayed placeholder (matches what Max fills).
        inputById('cov-mw-amount').placeholder = 'Computing max...';
        try {
            const m = await mwMaxSompi();
            inputById('cov-mw-amount').placeholder = m ? (sompiToKasString(m) + ' max') : 'e.g. 5.0';
        } catch { inputById('cov-mw-amount').placeholder = 'e.g. 5.0'; }
    };
    elementById('btn-cov-mw-spend-back').onclick = () => covShowPanel('result');
    elementById('btn-cov-mw-spend-create').onclick = () => handleCovMwSpend();
    if (byId('btn-cov-mw-max')) elementById('btn-cov-mw-max').onclick = async () => {
        showLoading('Computing max...');
        try {
            const m = await mwMaxSompi();
            hideLoading();
            if (!m) { toast('No spendable balance', 'error'); return; }
            inputById('cov-mw-amount').value = sompiToKasString(m);
        } catch (e) { hideLoading(); toast('Max failed: ' + e, 'error'); }
    };
    if (byId('btn-cov-scan-mw-addr')) elementById('btn-cov-scan-mw-addr').onclick = () => covScanAddress('cov-mw-addr', 'Scan covenant address');
    elementById('btn-cov-scan-mw-dest').onclick = () => covScanAddress('cov-mw-dest', 'Scan destination');
    if (byId('btn-cov-scan-mw-add')) elementById('btn-cov-scan-mw-add').onclick = () => covScanAddressAppend('cov-mw-addresses', 'Scan whitelist address');
}
