// @ts-check
import { handleCovGenerate } from '../../../../features/covenants/generation/create.js';
import { handleCovFund } from '../../../../features/covenants/generation/fund.js';
import { covScanAddress } from '../../../../features/covenants/scanning.js';
// KasKold Companion Web — app/events/contracts/covenant_creation/creation_options
import { byId, elementById, inputById } from '../../../../core/dom.js';
import { bindDurationInputs } from '../../../../core/forms/duration.js';


export function registerCreationOptions() {
    // Spending-limit cooldown preset + custom timer
    if (byId('cov-splimit-preset')) {
        bindDurationInputs({ prefix: 'cov-splimit', outputId: 'cov-splimit-cooldown' });
        elementById('cov-splimit-preset').onchange = () => {
            const v = inputById('cov-splimit-preset').value;
            const customWrap = byId('cov-splimit-custom-wrap');
            if (customWrap) customWrap.classList.toggle('hidden', v !== 'custom');
            if (v !== 'custom') inputById('cov-splimit-cooldown').value = v;
        };
    }
    if (byId('btn-cov-scan-bene-addr')) elementById('btn-cov-scan-bene-addr').onclick = () => covScanAddress('cov-bene-addr', 'Scan covenant address');
    if (byId('btn-cov-scan-bene-dest')) elementById('btn-cov-scan-bene-dest').onclick = () => covScanAddress('cov-bene-dest', 'Scan destination');
    elementById('btn-cov-generate').onclick = () => handleCovGenerate();
    elementById('btn-cov-fund').onclick = () => handleCovFund();

}
