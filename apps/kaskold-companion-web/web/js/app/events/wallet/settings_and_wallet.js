import { navigationState, walletSession } from '../../state/index.js';
import { navigateBack, showScreen } from '../../navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { visibleScreenName } from '../../../core/ui/screen_dom.js';
import { showTokens } from '../../../features/assets/index.js';
import { clearCustomNode, exitSettings, saveSettings, showSettings } from '../../../features/settings/screen.js';
import {
    closeKpubImport,
    openKpubImport,
    saveManagedKpub,
    showCurrentKpub,
    scanManagedKpub,
    uploadManagedKpubImage,
    useKpubOnce,
} from '../../../features/wallet/kpub_manager/index.js';
import { clearHistory, handleConsolidate, handleConsolidateSelected, handleSendSelectedUtxos, showAddresses, showHistory, showUtxos } from '../../../features/wallet/tools.js';
import { showPortfolio } from '../../../features/portfolio/index.js';
// KasKold Companion Web — app/events/settings and wallet
// Binds settings, wallet-history, address, UTXO, and consolidation events.

import { bindChange, bindClick, bindKeydown, byId } from '../../../core/dom.js';


export function bindSettingsAndWalletEvents() {
    bindClick('btn-header-settings', () => showSettings(visibleScreenName(walletSession.hasWallet() ? 'dashboard' : 'welcome')));
    bindClick('btn-save-settings', () => saveSettings());
    bindClick('btn-use-public', () => { clearCustomNode(); exitSettings(); });
    bindClick('btn-settings-back', () => navigateBack());
    bindClick('btn-settings-back-top', () => navigateBack());
    bindClick('btn-kpub-manager-back', () => navigateBack());
    bindClick('btn-kpub-manager-back-top', () => navigateBack());
    bindClick('btn-open-kpub-import', () => openKpubImport());
    bindClick('btn-close-kpub-import', () => closeKpubImport());
    bindClick('btn-scan-managed-kpub', () => scanManagedKpub());
    const managedImageInput = byId('input-managed-kpub-image');
    bindClick('btn-upload-managed-kpub', () => managedImageInput?.click());
    bindChange('input-managed-kpub-image', async event => {
        const input = event.currentTarget;
        const [file] = input.files || [];
        await uploadManagedKpubImage(file);
        input.value = '';
    });
    bindClick('btn-save-managed-kpub', () => saveManagedKpub());
    bindClick('btn-use-current-kpub', () => useKpubOnce());
    bindKeydown('input-kpub-friendly-name', event => {
        if (event.key === 'Enter') saveManagedKpub();
    });

    bindClick('btn-dashboard-tokens', () => showTokens());
    bindClick('btn-dashboard-history', () => showHistory());
    bindClick('btn-dashboard-portfolio', () => showPortfolio());
    bindClick('btn-view-kpub', () => showCurrentKpub());
    bindClick('btn-copy-view-kpub', async () => {
        const kpub = byId('view-kpub-text')?.textContent?.trim() || '';
        if (!kpub) return;
        try {
            await navigator.clipboard.writeText(kpub);
            toast('kpub copied', 'ok', 1200);
        } catch (_) {
            toast('Could not copy kpub', 'error', 2200);
        }
    });
    bindClick('btn-view-kpub-back', () => navigateBack('advanced'));
    bindClick('btn-advanced-addresses', () => showAddresses());
    bindClick('btn-advanced-utxos', () => showUtxos());
    bindClick('btn-addresses-back', () => navigateBack(navigationState.addressesReturnScreen));
    bindClick('btn-addresses-back-top', () => navigateBack(navigationState.addressesReturnScreen));
    bindClick('btn-tokens-back', () => navigateBack());
    bindClick('btn-tokens-back-top', () => navigateBack());
    bindClick('btn-verify-copy', async () => {
        const address = byId('verify-address').textContent.trim();
        if (!address) return;
        try {
            await navigator.clipboard.writeText(address);
            toast('Address copied', 'ok', 1200);
        } catch (_) {
            toast('Could not copy address', 'error', 2200);
        }
    });
    bindClick('btn-verify-back', () => {
        showScreen('addresses');
        document.querySelector('main').scrollTop = 0;
    });
    bindClick('btn-utxos-back', () => navigateBack());
    bindClick('btn-utxos-back-top', () => navigateBack());
    bindClick('btn-consolidate', () => handleConsolidate());
    bindClick('btn-consolidate-selected', () => handleConsolidateSelected());
    bindClick('btn-send-selected-utxos', () => handleSendSelectedUtxos());
    bindClick('btn-history-back', () => navigateBack());
    bindClick('btn-history-back-top', () => navigateBack());
    bindClick('btn-portfolio-back', () => navigateBack());
    bindClick('btn-portfolio-back-top', () => navigateBack());
    bindClick('btn-clear-history', () => clearHistory());
}
