import { navigationState, walletSession } from '../../state/index.js';
import { navigateBack, showScreen } from '../../navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { startScanner, stopScanner } from '../../../features/stealth/index/camera.js';
import { discoverMultisigBranch, handleDescriptorScan, handleMsMax, handleMultisigCreate, toggleMsUtxos } from '../../../features/transactions/pskt_multisig/multisig.js';
import { hideBroadcastResult } from '../../../features/transactions/send/broadcast.js';
import { openSendScreen } from '../../../features/transactions/send/compose/send_form.js';
import { showReceive } from '../../../features/transactions/send/receive.js';
import { showKpubManager } from '../../../features/wallet/kpub_manager/index.js';
import { handleSavedDescriptorScan, renderSavedMultisigDescriptors, renderSavedMultisigSpendOptions, saveMultisigDescriptor, uploadSavedDescriptorQrImage, useSelectedMultisigDescriptor } from '../../../features/transactions/pskt_multisig/descriptors.js';
// KasKold Companion Web — app/events/system/core
// Binds application-wide scanner and primary navigation events.

import { byId } from '../../../core/dom.js';


export function bindCoreEvents() {
    byId('btn-scan-kpub').onclick = () => showKpubManager('welcome', { openImport: true });
    const openMultisigSpend = () => { renderSavedMultisigSpendOptions(); showScreen('multisig-spend'); };
    byId('btn-multisig-welcome').onclick = openMultisigSpend;
    byId('btn-broadcast-welcome').onclick = () => { navigationState._broadcastReturnScreen = 'welcome'; hideBroadcastResult(); showScreen('broadcast'); };
    byId('btn-send').onclick = () => openSendScreen();
    byId('btn-receive').onclick = () => showReceive();
    byId('btn-advanced').onclick = () => showScreen('advanced');
    byId('btn-advanced-back').onclick = () => navigateBack('dashboard');
    byId('btn-broadcast').onclick = () => { navigationState._broadcastReturnScreen = 'advanced'; hideBroadcastResult(); showScreen('broadcast'); };
    byId('btn-multisig-spend').onclick = () => showScreen('multisig');
    byId('btn-ms-menu-spend').onclick = openMultisigSpend;
    byId('btn-ms-menu-descriptors').onclick = () => { renderSavedMultisigDescriptors(); showScreen('multisig-descriptors'); };
    byId('btn-ms-menu-back').onclick = () => navigateBack(walletSession.hasWallet() ? 'dashboard' : 'welcome');
    byId('btn-ms-descriptors-back').onclick = () => showScreen('multisig', { recordHistory: false });
    byId('btn-ms-descriptor-save').onclick = () => saveMultisigDescriptor();
    byId('select-ms-saved-descriptor').onchange = () => useSelectedMultisigDescriptor();
    byId('btn-ms-descriptor-scan-saved').onclick = () => startScanner('Scan multisig descriptor QR', handleSavedDescriptorScan);
    const descriptorImageInput = byId('input-ms-descriptor-image-saved');
    byId('btn-ms-descriptor-upload-saved').onclick = () => descriptorImageInput.click();
    descriptorImageInput.onchange = async event => {
        const input = event.currentTarget;
        const [file] = input.files || [];
        await uploadSavedDescriptorQrImage(file);
        input.value = '';
    };
    byId('btn-ms-back').onclick = () => {
        if (walletSession.hasWallet()) showScreen('multisig', { recordHistory: false });
        else navigateBack('welcome');
    };
    byId('btn-ms-create').onclick = () => handleMultisigCreate();
    byId('btn-ms-max').onclick = () => handleMsMax();
    byId('btn-toggle-ms-utxos').onclick = () => toggleMsUtxos();
    byId('btn-ms-discover').onclick = () => discoverMultisigBranch();
    byId('btn-scan-ms-source').onclick = () => startScanner('Scan P2SH address', data => {
        const text = new TextDecoder().decode(new Uint8Array(data));
        const addr = text.trim();
        if (addr.startsWith('kaspa:')) {
            stopScanner();
            byId('input-ms-source').value = addr;
            showScreen('multisig-spend');
            toast('Address scanned', 'ok', 1500);
        }
    });
    byId('btn-scan-ms-dest').onclick = () => startScanner('Scan destination', data => {
        const text = new TextDecoder().decode(new Uint8Array(data));
        const addr = text.trim();
        if (addr.startsWith('kaspa:') || addr.endsWith('.kas')) {
            stopScanner();
            byId('input-ms-dest').value = addr;
            showScreen('multisig-spend');
            toast('Address scanned', 'ok', 1500);
        }
    });
    byId('btn-scan-ms-descriptor').onclick = () => startScanner('Scan descriptor QR', handleDescriptorScan);
}
