import { covShowPanel, covTypeChanged } from '../../../../features/covenants/generation/panel_keys.js';
import { covScanAddress, covScanPubkey } from '../../../../features/covenants/scanning.js';
import { handleCovBorrowerSpend, handleCovPayjoinClaim } from '../../../../features/covenants/spending/standard/spend_paths.js';
import { byId } from '../../../../core/dom.js';
export function bindCovenantScansAndClaims(_context) {
    byId('btn-cov-borrower-create').onclick = () => handleCovBorrowerSpend();
    byId('cov-type').onchange = () => covTypeChanged();
    const recovery = byId('btn-cov-scan-savings-recovery');
    if (recovery) recovery.onclick = () => covScanPubkey('cov-savings-recovery-pk', 'Scan backup wallet address or x-only (not a kpub)', true);
    byId('btn-cov-scan-escrow-pk').onclick = () => covScanPubkey('cov-escrow-pk', 'Scan seller pubkey');
    byId('btn-cov-scan-escrow-arbiter').onclick = () => covScanPubkey('cov-escrow-arbiter-pk', 'Scan arbiter pubkey');
    byId('btn-cov-scan-allowance-bene').onclick = () => covScanPubkey('cov-allowance-bene-pk', 'Scan beneficiary address or x-only (not a kpub)', true);
    byId('btn-cov-scan-payjoin-bene').onclick = () => covScanPubkey('cov-payjoin-bene-pk', 'Scan beneficiary address');

    const payjoin = byId('btn-cov-payjoin-claim');
    if (payjoin) payjoin.onclick = () => covShowPanel('payjoin-claim');
    byId('btn-cov-payjoin-claim-back').onclick = () => covShowPanel('menu');
    byId('btn-cov-payjoin-claim-create').onclick = () => handleCovPayjoinClaim();
    const claimAddress = byId('btn-cov-scan-payjoin-claim-addr');
    if (claimAddress) claimAddress.onclick = () => covScanAddress('cov-payjoin-claim-addr', 'Scan covenant address');
    const mixAddress = byId('btn-cov-scan-payjoin-mix-addr');
    if (mixAddress) mixAddress.onclick = () => covScanAddress('cov-payjoin-mix-addr', 'Scan mixing address');
    byId('btn-cov-scan-payjoin-claim-dest').onclick = () => covScanAddress('cov-payjoin-claim-dest', 'Scan destination');

}
