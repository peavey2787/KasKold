// @ts-check
import { renderScanProgress } from '../../../core/ui/scan_progress.js';
import { bytesToHex, hexFrameBatchToBytes } from '../../../core/bytes.js';
import {
    covenantState,
    networkState,
    oracleState,
    transactionState,
    broadcast_signed,
    decode_qr_frame,
    decoder_progress,
    kaskold_sdk_complete,
    kspt_signature_status,
    pskt_detect,
    reset_qr_decoder,
} from '../boundary.js';
import { BROADCAST_ENABLED } from '../../../core/config/runtime.js';
import { oracleMbCardRefresh } from '../../oracle/model_b/controller.js';
import { hideLoading, showLoading, showScreen } from '../../../app/navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { stopScanner } from '../../stealth/index/camera.js';
import { openPsktReview } from '../pskt_multisig/review.js';
import { displayKsptQr } from './review.js';
import { markStandardChangeBroadcast, refreshBalanceAfterBroadcast, withNodeRetry } from '../../wallet/core.js';
// KasKold Companion Web — features/transactions/send/broadcast
import { byId, elementById, inputById } from '../../../core/dom.js';
import { ANTI_KLEPTO_KEEP_SCANNING, processAntiKleptoResponse } from '../anti_klepto/response.js';


// ─── Broadcast ───

let signedFrameHexSessionActive = false;

export function resetSignedFrameHexImportSession() {
    signedFrameHexSessionActive = false;
    try { reset_qr_decoder(); } catch { /* A decoder that never started has nothing to reset. */ }
    const input = inputById('input-signed-frame-hex');
    const status = byId('broadcast-frame-status');
    if (input) input.value = '';
    if (status) status.textContent = '';
}

function parseSignedFrameInput(value) {
    if (!value) {
        toast('Paste one QR Frame Hex value first', 'error', 2500);
        return null;
    }
    try {
        const frames = hexFrameBatchToBytes(value);
        if (!frames.length) throw new Error('QR Frame Hex is empty');
        return frames;
    } catch (error) {
        toast(`Invalid QR Frame Hex: ${error.message || error}`, 'error', 3500);
        return null;
    }
}

function ensureSignedFrameSession() {
    if (signedFrameHexSessionActive) return;
    try { reset_qr_decoder(); } catch { /* A decoder that never started has nothing to reset. */ }
    signedFrameHexSessionActive = true;
}

function consumeSignedFrames(frames) {
    /** @type {boolean | null} */
    let result = false;
    let accepted = 0;
    for (const bytes of frames) {
        result = handleSignedScan(bytes, {
            stopCamera: false,
            progressTargetId: 'broadcast-frame-status',
            showDecodeErrors: true,
        });
        accepted += 1;
        if (result !== false) break;
    }
    return { result, accepted };
}

function reportSignedFrameProgress(status, accepted) {
    if (!status || status.textContent.trim()) return;
    status.textContent = accepted > 1
        ? `${accepted} frames accepted. Paste any remaining QR Frame Hex values.`
        : 'Frame accepted. Paste the next QR Frame Hex.';
}

export function handleSignedFrameHex() {
    const input = inputById('input-signed-frame-hex');
    const status = byId('broadcast-frame-status');
    const frames = parseSignedFrameInput(input?.value.trim() || '');
    if (!frames) return false;
    ensureSignedFrameSession();
    const { result, accepted } = consumeSignedFrames(frames);
    input.value = '';
    if (result === false) {
        reportSignedFrameProgress(status, accepted);
        input.focus();
    } else {
        signedFrameHexSessionActive = false;
    }
    return result;
}

export function hideBroadcastResult() {
    const card = elementById('broadcast-result');
    card.classList.add('hidden');
    card.className = 'result-card hidden';
    inputById('input-signed-hex').value = '';
    byId('broadcast-instruction')?.classList.remove('hidden');
    const explorer = /** @type {HTMLAnchorElement} */ (elementById('link-view-explorer'));
    if (explorer) {
        explorer.classList.add('hidden');
        explorer.href = '#';
    }
    // Re-show the form card
    const formCard = /** @type {HTMLElement | null} */ (document.querySelector('#screen-broadcast .card'));
    if (formCard) formCard.style.display = '';
}
export function showBroadcastSuccess(txId) {
    const reservedIndex = transactionState._standardChangeReservationIndex;
    if (Number.isSafeInteger(reservedIndex)) {
        markStandardChangeBroadcast(reservedIndex);
        transactionState._standardChangeReservationIndex = null;
    }
    oracleState._oracleMbRollActive = false;
    oracleState._oracleMbPreSignAwaiting = false;
    oracleState._oracleMbAutoBroadcast = false;
    const card = elementById('broadcast-result');
    card.className = 'result-card success';
    card.classList.remove('hidden');
    elementById('broadcast-result-icon').textContent = '';
    elementById('broadcast-result-msg').textContent = 'Transaction broadcast!';
    elementById('broadcast-result-txid').textContent = txId;
    elementById('btn-copy-txid').style.display = 'block';
    elementById('btn-broadcast-done').style.display = 'block';
    byId('broadcast-instruction')?.classList.add('hidden');
    const explorer = /** @type {HTMLAnchorElement} */ (elementById('link-view-explorer'));
    if (explorer) {
        explorer.href = `https://kaspa.stream/transactions/${encodeURIComponent(txId)}`;
        explorer.classList.remove('hidden');
    }
    // Start convergence polling immediately after node acceptance. The node may
    // acknowledge the transaction before its UTXO index exposes the new change
    // output, so keep refreshing in the background while the success card is visible.
    void refreshBalanceAfterBroadcast();
    // Hide the form card
    const formCard = /** @type {HTMLElement | null} */ (document.querySelector('#screen-broadcast .card'));
    if (formCard) formCard.style.display = 'none';

    // Path C post-broadcast hook: capture tx_id and trigger TX2
    if (covenantState._kasFreezePathCPostBroadcast) {
        const cb = covenantState._kasFreezePathCPostBroadcast;
        covenantState._kasFreezePathCPostBroadcast = null;
        cb(txId);
    }
}
export function showBroadcastError(err) {
    const es = String(err);
    oracleState._oracleMbPreSignAwaiting = false;
    oracleState._oracleMbAutoBroadcast = false;
    // Oracle roll lost the race: someone rolled the singleton first, so the node rejects ours
    // (the oracle/heartbeat UTXOs are already spent, or our tx is an orphan). No funds moved and
    // no fee was charged. Show the free-rider outcome instead of a scary error, and refresh the card.
    if (oracleState._oracleMbRollActive && /already spent|orphan|disallow|already .*mempool/i.test(es)) {
        oracleState._oracleMbRollActive = false;
        showScreen('broadcast');   // the finalize error path does not navigate; make the card visible
        const c = elementById('broadcast-result');
        c.className = 'result-card success';
        c.classList.remove('hidden');
        elementById('broadcast-result-icon').textContent = '';
        elementById('broadcast-result-msg').textContent = 'Someone rolled it first';
        elementById('broadcast-result-txid').textContent = "You're now on the fresh price. No fee charged.";
        elementById('btn-copy-txid').style.display = 'none';
        elementById('btn-broadcast-done').style.display = 'block';
        byId('broadcast-instruction')?.classList.add('hidden');
        byId('link-view-explorer')?.classList.add('hidden');
        const formCard = /** @type {HTMLElement | null} */ (document.querySelector('#screen-broadcast .card'));
        if (formCard) formCard.style.display = 'none';
        try { oracleMbCardRefresh(); } catch { /* The card refreshes again on its next poll. */ }
        return;
    }
    oracleState._oracleMbRollActive = false;
    const card = elementById('broadcast-result');
    card.className = 'result-card error';
    card.classList.remove('hidden');
    elementById('broadcast-result-icon').textContent = '';
    elementById('broadcast-result-msg').textContent = 'Broadcast failed';
    elementById('broadcast-result-txid').textContent = es;
    elementById('btn-copy-txid').style.display = 'none';
    elementById('btn-broadcast-done').style.display = 'block';
    byId('link-view-explorer')?.classList.add('hidden');
}

function scanProgressTarget(options) {
    return byId(options.progressTargetId || 'scanner-status');
}

function reportSignedDecodeError(error, options) {
    const progressTarget = scanProgressTarget(options);
    if (progressTarget) progressTarget.textContent = 'QR decode failed';
    if (options.showDecodeErrors === true) {
        toast('Signed QR decode failed: ' + error, 'error', 5000);
    } else {
        console.error('Decode error:', error);
    }
}

// Runs the anti-klepto gate. Returns a status: 'keep-scanning', 'done',
// 'failed', or 'continue' carrying the accepted payload.
function resolveSignedScanAntiKlepto(decoded, options, progressTarget) {
    try {
        const accepted = processAntiKleptoResponse(decoded);
        if (accepted === ANTI_KLEPTO_KEEP_SCANNING) {
            try { reset_qr_decoder(); } catch { /* A decoder that never started has nothing to reset. */ }
            if (progressTarget) progressTarget.textContent = 'Commitment already received — waiting for final KasKold QR';
            return { status: 'keep-scanning' };
        }
        if (options.stopCamera !== false) stopScanner();
        if (accepted === null) return { status: 'done' };
        return { status: 'continue', payload: accepted };
    } catch (error) {
        if (options.stopCamera !== false) stopScanner();
        console.error('[Companion] anti-klepto response rejected:', error);
        toast('Anti-klepto verification failed — transaction rejected: ' + error, 'error', 6000);
        return { status: 'failed' };
    }
}

function mergeKsptReturn(keptPayload) {
    console.log('[Companion] KSPT v1 return with canonical PSKB held — merging');
    try {
        const signed = JSON.parse(kaskold_sdk_complete(
            transactionState._psktReviewHex,
            keptPayload,
            networkState.network,
        ));
        const mergedSummary = openPsktReview(signed.psktHex);
        // Preserve the exact compact KSPT emitted by the signer. For the
        // next KasKold cosigner, relay these exact bytes instead of
        // rebuilding a compact transaction from the merged PSKB. This
        // keeps every firmware-authored partial-signature field and
        // trailer byte intact across signer handoff.
        transactionState._lastKasKoldKsptHex = keptPayload;
        if (mergedSummary?.finalize_ready) {
            toast('Final signature verified — ready to Finalize + broadcast', 'ok', 3500);
        } else {
            toast('Signature verified and merged — another signer is still required', 'ok', 3500);
        }
        return true;
    } catch (e) {
        console.error('[Companion] merge failed:', e);
        toast('Merge failed: ' + e, 'error', 5000);
        return null;
    }
}

function routeDecodedKspt(kspt) {
    // A PSKT or PSKB envelope opens review. Compact KSPT is handled below.
    const psktFormat = pskt_detect(kspt);
    if (psktFormat === 'pskb' || psktFormat === 'pskt') {
        console.log('[Companion] ' + psktFormat.toUpperCase() + ' detected — opening review');
        openPsktReview(kspt);
        return true;
    }
    const sigStatus = kspt_signature_status(kspt);
    // Compact-relay return path: a canonical PSKB is held from the
    // handlePsktRelayCompact send, so merge the returned partial signatures.
    if ((sigStatus === 'partial' || sigStatus === 'signed') && transactionState._psktReviewHex) {
        return mergeKsptReturn(kspt);
    }
    if (sigStatus === 'partial') {
        console.log('[Companion] Partial signature — relay to next signer');
        toast('Partial signature — scan with next device', 'info', 3000);
        displayKsptQr(kspt, 'Relay to next signer');
    } else {
        inputById('input-signed-hex').value = kspt;
        showScreen('broadcast');
    }
    return true;
}

function handleDecodedSignedFrame(decoded, options) {
    signedFrameHexSessionActive = false;
    const progressTarget = scanProgressTarget(options);
    if (progressTarget) progressTarget.textContent = '';
    console.log('[Companion] Scan complete: ' + decoded.length / 2 + ' bytes');
    const gate = resolveSignedScanAntiKlepto(decoded, options, progressTarget);
    if (gate.status === 'keep-scanning') return false;
    if (gate.status === 'done') return true;
    if (gate.status === 'failed') return null;
    return routeDecodedKspt(gate.payload);
}

export function handleSignedScan(data, options = {}) {
    const hexStr = bytesToHex(new Uint8Array(data));
    try {
        const decoded = decode_qr_frame(hexStr);
        if (decoded && decoded.length > 0) return handleDecodedSignedFrame(decoded, options);
        renderScanProgress(scanProgressTarget(options), JSON.parse(decoder_progress()));
        return false;
    } catch (e) {
        reportSignedDecodeError(e, options);
        return null;
    }
}
export async function handleBroadcastHex() {
    let hex = inputById('input-signed-hex').value.trim();
    if (!hex) { toast('Paste a signed KSPT hex string', 'error'); return; }

    try {
        hex = processAntiKleptoResponse(hex);
        if (hex === null) return;
    } catch (error) {
        console.error('[Companion] anti-klepto pasted response rejected:', error);
        toast('Anti-klepto verification failed — transaction rejected: ' + error, 'error', 6000);
        return;
    }

    // If someone pasted a PSKB/PSKT hex, route through the PSKT review.
    const psktFormat = pskt_detect(hex);
    if (psktFormat === 'pskb' || psktFormat === 'pskt') {
        openPsktReview(hex);
        return;
    }

    const sigStatus = kspt_signature_status(hex);
    if (sigStatus === 'unknown') {
        toast('Not a KSPT this Companion accepts', 'error', 5000);
        return;
    }
    if (sigStatus === 'unsigned') {
        // Signature-state inspection is advisory; WASM finalization performs the
        // authoritative transaction/signature validation before broadcast.
        toast('Warning: KSPT appears unsigned. If KasKold showed signed QR, press Broadcast.', 'error', 5000);
    }

    if (!BROADCAST_ENABLED) {
        toast('Broadcast disabled in this version — testing only', 'error', 5000);
        return;
    }

    showLoading('Broadcasting...');
    try {
        const txId = await withNodeRetry(wsUrl => broadcast_signed(hex, wsUrl));
        hideLoading();
        showBroadcastSuccess(txId);
    } catch (e) {
        hideLoading();
        showBroadcastError(e);
        console.error('Broadcast failed:', e);
    }
}
