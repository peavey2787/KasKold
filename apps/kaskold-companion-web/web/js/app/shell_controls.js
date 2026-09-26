import { bindBrowserConnectivity } from '../core/ui/connectivity_status.js';
import { setScreenReturn, takeScreenReturn, visibleScreenName } from '../core/ui/screen_dom.js';
import { navigateHome, showScreen } from './navigation.js';

function byId(id) {
    return document.getElementById(id);
}

function showNodeSettings() {
    const source = visibleScreenName();
    if (source === 'settings') {
        showScreen(takeScreenReturn('settings', 'welcome'), { recordHistory: false });
        return;
    }
    setScreenReturn('settings', source);
    showScreen('settings');
}

function showManagedWallets(openImport = false) {
    setScreenReturn('kpub-manager', visibleScreenName());
    byId('kpub-import-form')?.classList.toggle('hidden', !openImport);
    showScreen('kpub-manager');
}

export function bindShellControls() {
    bindBrowserConnectivity();
    const settings = byId('btn-header-settings');
    if (settings) settings.onclick = showNodeSettings;

    const closeSettings = () => showScreen(takeScreenReturn('settings', 'welcome'), { recordHistory: false });
    const closeWalletManager = () => showScreen(takeScreenReturn('kpub-manager', 'welcome'), { recordHistory: false });
    const settingsBack = byId('btn-settings-back');
    const settingsBackTop = byId('btn-settings-back-top');
    if (settingsBack) settingsBack.onclick = closeSettings;
    if (settingsBackTop) settingsBackTop.onclick = closeSettings;
    const managerBack = byId('btn-kpub-manager-back');
    const managerBackTop = byId('btn-kpub-manager-back-top');
    if (managerBack) managerBack.onclick = closeWalletManager;
    if (managerBackTop) managerBackTop.onclick = closeWalletManager;

    const loadWallet = byId('btn-scan-kpub');
    if (loadWallet) loadWallet.onclick = () => showManagedWallets(true);

    const logo = byId('btn-logo');
    if (logo) {
        logo.onclick = () => navigateHome();
        logo.onkeydown = event => {
            if (event.key !== 'Enter' && event.key !== ' ') return;
            event.preventDefault();
            navigateHome();
        };
    }
}
