import { navigationState } from '../../app/state/index.js';
import { showScreen } from '../../app/navigation.js';
import { byId } from '../../core/dom.js';
import { showQrSvg } from '../../core/qr/svg_image.js';
import { setContent } from '../../core/ui/elements.js';
import { generate_qr_svg_text } from '../../wasm/api.js';
import { hideQrFrameHexDisplay, pauseQrCycle } from '../transactions/send/review.js';

// One presenter for every single-frame payload shown on the QR screen
// (covenant, crowdfunding and Private Swap invites). Transaction signing
// requests use the animated presenter in transactions/send/review.js.

const SIGNING_ONLY_CONTROLS = ['btn-scan-next-sig', 'btn-copy-kspt', 'btn-qr-scan-signed', 'qr-tx-info'];

/**
 * @param {string} payload text encoded into the QR
 * @param {string} title screen title
 * @param {string|Node} [caption] text, or built nodes, shown under the code
 * @param {string} [returnScreen] screen the QR screen's back action returns to
 */
export function showStaticPayloadQr(payload, title, caption = '', returnScreen = 'covenant') {
    pauseQrCycle();
    hideQrFrameHexDisplay();
    showQrSvg(byId('qr-container'), generate_qr_svg_text(payload), title);
    setContent(byId('qr-frame-info'), caption);
    byId('qr-display-title').textContent = title;
    for (const id of SIGNING_ONLY_CONTROLS) byId(id)?.style.setProperty('display', 'none');
    navigationState._broadcastReturnScreen = returnScreen;
    showScreen('qr-display');
}
