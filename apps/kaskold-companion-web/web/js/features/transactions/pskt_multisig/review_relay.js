import { networkState, transactionState } from '../../../app/state/index.js';
import { toast } from '../../../core/ui/toast.js';
import { displayKsptQr } from '../send/review.js';
import { kaskold_sdk_prepare, pskt_relay_to_kspt } from '../../../wasm/api.js';
import { byId } from '../../../core/dom.js';
import { beginAntiKlepto, clearAntiKleptoSession } from '../anti_klepto/session.js';

export function createPsktRelayActions() {
  function openRelayModal() {
    if (!transactionState._psktReviewHex) {
      toast('No PSKT loaded', 'error');
      return;
    }
    const actionLabel = byId('btn-pskt-relay')?.textContent?.trim() || 'Relay to next signer';
    const sending = actionLabel === 'Send to signer';
    const modal = byId('relay-choice-modal');
    const title = modal?.querySelector?.('.relay-dialog-title');
    const prompt = modal?.querySelector?.('.relay-dialog-prompt');
    if (title) title.textContent = sending ? 'Send to signer' : 'Relay to next signer';
    if (prompt) prompt.textContent = sending ? 'Choose the signer format' : 'Who is the next signer?';
    modal?.classList.remove('hidden');
  }

  function closeRelayModal() {
    byId('relay-choice-modal').classList.add('hidden');
  }

  function handlePsktRelay() {
    clearAntiKleptoSession();
    if (!transactionState._psktReviewHex) {
      toast('No PSKT loaded', 'error');
      return;
    }
    displayKsptQr(transactionState._psktReviewHex, 'Relay to next signer');
  }

  function handlePsktRelayKasKoldStandard() {
    clearAntiKleptoSession();
    if (!transactionState._psktReviewHex) {
      toast('No PSKT loaded', 'error');
      return;
    }
    let ksptHex = transactionState._lastKasKoldKsptHex;
    if (ksptHex) {
      console.log('[Companion] KasKold standard relay: preserving exact signer-returned KSPT v1 (' + ksptHex.length + ' hex chars)');
    } else {
      try {
        const request = JSON.parse(kaskold_sdk_prepare(
          transactionState._psktReviewHex,
          networkState.network,
        ));
        ksptHex = request.ksptHex;
      } catch (error) {
        console.error('[Companion] KasKold standard compact encode failed:', error);
        toast('KasKold relay failed: ' + error, 'error', 5000);
        return;
      }
      console.log(
        '[Companion] KasKold standard relay: PSKB hex ' + transactionState._psktReviewHex.length
        + ' → KSPT v1 hex ' + ksptHex.length,
      );
    }
    displayKsptQr(ksptHex, 'Scan with KasKold', {
      mode: 'kaskold-standard',
      instruction: 'Scan these compact transaction QR codes with KasKold. After signing, scan the signed KasKold QR back into Companion.',
      primaryScanLabel: 'Scan Signed KasKold QR',
      scannerTitle: 'Scan signed KasKold QR',
    });
  }


  function handlePsktRelayCompact() {
    if (!transactionState._psktReviewHex) {
      toast('No PSKT loaded', 'error');
      return;
    }
    let ksptHex = transactionState._lastKasKoldKsptHex;
    if (!ksptHex) {
      try {
        ksptHex = pskt_relay_to_kspt(transactionState._psktReviewHex, networkState.network);
      } catch (error) {
        console.error('[Companion] compact relay encode failed:', error);
        toast('Compact relay failed: ' + error, 'error', 5000);
        return;
      }
    }
    console.log(
      '[Companion] Compact relay: PSKB hex ' + transactionState._psktReviewHex.length
      + ' → KSPT v1 hex ' + ksptHex.length
      + ' (' + Math.round((1 - ksptHex.length / transactionState._psktReviewHex.length) * 100) + '% smaller)',
    );
    try {
      const requestHex = beginAntiKlepto(ksptHex);
      displayKsptQr(requestHex, 'Anti-klepto 1/3 — Scan request with KasKold', {
        mode: 'anti-klepto-request',
        instruction: 'Scan this request with KasKold and confirm the transaction there. KasKold will show a commitment QR first; scan that commitment back into Companion.',
        primaryScanLabel: 'Scan KasKold Commitment',
        scannerTitle: 'Scan KasKold commitment',
      });
    } catch (error) {
      console.error('[Companion] anti-klepto session failed:', error);
      toast('Anti-klepto setup failed: ' + error, 'error', 5000);
    }
  }

  return { openRelayModal, closeRelayModal, handlePsktRelay, handlePsktRelayKasKoldStandard, handlePsktRelayCompact };
}
