import { decodeHexPayloadIfMagic, downloadBytes, hasMagic, hexToBytes } from './vault_bytes.js';

export function installPrivateSwapWorkflows(ctx) {
  const { $, api, parse, show, status, openScanner, stopCamera, renderHome } = ctx;
  let lastResponse = null;

  function renderReview(review) {
    for (const [key, id] of [
      ['mode', 'private-swap-mode'], ['keyIdHex', 'private-swap-key-id'],
      ['claimPubkeyHex', 'private-swap-pubkey'], ['adaptorPointHex', 'private-swap-adaptor'],
      ['scriptHashHex', 'private-swap-script-hash'], ['sighashHex', 'private-swap-sighash'],
      ['inputAmount', 'private-swap-input'], ['outputAmount', 'private-swap-output'],
      ['fee', 'private-swap-fee'], ['refundLocktimeDaa', 'private-swap-refund'],
      ['destinationHashHex', 'private-swap-destination'],
    ]) $(id).textContent = String(review[key]);
    show('private-swap-review');
    status('Private Swap request parsed and validated inside Rust custody.');
  }

  function renderResponse(response) {
    lastResponse = response;
    $('private-swap-result-title').textContent = `Private Swap ${response.kind}`;
    $('private-swap-result-note').textContent = response.awaitingReveal
      ? 'Return this nonce point to the host, then scan the reveal from the same committed session.'
      : 'Return this protocol response to the Private Swap host.';
    $('private-swap-result-qr').innerHTML = response.svg;
    $('private-swap-result-hex').textContent = response.payloadHex;
    $('private-swap-scan-reveal').classList.toggle('hidden', !response.awaitingReveal);
    show('private-swap-result');
  }

  function prepare(bytes) {
    try {
      stopCamera();
      const result = parse(api('kaskold_vault_private_swap_prepare')(bytes));
      if (result.state === 'review') renderReview(result);
      else renderResponse(result);
    } catch (error) { status(String(error), true); }
  }

  function reveal(bytes) {
    try {
      stopCamera();
      renderResponse(parse(api('kaskold_vault_private_swap_reveal')(bytes)));
      status('Private Swap host reveal verified; adaptor pre-signature finalized.');
    } catch (error) { status(String(error), true); }
  }

  function routePayload(bytes) {
    const payload = decodeHexPayloadIfMagic(bytes, ['PSWG', 'PSWR']) || bytes;
    if (hasMagic(payload, 'PSWG')) { prepare(payload); return true; }
    if (hasMagic(payload, 'PSWR')) { reveal(payload); return true; }
    return false;
  }

  $('private-swap-cancel').onclick = () => {
    api('kaskold_vault_private_swap_cancel')();
    renderHome(false);
    status('Private Swap request rejected.');
  };
  $('private-swap-confirm').onclick = () => {
    try { renderResponse(parse(api('kaskold_vault_private_swap_confirm')())); }
    catch (error) { status(String(error), true); }
  };
  $('private-swap-scan-reveal').onclick = () => openScanner(
    'Private Swap Host Reveal',
    'Scan the reveal from the host that received this nonce point.',
    reveal,
    () => api('kaskold_vault_private_swap_cancel')(),
  );
  $('private-swap-save').onclick = () => {
    try {
      if (!lastResponse?.payloadHex) throw new Error('No Private Swap response is available.');
      downloadBytes(hexToBytes(lastResponse.payloadHex), `kaskold-private-swap-${lastResponse.kind.toLowerCase()}.bin`);
      status('Private Swap response saved.');
    } catch (error) { status(String(error), true); }
  };
  $('private-swap-done').onclick = () => {
    api('kaskold_vault_private_swap_cancel')();
    lastResponse = null;
    renderHome(false);
  };

  ctx.routePrivateSwapPayload = routePayload;
  return { routePayload, prepare };
}
