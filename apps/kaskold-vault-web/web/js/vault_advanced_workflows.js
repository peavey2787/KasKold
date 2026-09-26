import { downloadBytes, hexToBytes } from './vault_bytes.js';

function indexValue(node, label) {
  const text = node.value.trim();
  if (!/^\d+$/.test(text)) throw new Error(`${label} must be a non-negative integer.`);
  const value = Number(text);
  if (!Number.isSafeInteger(value) || value > 0x7fffffff) throw new Error(`${label} is out of range.`);
  return value;
}

function hex(bytes) {
  return Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
}

function decodeMessage(bytes) {
  if (!bytes?.length) throw new Error('Message QR is empty.');
  if (bytes.length > 4096) throw new Error('Message is too large.');
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}

export function installAdvancedWorkflows({ $, api, parse, show, status, openScanner, renderHome }) {
  let reviewedMessage = '';
  let reviewedSecret = '';
  let signedMessagePayload = null;

  $('advanced-bip85').onclick = () => show('bip85');
  $('bip85-generate').onclick = () => {
    try {
      const result = parse(api('kaskold_vault_bip85')(
        Number($('bip85-words').value),
        indexValue($('bip85-index'), 'Child index'),
      ));
      $('bip85-result').textContent = result.value;
      $('bip85-result').classList.remove('hidden');
      status('BIP85 child recovery phrase derived locally.');
    } catch (error) { status(String(error), true); }
  };

  function previewMessage(message) {
    if (!message) throw new Error('Message cannot be empty.');
    if (new TextEncoder().encode(message).length > 4096) throw new Error('Message is too large.');
    reviewedMessage = message;
    $('sign-message-value').value = message;
    $('sign-message-preview-value').textContent = message;
    show('sign-message-preview');
  }

  $('advanced-sign-message').onclick = () => show('sign-message');
  $('sign-message-type').onclick = () => { $('sign-message-value').value = ''; show('sign-message-input'); };
  $('sign-message-review').onclick = () => { try { previewMessage($('sign-message-value').value); } catch (error) { status(String(error), true); } };
  $('sign-message-scan').onclick = () => openScanner(
    'Scan Message QR',
    'Scan the exact UTF-8 message you intend to review and sign.',
    bytes => { try { previewMessage(decodeMessage(bytes)); } catch (error) { status(String(error), true); } },
  );
  $('sign-message-file').onclick = () => $('sign-message-file-input').click();
  $('sign-message-file-input').onchange = async event => {
    try {
      const file = event.target.files?.[0];
      if (!file) return;
      if (file.size > 4096) throw new Error('Message file is too large.');
      previewMessage(await file.text());
    } catch (error) { status(String(error), true); }
    finally { event.target.value = ''; }
  };
  $('sign-message-confirm').onclick = () => {
    try {
      const result = parse(api('kaskold_vault_sign_message')(reviewedMessage));
      $('sign-message-signature').textContent = result.signatureHex;
      $('sign-message-digest').textContent = result.digestHex;
      $('sign-message-qr').innerHTML = result.svg;
      signedMessagePayload = hexToBytes(result.payloadHex, 'signed-message payload');
      $('sign-message-value').value = '';
      reviewedMessage = '';
      show('sign-message-result');
      status('Message signed after explicit preview.');
    } catch (error) { status(String(error), true); }
  };
  $('sign-message-save').onclick = () => {
    try {
      if (!signedMessagePayload?.length) throw new Error('No signed message result is available.');
      downloadBytes(signedMessagePayload, 'kaskold-signed-message.bin');
      status('Signed message result saved locally.');
    } catch (error) { status(String(error), true); }
  };
  $('sign-message-done').onclick = () => { signedMessagePayload = null; renderHome(false); };

  $('advanced-commit-secret').onclick = () => show('commit-secret');
  $('commit-secret-review').onclick = () => {
    try {
      reviewedSecret = $('commit-secret-value').value;
      if (!reviewedSecret) throw new Error('Enter a secret to commit.');
      if (new TextEncoder().encode(reviewedSecret).length > 33) throw new Error('Secret must be at most 33 UTF-8 bytes.');
      $('commit-secret-preview-value').textContent = reviewedSecret;
      show('commit-secret-preview');
    } catch (error) { status(String(error), true); }
  };
  $('commit-secret-confirm').onclick = () => {
    try {
      const result = parse(api('kaskold_vault_commit_secret')(reviewedSecret));
      $('commit-secret-hash').textContent = result.commitmentHex;
      $('commit-secret-payload').textContent = result.payloadHex;
      $('commit-secret-qr').innerHTML = result.svg;
      $('commit-secret-value').value = '';
      $('commit-secret-preview-value').textContent = '';
      reviewedSecret = '';
      show('commit-secret-result');
      status('Secret committed and encrypted to this wallet after explicit review.');
    } catch (error) { status(String(error), true); }
  };
  $('commit-secret-done').onclick = () => renderHome(false);

  function showDecrypted(payloadHex) {
    const result = parse(api('kaskold_vault_decrypt_secret')(payloadHex));
    $('decrypt-secret-result').textContent = result.value;
    $('decrypt-secret-result-qr').innerHTML = api('kaskold_vault_payload_qr_svg')(new TextEncoder().encode(result.value));
    $('decrypt-secret-payload').value = '';
    show('decrypt-secret-result');
    status('Secret decrypted and commitment verified.');
  }
  $('advanced-decrypt-secret').onclick = () => show('decrypt-secret');
  $('decrypt-secret-scan').onclick = () => openScanner(
    'Decrypt Secret',
    'Scan the commit/reveal payload QR.',
    bytes => { try { showDecrypted(hex(bytes)); } catch (error) { status(String(error), true); } },
  );
  $('decrypt-secret-submit').onclick = () => {
    try {
      const payload = $('decrypt-secret-payload').value.trim();
      if (!payload) throw new Error('Enter the commit/reveal payload.');
      showDecrypted(payload);
    } catch (error) { status(String(error), true); }
  };
  $('decrypt-secret-done').onclick = () => renderHome(false);
}
