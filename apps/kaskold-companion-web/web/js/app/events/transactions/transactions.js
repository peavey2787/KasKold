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

import { byId } from '../../../core/dom.js';
import { returnFromTransaction, showTransactionReturnScreen, takeTransactionReturnScreen, walletAwareDefaultScreen } from './return_routing.js';


export function bindTransactionsEvents() {

    byId('btn-refresh').onclick = () => refreshBalance();
    byId('btn-balance-unit-toggle').onclick = () => toggleBalanceDisplayUnit();
    byId('btn-reset-wallet').onclick = () => resetWallet();
    byId('btn-create-tx').onclick = () => handleCreateTx();
    byId('input-amount').oninput = event => limitKasAmountPrecision(event.currentTarget);
    byId('btn-send-max').onclick = () => handleSendMax();
    byId('input-fee-display').oninput = () => syncFeeInputFromDisplay();
    byId('btn-fee-unit-toggle').onclick = () => toggleFeeDisplayUnit();
    byId('btn-scan-dest').onclick = () => startScanner('Scan address QR', handleDestScan);
    byId('btn-toggle-utxos').onclick = () => toggleSendUtxos();
    byId('btn-fee-low').onclick = () => setFeeLevel('low');
    byId('btn-fee-normal').onclick = () => setFeeLevel('normal');
    byId('btn-fee-priority').onclick = () => setFeeLevel('priority');
    byId('btn-send-back').onclick = () => returnFromTransaction();
    byId('btn-qr-back').onclick = () => {
        stopQrCycle();
        clearAntiKleptoSession();
        // Restore QR screen buttons (may have been hidden by piggy share)
        if (byId('btn-qr-scan-signed')) byId('btn-qr-scan-signed').style.display = '';
        if (byId('btn-scan-next-sig')) byId('btn-scan-next-sig').style.display = '';
        returnFromTransaction();
    };
    byId('btn-scan-next-sig').onclick = () => { pauseQrCycle(); resetSignedQrImageImportSession(); startScanner('Scan next signature', handleSignedScan, undefined, { allowFrameHex: true }); };
    byId('btn-toggle-qr-frame-hex').onclick = () => toggleQrFrameHex();
    byId('btn-copy-all-qr-frame-hex').onclick = () => { void copyAllQrFrameHex(); };
    byId('btn-qr-scan-signed').onclick = event => {
        pauseQrCycle();
        resetSignedQrImageImportSession();
        const title = event.currentTarget.dataset.scanTitle || 'Scan signed QR';
        startScanner(title, handleSignedScan, undefined, { allowFrameHex: true });
    };
    byId('btn-copy-kspt').onclick = () => { if (transactionState._currentKsptHex) { navigator.clipboard.writeText(transactionState._currentKsptHex); toast('KSPT hex copied — share with next signer', 'ok', 2000); } };
    byId('btn-scanner-cancel').onclick = () => stopScanner();
    byId('btn-copy-address').onclick = () => copyAddress();
    byId('btn-receive-advanced').onclick = () => toggleReceiveAdvanced();
    byId('btn-receive-apply').onclick = () => applyReceiveAdvanced();
    byId('btn-receive-back').onclick = () => navigateBack('dashboard');
    byId('btn-scan-signed').onclick = () => { resetSignedQrImageImportSession(); resetSignedFrameHexImportSession(); startScanner('Scan signed KasKold QR', handleSignedScan, undefined, { allowFrameHex: true }); };
    byId('btn-load-signed-qr-image').onclick = () => { resetSignedFrameHexImportSession(); byId('input-signed-qr-image').click(); };
    byId('input-signed-qr-image').onchange = async event => {
        const input = event.currentTarget;
        const files = Array.from(input.files || []);
        input.value = '';
        for (const file of files) {
            const complete = await importSignedQrImage(file);
            if (complete !== false) break;
        }
    };
    const signedFrameHexInput = byId('input-signed-frame-hex');
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
    byId('btn-broadcast-hex').onclick = () => handleBroadcastHex();
    byId('btn-broadcast-back').onclick = () => { clearAntiKleptoSession(); resetSignedQrImageImportSession(); resetSignedFrameHexImportSession(); returnFromTransaction({ defaultScreen: walletAwareDefaultScreen() }); };
    byId('btn-pskt-back').onclick = () => {
        clearAntiKleptoSession();
        const reservedIndex = transactionState._standardChangeReservationIndex;
        if (Number.isSafeInteger(reservedIndex)) releasePendingStandardChange(reservedIndex);
        transactionState._standardChangeReservationIndex = null;
        transactionState._psktReviewHex = null;
        returnFromTransaction();
    };
    byId('btn-pskt-relay').onclick = () => openRelayModal();
    byId('btn-relay-standard').onclick = () => { closeRelayModal(); handlePsktRelay(); };
    byId('btn-relay-kaskold-standard').onclick = () => { closeRelayModal(); handlePsktRelayKasKoldStandard(); };
    byId('btn-relay-compact').onclick = () => { closeRelayModal(); handlePsktRelayCompact(); };
    byId('btn-relay-cancel').onclick = () => closeRelayModal();
    byId('btn-pskt-finalize').onclick = () => handlePsktFinalize();
    byId('btn-broadcast-done').onclick = () => {
        hideBroadcastResult();
        const returnScreen = takeTransactionReturnScreen() || walletAwareDefaultScreen();
        showTransactionReturnScreen(returnScreen);
        if (walletSession.hasWallet()) void refreshBalanceAfterBroadcast();
    };
    byId('btn-copy-txid').onclick = () => {
        const txid = byId('broadcast-result-txid').textContent.trim();
        navigator.clipboard.writeText(txid);
        toast('TX ID copied', 'ok', 1500);
    };
}
