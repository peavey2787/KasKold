// @ts-check
import { is_kaspa_address } from '../../../../wasm/api.js';
import { covenantState } from '../../../state/index.js';
import { showScreen } from '../../../navigation.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../features/covenants/generation/panel_keys.js';
import { handleCovCheckBalance } from '../../../../features/covenants/spending/advanced.js';
import { startScanner, stopScanner } from '../../../../features/stealth/index/camera.js';
import { byId, elementById, inputById } from '../../../../core/dom.js';

export function bindSwapAndUtilityActions() {
    elementById('cov-result-addr').onclick = () => { navigator.clipboard.writeText(elementById('cov-result-addr').textContent); toast('Address copied', 'ok', 1200); };
    elementById('cov-result-script').onclick = () => { navigator.clipboard.writeText(elementById('cov-result-script').textContent); toast('Redeem script copied', 'ok', 1200); };
    if (byId('btn-cov-scan-owner-addr')) elementById('btn-cov-scan-owner-addr').onclick = () => startScanner('Scan covenant address', (data) => {
        const addr = new TextDecoder().decode(new Uint8Array(data)).trim();
        if (is_kaspa_address(addr)) { stopScanner(); inputById('cov-owner-addr').value = addr; showScreen('covenant'); toast('Address scanned', 'ok', 1500); }
    });
    elementById('btn-cov-scan-owner-dest').onclick = () => startScanner('Scan destination', (data) => {
        const addr = new TextDecoder().decode(new Uint8Array(data)).trim();
        if (is_kaspa_address(addr)) { stopScanner(); inputById('cov-owner-dest').value = addr; showScreen('covenant'); covShowPanel('owner'); toast('Address scanned', 'ok', 1500); }
    });
    elementById('btn-consol-scan-dest').onclick = () => startScanner('Scan destination', (data) => {
        const addr = new TextDecoder().decode(new Uint8Array(data)).trim();
        if (is_kaspa_address(addr)) { stopScanner(); inputById('cov-consol-dest').value = addr; showScreen('covenant'); covShowPanel('consolidate'); toast('Address scanned', 'ok', 1500); }
    });
    // Balance checker
    elementById('btn-cov-check-balance').onclick = () => {
        covShowPanel('balance');
        if (covenantState.lastCovenantResult) {
            inputById('cov-balance-addr').value = covenantState.lastCovenantResult.address || '';
        }
        elementById('cov-balance-result').classList.add('hidden');
    };
    elementById('btn-cov-owner-reclaim').onclick = () => {
        covShowPanel('owner');
        if (covenantState.lastCovenantResult) {
            inputById('cov-owner-addr').value = covenantState.lastCovenantResult.address || '';
            inputById('cov-owner-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
        }
    };
    elementById('btn-cov-balance-back').onclick = () => covShowPanel('menu');
    elementById('btn-cov-balance-check').onclick = () => handleCovCheckBalance();
    elementById('btn-cov-scan-balance-addr').onclick = () => startScanner('Scan covenant address', (data) => {
        const addr = new TextDecoder().decode(new Uint8Array(data)).trim();
        if (is_kaspa_address(addr)) { stopScanner(); inputById('cov-balance-addr').value = addr; showScreen('covenant'); toast('Address scanned', 'ok', 1500); }
    });
}
