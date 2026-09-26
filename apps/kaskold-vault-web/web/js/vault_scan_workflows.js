import {
  bytesToText, decodeHexPayloadIfMagic, downloadBytes, hasMagic, hexToBytes,
} from './vault_bytes.js';

const TRANSACTION_MAGICS = ['KSPT', 'PSKB', 'PSKT', 'KAKP'];
const COVENANT_MAGICS = ['CVSG', 'CVRV'];
const PRIVATE_SWAP_MAGICS = ['PSWG', 'PSWR'];

function isTransactionPayload(bytes) {
  if (hasMagic(bytes, 'KQ')) return true;
  if (hasMagic(bytes, 'KSPT')) return bytes.length >= 5 && bytes[4] === 4;
  return TRANSACTION_MAGICS.slice(1).some(magic => hasMagic(bytes, magic));
}

function isSeedQr(bytes) {
  if (![48, 96].includes(bytes.length)) return false;
  return bytes.every(byte => byte >= 0x30 && byte <= 0x39);
}

function decodeText(bytes) {
  try { return bytesToText(bytes, true).trim(); } catch { return ''; }
}

function looksLikeDescriptor(text) {
  return text.startsWith('multi_hd45(') || text.startsWith('multi_hd(') || text.startsWith('multi(');
}

export function installScanWorkflows(ctx) {
  const { $, api, parse, show, status, stopCamera, renderHome } = ctx;
  let responseBytes = null;
  let responseFilename = 'kaskold-scan-response.bin';

  function renderBinaryResponse(title, note, result, filename) {
    responseBytes = hexToBytes(result.payloadHex, 'response payload');
    responseFilename = filename;
    $('scan-response-title').textContent = title;
    $('scan-response-note').textContent = note;
    $('scan-response-qr').innerHTML = result.svg;
    $('scan-response-hex').textContent = result.payloadHex;
    stopCamera();
    show('scan-response');
  }

  function renderCovenantBackup(result) {
    responseBytes = hexToBytes(result.value, 'covenant backup');
    responseFilename = result.value.toLowerCase().startsWith('434f5649')
      ? 'kaskold-covenant-import.cov'
      : 'kaskold-covenant-backup.cov';
    $('scan-response-title').textContent = 'Covenant Backup';
    $('scan-response-note').textContent = 'The shared M5 covenant-backup format was validated. Save the canonical raw payload to local removable storage or another offline location.';
    $('scan-response-qr').innerHTML = result.svg;
    $('scan-response-hex').textContent = result.value;
    stopCamera();
    show('scan-response');
  }

  function route(bytes) {
    if (!(bytes instanceof Uint8Array) || !bytes.length) throw new Error('Scanned QR payload is empty.');

    // Protocol-specific controllers own their own two-round state machines.
    if (ctx.routePrivateSwapPayload?.(bytes)) return true;
    if (ctx.routeCovenantPayload?.(bytes)) return true;

    // Transaction and anti-klepto payloads stay in the one shared signing scanner.
    if (isTransactionPayload(bytes)) return false;

    const text = decodeText(bytes);
    if (text.toLowerCase().startsWith('kaspa:')) {
      ctx.renderPublicImport('Kaspa Address', parse(api('kaskold_vault_validate_address')(text)));
      stopCamera();
      return true;
    }

    if (isSeedQr(bytes)) {
      ctx.stageRecoveryScan(bytes, 'SeedQR');
      return true;
    }
    if (bytes.length === 16 || bytes.length === 32) {
      ctx.stageRecoveryScan(bytes, 'Compact SeedQR');
      return true;
    }

    if (hasMagic(bytes, 'STLH') && bytes.length >= 37) {
      renderBinaryResponse(
        'Stealth Scan Response',
        'Return this STLR response to the requesting wallet. Stealth derivation ran inside the shared offline signer.',
        parse(api('kaskold_vault_stealth_scan')(bytes)),
        'kaskold-stealth-response.bin',
      );
      return true;
    }

    if (hasMagic(bytes, 'KSPR') && bytes.length === 31) {
      renderBinaryResponse(
        'Privacy Pairing Response',
        'Return this KSPB address-batch response to the requesting watch-only wallet.',
        parse(api('kaskold_vault_privacy_pairing')(bytes)),
        'kaskold-privacy-pairing-response.bin',
      );
      return true;
    }

    if (hasMagic(bytes, 'KSFU') && bytes.length === 152) {
      stopCamera();
      ctx.showFirmwareUpdate();
      status('Firmware-update QR recognized. KasKold M5 firmware updates are performed over USB-C; QR firmware installation is intentionally rejected.');
      return true;
    }

    const covenantBackup = decodeHexPayloadIfMagic(bytes, ['COVB', 'COVI']) || bytes;
    if (hasMagic(covenantBackup, 'COVB') || hasMagic(covenantBackup, 'COVI')) {
      renderCovenantBackup(parse(api('kaskold_vault_normalize_covenant_backup')(bytes)));
      return true;
    }

    const covenantMessage = decodeHexPayloadIfMagic(bytes, COVENANT_MAGICS);
    if (covenantMessage && ctx.routeCovenantPayload?.(covenantMessage)) return true;
    const swapMessage = decodeHexPayloadIfMagic(bytes, PRIVATE_SWAP_MAGICS);
    if (swapMessage && ctx.routePrivateSwapPayload?.(swapMessage)) return true;

    if (looksLikeDescriptor(text)) {
      stopCamera();
      ctx.stageMultisigDescriptor(text);
      return true;
    }

    if (text) {
      try {
        const normalized = parse(api('kaskold_vault_normalize_kpub')(text));
        stopCamera();
        ctx.renderPublicImport('kpub (Watch-Only)', normalized);
        return true;
      } catch {}
    }

    throw new Error('Unknown QR format. No wallet state was changed.');
  }

  $('scan-response-save').onclick = () => {
    try {
      if (!responseBytes?.length) throw new Error('No QR response is available to save.');
      downloadBytes(responseBytes, responseFilename);
      status('QR response saved locally.');
    } catch (error) { status(String(error), true); }
  };
  $('scan-response-done').onclick = () => {
    responseBytes = null;
    $('scan-response-qr').replaceChildren();
    $('scan-response-hex').textContent = '';
    renderHome(false);
  };

  return { route };
}
