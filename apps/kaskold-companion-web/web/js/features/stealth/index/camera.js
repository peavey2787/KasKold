import { navigationState, scannerState, walletSession } from '../../../app/state/index.js';
import { showScreen } from '../../../app/navigation.js';
import { covShowPanel } from '../../covenants/generation/ui_and_keys.js';
import { resumeQrCycleIfPossible } from '../../transactions/send/review.js';
import { reset_qr_decoder } from '../../../wasm/api.js';
// KasKold Companion Web — features/stealth/index/camera
import { byId } from '../../../core/dom.js';
import { hexFrameBatchToBytes } from '../../../core/bytes.js';


// ─── Camera QR scanner ───

const SCAN_INTERVAL_MS = 80; // ~12.5 fps: enough for QR acquisition without full-frame decode every display frame.
const PHOTO_MAX_DIMENSION = 1600;
let lastDecodeAt = 0;
let scannerGeneration = 0;

scannerState._scannerReturnScreen = null;
scannerState._scannerReturnPanel = null;

function isSecureCameraContext() {
    if (globalThis.isSecureContext === true) return true;
    const host = globalThis.location?.hostname || '';
    return host === 'localhost' || host === '127.0.0.1' || host === '::1';
}

function cameraErrorMessage(err) {
    if (!isSecureCameraContext()) {
        return 'Live camera preview requires HTTPS in Chrome. Use Take QR Photo below, or open Companion over HTTPS.';
    }
    if (!err) return 'Live camera preview is unavailable. Use Take QR Photo below.';
    const name = err.name || '';
    if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
        return 'Camera permission was denied. Allow camera access, or use Take QR Photo below.';
    }
    return `${err.message || name || String(err)}. You can still use Take QR Photo below.`;
}

function modernCameraRequest() {
    if (!navigator.mediaDevices || typeof navigator.mediaDevices.getUserMedia !== 'function') {
        return null;
    }
    const request = navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
    const preferred = {
        audio: false,
        video: { facingMode: { ideal: 'environment' }, width: { ideal: 720 }, height: { ideal: 720 } }
    };
    return request(preferred).catch(err => {
        // Older Chromium/WebView builds often implement mediaDevices but reject
        // modern constraint dictionaries. Retry only constraint/API failures;
        // never turn a permission denial into a second permission prompt.
        const name = err && err.name ? err.name : '';
        if (name !== 'OverconstrainedError'
            && name !== 'ConstraintNotSatisfiedError'
            && name !== 'TypeError') {
            throw err;
        }
        return request({ audio: false, video: true });
    });
}

function legacyCameraRequest() {
    const legacy = navigator.getUserMedia
        || navigator.webkitGetUserMedia
        || navigator.mozGetUserMedia;
    if (typeof legacy !== 'function') {
        return Promise.reject(new Error('Live camera capture is not supported by this browser'));
    }
    return new Promise((resolve, reject) => {
        legacy.call(navigator, { audio: false, video: true }, resolve, reject);
    });
}

function requestCameraStream() {
    const modern = modernCameraRequest();
    return modern || legacyCameraRequest();
}

function stopStream(stream) {
    if (!stream || typeof stream.getTracks !== 'function') return;
    stream.getTracks().forEach(track => track.stop());
}

function teardownScannerResources() {
    if (scannerState.scanAnimFrame) {
        cancelAnimationFrame(scannerState.scanAnimFrame);
        scannerState.scanAnimFrame = null;
    }
    if (scannerState.scanStream) {
        stopStream(scannerState.scanStream);
        scannerState.scanStream = null;
    }
    lastDecodeAt = 0;
}

function resetFallbackUi() {
    byId('scanner-viewport')?.classList.remove('hidden');
    byId('scanner-camera-fallback')?.classList.add('hidden');
    byId('scanner-frame-hex-panel')?.classList.add('hidden');
    const capture = byId('scanner-capture-input');
    const image = byId('scanner-image-input');
    const frameHex = byId('scanner-frame-hex');
    if (capture) capture.value = '';
    if (image) image.value = '';
    if (frameHex) frameHex.value = '';
}

function bindFrameHexControls(generation, allowFrameHex) {
    const panel = byId('scanner-frame-hex-panel');
    const input = byId('scanner-frame-hex');
    panel?.classList.toggle('hidden', !allowFrameHex);
    if (!input) return;

    const acceptFrameHex = () => {
        if (!allowFrameHex || generation !== scannerGeneration) return;
        const value = input.value.trim();
        if (!value) {
            byId('scanner-status').textContent = 'Paste one QR Frame Hex value first.';
            return;
        }
        let frames;
        try {
            frames = hexFrameBatchToBytes(value);
            if (!frames.length) throw new Error('QR Frame Hex is empty');
        } catch (error) {
            byId('scanner-status').textContent = `Invalid QR Frame Hex: ${error.message || error}`;
            return;
        }
        input.value = '';
        for (const bytes of frames) {
            if (generation !== scannerGeneration) break;
            const result = scannerState.scanCallback?.(bytes);
            if (result !== false) break;
        }
        input.focus();
    };

    input.onpaste = event => {
        if (!allowFrameHex || generation !== scannerGeneration) return;
        const pasted = event.clipboardData?.getData('text')?.trim() || '';
        if (!pasted) return;
        event.preventDefault();
        input.value = pasted;
        acceptFrameHex();
    };
    input.onkeydown = event => {
        if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
        event.preventDefault();
        acceptFrameHex();
    };
}

function showCameraFallback(message) {
    byId('scanner-viewport')?.classList.add('hidden');
    byId('scanner-camera-fallback')?.classList.remove('hidden');
    const fallbackMessage = byId('scanner-camera-fallback-message');
    if (fallbackMessage) fallbackMessage.textContent = message;
    byId('scanner-status').textContent = 'Use the device camera or choose an existing QR image.';
}

function attachCameraStream(video, stream) {
    scannerState.scanStream = stream;
    if ('srcObject' in video) {
        video.srcObject = stream;
    } else {
        if (typeof URL === 'undefined' || typeof URL.createObjectURL !== 'function') {
            throw new Error('This browser cannot attach a camera stream to video');
        }
        video.src = URL.createObjectURL(stream);
    }
    const playback = video.play();
    if (playback && typeof playback.catch === 'function') {
        return playback;
    }
    return Promise.resolve();
}

function loadImageFile(file) {
    return new Promise((resolve, reject) => {
        if (!file) {
            reject(new Error('No image selected'));
            return;
        }
        const image = new Image();
        let objectUrl = null;
        let reader = null;
        image.onload = () => resolve({ image, release: () => {
            if (objectUrl) URL.revokeObjectURL(objectUrl);
        }});
        image.onerror = () => reject(new Error('Could not read that image'));
        if (typeof URL !== 'undefined' && typeof URL.createObjectURL === 'function') {
            objectUrl = URL.createObjectURL(file);
            image.src = objectUrl;
            return;
        }
        if (typeof FileReader !== 'function') {
            reject(new Error('This browser cannot read image files'));
            return;
        }
        reader = new FileReader();
        reader.onerror = () => reject(new Error('Could not read that image'));
        reader.onload = () => { image.src = String(reader.result || ''); };
        reader.readAsDataURL(file);
    });
}

async function scanImageFile(file, canvas, ctx, generation) {
    byId('scanner-status').textContent = 'Reading QR image…';
    const loaded = await loadImageFile(file);
    try {
        if (generation !== scannerGeneration) return;
        const sourceWidth = loaded.image.naturalWidth || loaded.image.width;
        const sourceHeight = loaded.image.naturalHeight || loaded.image.height;
        if (!sourceWidth || !sourceHeight) throw new Error('The selected image has no readable pixels');
        const scale = Math.min(1, PHOTO_MAX_DIMENSION / Math.max(sourceWidth, sourceHeight));
        canvas.width = Math.max(1, Math.round(sourceWidth * scale));
        canvas.height = Math.max(1, Math.round(sourceHeight * scale));
        ctx.drawImage(loaded.image, 0, 0, canvas.width, canvas.height);
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const code = jsQR(imageData.data, imageData.width, imageData.height, { inversionAttempts: 'attemptBoth' });
        if (!code || !code.binaryData || code.binaryData.length === 0) {
            byId('scanner-status').textContent = 'No QR code found. Try a closer, sharper photo.';
            return;
        }
        byId('scanner-status').textContent = 'QR code found';
        scannerState.scanCallback?.(new Uint8Array(code.binaryData));
    } finally {
        loaded.release();
    }
}

function bindFallbackControls(canvas, ctx, generation) {
    const capture = byId('scanner-capture-input');
    const image = byId('scanner-image-input');
    const handle = input => async () => {
        const file = input?.files?.[0];
        if (!file || generation !== scannerGeneration) return;
        try {
            await scanImageFile(file, canvas, ctx, generation);
        } catch (error) {
            if (generation === scannerGeneration) {
                byId('scanner-status').textContent = `Image error: ${error.message || error}`;
            }
        } finally {
            if (input) input.value = '';
        }
    };
    if (capture) capture.onchange = handle(capture);
    if (image) image.onchange = handle(image);
    const takePhoto = byId('btn-scanner-take-photo');
    const chooseImage = byId('btn-scanner-choose-image');
    if (takePhoto) takePhoto.onclick = () => capture?.click();
    if (chooseImage) chooseImage.onclick = () => image?.click();
}

export function startScanner(title, callback, returnPanel, options = {}) {
    const generation = ++scannerGeneration;
    teardownScannerResources();
    resetFallbackUi();
    scannerState.scanCallback = callback;
    const currentScreen = navigationState.currentScreenName || 'dashboard';
    if (currentScreen !== 'scanner' || !scannerState._scannerReturnScreen) {
        scannerState._scannerReturnScreen = currentScreen;
    }
    if (returnPanel !== undefined) scannerState._scannerReturnPanel = returnPanel;
    byId('scanner-title').textContent = title;
    byId('scanner-status').textContent = 'Starting camera...';
    showScreen('scanner');
    bindFrameHexControls(generation, options.allowFrameHex === true);
    try { reset_qr_decoder(); } catch (_) {}

    lastDecodeAt = 0;
    const video = byId('scanner-video');
    const canvas = byId('scanner-canvas');
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    bindFallbackControls(canvas, ctx, generation);

    requestCameraStream().then(stream => {
        if (generation !== scannerGeneration) {
            stopStream(stream);
            return null;
        }
        return attachCameraStream(video, stream).then(() => stream);
    }).then(stream => {
        if (!stream || generation !== scannerGeneration) {
            stopStream(stream);
            return;
        }
        byId('scanner-status').textContent = 'Point at QR code';
        scanLoop(video, canvas, ctx, performance.now(), generation);
    }).catch(err => {
        if (generation !== scannerGeneration) return;
        teardownScannerResources();
        showCameraFallback(cameraErrorMessage(err));
    });
}

function scanLoop(video, canvas, ctx, now = performance.now(), generation = scannerGeneration) {
    if (generation !== scannerGeneration || !scannerState.scanStream) return;
    const due = now - lastDecodeAt >= SCAN_INTERVAL_MS;
    if (due && video.readyState === video.HAVE_ENOUGH_DATA && video.videoWidth > 0 && video.videoHeight > 0) {
        lastDecodeAt = now;
        if (canvas.width !== video.videoWidth || canvas.height !== video.videoHeight) {
            canvas.width = video.videoWidth;
            canvas.height = video.videoHeight;
        }
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const code = jsQR(imageData.data, imageData.width, imageData.height, { inversionAttempts: 'dontInvert' });
        if (code && code.binaryData && code.binaryData.length > 0) {
            scannerState.scanCallback?.(new Uint8Array(code.binaryData));
        }
    }
    scannerState.scanAnimFrame = requestAnimationFrame(timestamp => scanLoop(video, canvas, ctx, timestamp, generation));
}

export function stopScanner() {
    ++scannerGeneration;
    teardownScannerResources();
    resetFallbackUi();
    scannerState.scanCallback = null;
    const returnScreen = scannerState._scannerReturnScreen || (walletSession.hasWallet() ? 'dashboard' : 'welcome');
    const returnPanel = scannerState._scannerReturnPanel;
    scannerState._scannerReturnScreen = null;
    scannerState._scannerReturnPanel = null;
    showScreen(returnScreen);
    if (returnPanel) covShowPanel(returnPanel);
    // If we paused a QR cycle to open the scanner and the user cancelled
    // back to the QR display, resume the animation so they aren't stuck
    // on a frozen frame with non-functional play/pause controls.
    if (returnScreen === 'qr-display') resumeQrCycleIfPossible();
}
