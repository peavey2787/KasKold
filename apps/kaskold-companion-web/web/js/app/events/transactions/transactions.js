// @ts-check
import { transactionState, walletSession } from '../../state/index.js';
import { closeRelayModal, handlePsktFinalize, handlePsktRelay, handlePsktRelayKasKoldStandard, handlePsktRelayCompact, openRelayModal } from '../../../features/transactions/pskt_multisig/review.js';
import { navigateBack } from '../../navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { resetWallet } from '../../../features/wallet/reset.js';
import { startScanner, stopScanner } from '../../../features/stealth/index/camera.js';
import { handleBroadcastHex, handleSignedFrameHex, handleSignedScan, hideBroadcastResult, resetSignedFrameHexImportSession } from '../../../features/transactions/send/broadcast.js';
import { importSignedQrImage, resetSignedQrImageImportSession } from '../../../features/transactions/send/signed_qr_image_import.js';
import { clearAntiKleptoSession } from '../../../features/transactions/anti_klepto/session.js';
import { handleSendMax, limitKasAmountPrecision, setFeeLevel, syncFeeInputFromDisplay, toggleFeeDisplayUnit, toggleSendUtxos } from '../../../features/transactions/send/compose/send_form.js';
import { handleCreateTx, handleDestScan } from '../../../features/transactions/send/compose/transaction_building.js';
import { applyReceiveAdvanced, copyAddress, toggleReceiveAdvanced } from '../../../features/transactions/send/receive.js';
import { copyAllQrFrameHex, pauseQrCycle, stopQrCycle, toggleQrFrameHex } from '../../../features/transactions/send/review.js';
import { refreshBalance, refreshBalanceAfterBroadcast, releasePendingStandardChange, toggleBalanceDisplayUnit } from '../../../features/wallet/core.js';
// KasKold Companion Web — app/events/transactions/transactions
// Binds transaction creation, review, relay, broadcast, and receive events.

import { byId, elementById, inputById } from '../../../core/dom.js';
import { returnFromTransaction, showTransactionReturnScreen, takeTransactionReturnScreen, walletAwareDefaultScreen } from './return_routing.js';


export function bindTransactionsEvents() {

    elementById('btn-refresh').onclick = () => refreshBalance();
    elementById('btn-balance-unit-toggle').onclick = () => toggleBalanceDisplayUnit();
    elementById('btn-reset-wallet').onclick = () => resetWallet();
    elementById('btn-create-tx').onclick = () => handleCreateTx();
    elementById('input-amount').oninput = event => limitKasAmountPrecision(event.currentTarget);
    elementById('btn-send-max').onclick = () => handleSendMax();
    elementById('input-fee-display').oninput = () => syncFeeInputFromDisplay();
    elementById('btn-fee-unit-toggle').onclick = () => toggleFeeDisplayUnit();
    elementById('btn-scan-dest').onclick = () => startScanner('Scan address QR', handleDestScan);
    elementById('btn-toggle-utxos').onclick = () => toggleSendUtxos();
    elementById('btn-fee-low').onclick = () => setFeeLevel('low');
    elementById('btn-fee-normal').onclick = () => setFeeLevel('normal');
    elementById('btn-fee-priority').onclick = () => setFeeLevel('priority');
    elementById('btn-send-back').onclick = () => returnFromTransaction();
    elementById('btn-qr-back').onclick = () => {
        stopQrCycle();
        clearAntiKleptoSession();
        // Restore QR screen buttons (may have been hidden by piggy share)
        if (byId('btn-qr-scan-signed')) elementById('btn-qr-scan-signed').style.display = '';
        if (byId('btn-scan-next-sig')) elementById('btn-scan-next-sig').style.display = '';
        returnFromTransaction();
    };
    elementById('btn-scan-next-sig').onclick = () => { pauseQrCycle(); resetSignedQrImageImportSession(); startScanner('Scan next signature', handleSignedScan, undefined, { allowFrameHex: true }); };
    elementById('btn-toggle-qr-frame-hex').onclick = () => toggleQrFrameHex();
    elementById('btn-copy-all-qr-frame-hex').onclick = () => { void copyAllQrFrameHex(); };
    elementById('btn-qr-scan-signed').onclick = event => {
        pauseQrCycle();
        resetSignedQrImageImportSession();
        const title = /** @type {HTMLElement} */ (event.currentTarget).dataset.scanTitle || 'Scan signed QR';
        startScanner(title, handleSignedScan, undefined, { allowFrameHex: true });
    };
    elementById('btn-copy-kspt').onclick = () => { if (transactionState._currentKsptHex) { navigator.clipboard.writeText(transactionState._currentKsptHex); toast('KSPT hex copied — share with next signer', 'ok', 2000); } };
    elementById('btn-scanner-cancel').onclick = () => stopScanner();
    elementById('btn-copy-address').onclick = () => copyAddress();
    elementById('btn-receive-advanced').onclick = () => toggleReceiveAdvanced();
    elementById('btn-receive-apply').onclick = () => applyReceiveAdvanced();
    elementById('btn-receive-back').onclick = () => navigateBack('dashboard');
    elementById('btn-scan-signed').onclick = () => { resetSignedQrImageImportSession(); resetSignedFrameHexImportSession(); startScanner('Scan signed KasKold QR', handleSignedScan, undefined, { allowFrameHex: true }); };
    elementById('btn-load-signed-qr-image').onclick = () => { resetSignedFrameHexImportSession(); elementById('input-signed-qr-image').click(); };
    elementById('input-signed-qr-image').onchange = async event => {
        const input = /** @type {HTMLInputElement} */ (event.currentTarget);
        const files = Array.from(input.files || []);
        input.value = '';
        for (const file of files) {
            const complete = await importSignedQrImage(file);
            if (complete !== false) break;
        }
    };
    const signedFrameHexInput = inputById('input-signed-frame-hex');
    signedFrameHexInput.onpaste = event => {
        const pasted = event.clipboardData?.getData('text')?.trim() || '';
        if (!pasted) return;
        event.preventDefault();
        signedFrameHexInput.value = pasted;
        handleSignedFrameHex();
    };
    signedFrameHexInput.onkeydown = event => {
        if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
        event.preventDefault();
        handleSignedFrameHex();
    };
    elementById('btn-broadcast-hex').onclick = () => handleBroadcastHex();
    elementById('btn-broadcast-back').onclick = () => { clearAntiKleptoSession(); resetSignedQrImageImportSession(); resetSignedFrameHexImportSession(); returnFromTransaction({ defaultScreen: walletAwareDefaultScreen() }); };
    elementById('btn-pskt-back').onclick = () => {
        clearAntiKleptoSession();
        const reservedIndex = transactionState._standardChangeReservationIndex;
        if (Number.isSafeInteger(reservedIndex)) releasePendingStandardChange(reservedIndex);
        transactionState._standardChangeReservationIndex = null;
        transactionState._psktReviewHex = null;
        returnFromTransaction();
    };
    elementById('btn-pskt-relay').onclick = () => openRelayModal();
    elementById('btn-relay-standard').onclick = () => { closeRelayModal(); handlePsktRelay(); };
    elementById('btn-relay-kaskold-standard').onclick = () => { closeRelayModal(); handlePsktRelayKasKoldStandard(); };
    elementById('btn-relay-compact').onclick = () => { closeRelayModal(); handlePsktRelayCompact(); };
    elementById('btn-relay-cancel').onclick = () => closeRelayModal();
    elementById('btn-pskt-finalize').onclick = () => handlePsktFinalize();
    elementById('btn-broadcast-done').onclick = () => {
        hideBroadcastResult();
        const returnScreen = takeTransactionReturnScreen() || walletAwareDefaultScreen();
        showTransactionReturnScreen(returnScreen);
        if (walletSession.hasWallet()) void refreshBalanceAfterBroadcast();
    };
    elementById('btn-copy-txid').onclick = () => {
        const txid = elementById('broadcast-result-txid').textContent.trim();
        navigator.clipboard.writeText(txid);
        toast('TX ID copied', 'ok', 1500);
    };
}
