import { byId } from '../../../core/dom.js';
import { bytesToHex, hexToBytes } from '../../../core/bytes.js';
import { renderScanProgress } from '../../../core/ui/scan_progress.js';
import { decode_qr_frame, decoder_progress } from '../../../wasm/api.js';
import { startScanner } from './camera.js';

const FRAME_MAGIC = [0x4b, 0x51];

function isFrame(bytes) {
    return FRAME_MAGIC.every((byte, index) => bytes[index] === byte);
}

function frameResult(bytes) {
    try {
        return decode_qr_frame(bytesToHex(bytes));
    } catch (error) {
        byId('scanner-status').textContent = `Unreadable QR frame: ${error.message || error}`;
        return '';
    }
}

/** A device answer too large for one QR arrives as frames; `callback` only ever sees the whole payload. */
export function wholePayload(callback) {
    return raw => {
        const bytes = new Uint8Array(raw);
        if (!isFrame(bytes)) return callback(bytes);
        const payload = frameResult(bytes);
        if (payload) return callback(hexToBytes(payload));
        renderScanProgress(byId('scanner-status'), JSON.parse(decoder_progress()));
        return false;
    };
}

export function startPayloadScanner(title, callback, returnPanel, options) {
    startScanner(title, wholePayload(callback), returnPanel, options);
}
