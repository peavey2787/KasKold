// @ts-check
import { byId, elementById } from '../../../core/dom.js';
import { covShowPanel } from '../../../features/covenants/generation/panel_keys.js';
import { handleCovbScan } from '../../../features/covenants/recovery/import.js';
import { recoverCovenants } from '../../../features/covenants/recovery/scanner.js';
import { startScanner } from '../../../features/stealth/index/camera.js';

export function bindCovenantRecoveryEvents() {
    elementById('btn-cov-load-existing').onclick = () => {
        covShowPanel('load');
        const type = byId('cov-load-type');
        if (!type) return;
        type.style.display = '';
        const label = /** @type {HTMLElement | null} */ (type.previousElementSibling);
        if (label?.classList.contains('input-label')) label.style.display = '';
    };
    const recoverButton = byId('btn-cov-recover-chain');
    if (recoverButton) recoverButton.onclick = () => recoverCovenants();
    const scanButton = byId('btn-cov-import-scan');
    if (scanButton) {
        scanButton.onclick = () => startScanner('Scan Covenant Backup QR', handleCovbScan, 'menu');
    }
}
