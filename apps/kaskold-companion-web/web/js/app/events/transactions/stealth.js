import { bytesToHex } from '../../../core/bytes.js';
import { handleStealthScanResultQR, handleStealthShowScanQR } from '../../../features/stealth/index/scanning/live.js';
import { navigateBack, showScreen } from '../../navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { startScanner, stopScanner } from '../../../features/stealth/index/camera.js';
import { handleStealthFetchAnnouncements } from '../../../features/stealth/index/scanning/live.js';
import { handleStealthMeta, handleStealthSendGenerate, handleStealthSendPay, stealthFeeSetLevel, stealthShowPanel } from '../../../features/stealth/index/send.js';
// KasKold Companion Web — app/events/transactions/stealth
// Binds stealth-address generation, scanning, and payment events.

import { bindClick, byId } from '../../../core/dom.js';


export function bindStealthEvents() {
    bindClick('btn-stealth', () => { stealthShowPanel('menu'); showScreen('stealth'); });
    bindClick('btn-stealth-back', () => navigateBack('dashboard'));
    bindClick('btn-stealth-meta', () => handleStealthMeta());
    bindClick('btn-stealth-meta-back', () => stealthShowPanel('menu'));
    bindClick('btn-stealth-meta-copy', () => {
        const hex = byId('stealth-meta-hex')?.textContent || '';
        navigator.clipboard.writeText(hex).then(() => toast('Copied', 'ok', 1500));
    });
    bindClick('btn-stealth-send', () => stealthShowPanel('send'));
    bindClick('btn-stealth-send-back', () => stealthShowPanel('menu'));
    bindClick('btn-stealth-send-go', () => handleStealthSendGenerate());
    bindClick('btn-stealth-send-pay', () => handleStealthSendPay());
    bindClick('btn-sf-low', () => stealthFeeSetLevel('sf', 'send', 'low'));
    bindClick('btn-sf-normal', () => stealthFeeSetLevel('sf', 'send', 'normal'));
    bindClick('btn-sf-priority', () => stealthFeeSetLevel('sf', 'send', 'priority'));
    bindClick('btn-stealth-scan', () => stealthShowPanel('scan'));
    bindClick('btn-stealth-scan-back', () => stealthShowPanel('menu'));
    bindClick('btn-stealth-fetch-announcements', async () => {
        const button = byId('btn-stealth-fetch-announcements');
        if (!button || button.disabled) return;
        const originalText = button.textContent.trim() || '1. Fetch Announcements';
        button.disabled = true;
        button.textContent = 'Fetching Announcements…';
        try {
            await handleStealthFetchAnnouncements();
        } catch (error) {
            const status = byId('stealth-scan-status');
            if (status) status.textContent = `Could not fetch announcements: ${error}`;
            console.error('[Companion] stealth announcement fetch failed:', error);
        } finally {
            button.disabled = false;
            button.textContent = originalText;
        }
    });
    bindClick('btn-stealth-show-scan-qr', () => handleStealthShowScanQR());
    bindClick('btn-stealth-scan-result-qr', () => handleStealthScanResultQR());
    bindClick('btn-stealth-scan-meta', () => startScanner('Scan Stealth Meta-Address', (data) => {
        const bytes = new Uint8Array(data);
        let text = new TextDecoder().decode(bytes).trim();
        // Fallback: a meta QR encoded as 64 raw bytes -> hex-encode to 128 hex.
        if (!/^[0-9a-fA-F]{128}$/.test(text) && bytes.length === 64) {
            text = bytesToHex(bytes);
        }
        if (/^[0-9a-fA-F]{128}$/.test(text)) {
            stopScanner();
            const input = byId('stealth-send-meta');
            if (input) input.value = text;
            showScreen('stealth');
            stealthShowPanel('send');
            toast('Meta-address scanned', 'ok', 1500);
        }
    }));
}
