// @ts-check
import { is_kaspa_address } from '../../../wasm/api.js';
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

import { elementById, inputById } from '../../../core/dom.js';


export function bindCoreEvents() {
    elementById('btn-scan-kpub').onclick = () => showKpubManager('welcome', { openImport: true });
    const openMultisigSpend = () => { renderSavedMultisigSpendOptions(); showScreen('multisig-spend'); };
    elementById('btn-multisig-welcome').onclick = openMultisigSpend;
    elementById('btn-broadcast-welcome').onclick = () => { navigationState._broadcastReturnScreen = 'welcome'; hideBroadcastResult(); showScreen('broadcast'); };
    elementById('btn-send').onclick = () => openSendScreen();
    elementById('btn-receive').onclick = () => showReceive();
    elementById('btn-advanced').onclick = () => showScreen('advanced');
    elementById('btn-advanced-back').onclick = () => navigateBack('dashboard');
    elementById('btn-broadcast').onclick = () => { navigationState._broadcastReturnScreen = 'advanced'; hideBroadcastResult(); showScreen('broadcast'); };
    elementById('btn-multisig-spend').onclick = () => showScreen('multisig');
    elementById('btn-ms-menu-spend').onclick = openMultisigSpend;
    elementById('btn-ms-menu-descriptors').onclick = () => { renderSavedMultisigDescriptors(); showScreen('multisig-descriptors'); };
    elementById('btn-ms-menu-back').onclick = () => navigateBack(walletSession.hasWallet() ? 'dashboard' : 'welcome');
    elementById('btn-ms-descriptors-back').onclick = () => showScreen('multisig', { recordHistory: false });
    elementById('btn-ms-descriptor-save').onclick = () => saveMultisigDescriptor();
    elementById('select-ms-saved-descriptor').onchange = () => useSelectedMultisigDescriptor();
    elementById('btn-ms-descriptor-scan-saved').onclick = () => startScanner('Scan multisig descriptor QR', handleSavedDescriptorScan);
    const descriptorImageInput = elementById('input-ms-descriptor-image-saved');
    elementById('btn-ms-descriptor-upload-saved').onclick = () => descriptorImageInput.click();
    descriptorImageInput.onchange = async event => {
        const input = /** @type {HTMLInputElement} */ (event.currentTarget);
        const [file] = input.files || [];
        await uploadSavedDescriptorQrImage(file);
        input.value = '';
    };
    elementById('btn-ms-back').onclick = () => {
        if (walletSession.hasWallet()) showScreen('multisig', { recordHistory: false });
        else navigateBack('welcome');
    };
    elementById('btn-ms-create').onclick = () => handleMultisigCreate();
    elementById('btn-ms-max').onclick = () => handleMsMax();
    elementById('btn-toggle-ms-utxos').onclick = () => toggleMsUtxos();
    elementById('btn-ms-discover').onclick = () => discoverMultisigBranch();
    elementById('btn-scan-ms-source').onclick = () => startScanner('Scan P2SH address', data => {
        const text = new TextDecoder().decode(new Uint8Array(data));
        const addr = text.trim();
        if (is_kaspa_address(addr)) {
            stopScanner();
            inputById('input-ms-source').value = addr;
            showScreen('multisig-spend');
            toast('Address scanned', 'ok', 1500);
        }
    });
    elementById('btn-scan-ms-dest').onclick = () => startScanner('Scan destination', data => {
        const text = new TextDecoder().decode(new Uint8Array(data));
        const addr = text.trim();
        if (is_kaspa_address(addr) || addr.endsWith('.kas')) {
            stopScanner();
            inputById('input-ms-dest').value = addr;
            showScreen('multisig-spend');
            toast('Address scanned', 'ok', 1500);
        }
    });
    elementById('btn-scan-ms-descriptor').onclick = () => startScanner('Scan descriptor QR', handleDescriptorScan);
}
