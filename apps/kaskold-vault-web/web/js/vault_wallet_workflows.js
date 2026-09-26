import { downloadBytes, nonNegativeInteger, readFile, requireMatchingPasswords } from './vault_bytes.js';
import { normalizeRecoveryPhraseInput } from './workflows/vault_recovery_words.js';

export function installWalletWorkflows(ctx) {
  const { $, api, parse, show, status, renderSecretText, renderReview, renderHome, openScanner, stopCamera, openWalletPicker, applyStagedRestoreName = () => {} } = ctx;
  let pendingRecoveryBytes = null;
  let latestPublicImport = null;


  function activeWallet() {
    const wallets = parse(api('kaskold_vault_wallets')());
    return wallets.find(wallet => wallet.active);
  }

  function renderReceive() {
    try {
      const wallet = activeWallet();
      if (!wallet) throw new Error('No active wallet.');
      const raw = wallet.kind === 'Raw Private Key';
      if (raw) {
        $('receive-chain').value = '0';
        $('receive-index').value = '0';
      }
      $('receive-chain').disabled = raw;
      $('receive-index').disabled = raw;
      $('receive-prev').disabled = raw;
      $('receive-next').disabled = raw;
      const index = nonNegativeInteger($('receive-index').value, 'Address index');
      const chain = $('receive-chain').value === '1';
      const result = parse(api('kaskold_vault_receive_address')($('receive-network').value, chain, index));
      $('receive-address').textContent = result.address;
      $('receive-qr').innerHTML = result.svg;
      show('receive');
    } catch (error) { status(String(error), true); }
  }


  $('wallet-receive').onclick = renderReceive;
  $('receive-network').onchange = renderReceive;
  $('receive-chain').onchange = renderReceive;
  $('receive-index').onchange = renderReceive;
  $('receive-prev').onclick = () => {
    const index = nonNegativeInteger($('receive-index').value, 'Address index');
    $('receive-index').value = String(Math.max(0, index - 1));
    renderReceive();
  };
  $('receive-next').onclick = () => {
    const index = nonNegativeInteger($('receive-index').value, 'Address index');
    if (index >= 0x7fffffff) return status('Address index is at the supported maximum.', true);
    $('receive-index').value = String(index + 1);
    renderReceive();
  };
  $('receive-fullscreen').onclick = () => {
    const address = $('receive-address').textContent.trim();
    if (!address) return status('No receive address is available.', true);
    $('receive-qr-address').textContent = address;
    $('receive-qr-full').innerHTML = api('kaskold_vault_payload_qr_svg')(new TextEncoder().encode(address));
    show('receive-qr');
  };

  $('wallet-recovery').onclick = () => show('wallet-recovery');
  $('recovery-words-open').onclick = () => show('recovery-words');
  $('recovery-seedqr-open').onclick = () => show('recovery-seedqr');
  $('recovery-sd-open').onclick = () => show('recovery-sd');
  $('recovery-advanced-open').onclick = () => show('recovery-advanced');
  $('recovery-stego-open').onclick = () => show('recovery-stego');
  $('recovery-raw-open').onclick = () => show('recovery-raw');

  $('recovery-material-submit').onclick = () => {
    try {
      const material = normalizeRecoveryPhraseInput($('recovery-material').value);
      if (!material) throw new Error('Enter recovery words.');
      api('kaskold_vault_recover_material')(new TextEncoder().encode(material), $('recovery-passphrase').value);
      applyStagedRestoreName();
      $('recovery-material').value = '';
      $('recovery-passphrase').value = '';
      renderHome();
      status('Recovery words restored and selected.');
    } catch (error) { status(String(error), true); }
  };

  const importSeedQrBytes = (bytes, passphraseId, success) => {
    api('kaskold_vault_recover_material')(bytes, $(passphraseId)?.value || '');
    applyStagedRestoreName();
    if ($(passphraseId)) $(passphraseId).value = '';
    pendingRecoveryBytes = null;
    $('recovery-seedqr-text').value = '';
    renderHome();
    status(success);
  };

  function stageRecoveryScan(bytes, label = 'SeedQR') {
    pendingRecoveryBytes = new Uint8Array(bytes);
    $('recovery-seedqr-text').value = '';
    $('recovery-seedqr-text').placeholder = `${label} scanned. Enter an optional BIP39 passphrase, then restore.`;
    stopCamera();
    show('recovery-seedqr');
    status(`${label} validated as recovery material. Confirm the optional BIP39 passphrase before restoring.`);
  }

  $('recovery-seedqr-submit').onclick = () => {
    try {
      let bytes = pendingRecoveryBytes;
      if (!bytes) {
        const text = $('recovery-seedqr-text').value.replace(/\s+/g, '');
        if (!/^\d{48}(?:\d{48})?$/.test(text)) throw new Error('Standard SeedQR must contain exactly 48 or 96 digits.');
        bytes = new TextEncoder().encode(text);
      }
      importSeedQrBytes(bytes, 'recovery-seedqr-passphrase', 'Recovery material restored and selected.');
    } catch (error) { status(String(error), true); }
  };
  $('recovery-seedqr-scan').onclick = () => {
    openScanner('Scan SeedQR', 'Scan a standard numeric SeedQR.', bytes => {
      try { stageRecoveryScan(bytes, 'SeedQR'); }
      catch (error) { status(String(error), true); }
    });
  };
  $('recovery-compact-open').onclick = () => {
    openScanner('Compact SeedQR', 'Scan a 16-byte or 32-byte Compact SeedQR.', bytes => {
      try { stageRecoveryScan(bytes, 'Compact SeedQR'); }
      catch (error) { status(String(error), true); }
    });
  };
  $('recovery-plain-open').onclick = () => {
    openScanner('Plain-text SeedQR', 'Scan a plain-text BIP39 recovery phrase QR.', bytes => {
      try {
        const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes).trim();
        if (!text) throw new Error('Plain-text SeedQR is empty.');
        $('recovery-material').value = text;
        stopCamera();
        show('recovery-words');
        status('Recovery phrase scanned. Confirm the optional BIP39 passphrase before restoring.');
      } catch (error) { status(String(error), true); }
    });
  };

  $('recovery-portable-submit').onclick = async () => {
    try {
      const bytes = await readFile($('recovery-portable-file'), 'an encrypted backup file');
      api('kaskold_vault_restore_portable')(bytes, $('recovery-portable-password').value);
      applyStagedRestoreName();
      $('recovery-portable-password').value = '';
      renderHome();
      status('Encrypted backup authenticated, restored, and selected.');
    } catch (error) { status(String(error), true); }
  };
  $('recovery-stego-submit').onclick = async () => {
    try {
      const bytes = await readFile($('recovery-stego-file'), 'a steganographic JPEG');
      api('kaskold_vault_restore_stego')(bytes, $('recovery-stego-password').value);
      applyStagedRestoreName();
      $('recovery-stego-password').value = '';
      renderHome();
      status('Steganographic backup authenticated, restored, and selected.');
    } catch (error) { status(String(error), true); }
  };
  $('recovery-raw-submit').onclick = () => {
    try {
      const value = $('recovery-raw-key').value.trim();
      if (!/^[0-9a-fA-F]{64}$/.test(value)) throw new Error('Raw private key must contain exactly 64 hexadecimal characters.');
      api('kaskold_vault_import_raw_key')(value);
      applyStagedRestoreName();
      $('recovery-raw-key').value = '';
      renderHome();
      status('Raw private key imported and selected.');
    } catch (error) { status(String(error), true); }
  };

  const renderPublicImport = (title, result) => {
    latestPublicImport = { title, ...result };
    $('import-public-title').textContent = title;
    $('import-public-value').textContent = result.value;
    $('import-public-qr').innerHTML = result.svg;
    show('import-public');
  };

  $('sd-import-wallet-backup').onclick = () => show('sd-wallet-backup');
  $('sd-import-transaction').onclick = () => show('sd-transaction');
  $('sd-import-kpub').onclick = () => show('sd-kpub');
  $('sd-import-multisig-address').onclick = () => show('sd-multisig-address');
  $('sd-import-covenant').onclick = () => show('sd-covenant');
  $('sd-import-raw').onclick = () => show('recovery-raw');

  $('sd-wallet-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-wallet-file'), 'a Seed / XPrv backup file');
      let plain = '';
      try { plain = new TextDecoder('utf-8', { fatal: true }).decode(bytes).trim(); } catch {}
      if (plain.startsWith('kprv')) {
        api('kaskold_vault_import_xprv')(plain);
        applyStagedRestoreName();
        status('Account XPrv imported and selected.');
      } else {
        const password = $('sd-wallet-password').value;
        if (!password) throw new Error('Enter the encrypted backup password.');
        api('kaskold_vault_restore_portable')(bytes, password);
        applyStagedRestoreName();
        status('Encrypted wallet backup authenticated, restored, and selected.');
      }
      $('sd-wallet-password').value = '';
      $('sd-wallet-file').value = '';
      renderHome();
    } catch (error) {
      $('sd-wallet-password').value = '';
      status(String(error), true);
    }
  };

  $('sd-transaction-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-transaction-file'), 'a transaction file');
      const review = parse(api('kaskold_vault_open_transaction_file')(bytes));
      $('sd-transaction-file').value = '';
      renderReview(review);
      status('Transaction loaded. Review every field before approving.');
    } catch (error) { status(String(error), true); }
  };

  $('sd-kpub-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-kpub-file'), 'a kpub file');
      const text = new TextDecoder().decode(bytes).trim();
      renderPublicImport('kpub (Watch-Only)', parse(api('kaskold_vault_normalize_kpub')(text)));
      $('sd-kpub-file').value = '';
    } catch (error) { status(String(error), true); }
  };

  $('sd-multisig-address-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-multisig-address-file'), 'a multisig address file');
      const text = new TextDecoder().decode(bytes).trim();
      renderPublicImport('Multisig Address', parse(api('kaskold_vault_validate_address')(text)));
      $('sd-multisig-address-file').value = '';
    } catch (error) { status(String(error), true); }
  };

  $('sd-multisig-descriptor-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-multisig-descriptor-file'), 'a multisig descriptor file');
      const descriptor = new TextDecoder().decode(bytes).trim();
      if (!descriptor) throw new Error('Multisig descriptor file is empty.');
      const result = parse(api('kaskold_vault_import_multisig')(
        descriptor,
        $('sd-multisig-descriptor-network').value,
        Number($('sd-multisig-descriptor-chain').value),
        nonNegativeInteger($('sd-multisig-descriptor-index').value, 'Address index'),
      ));
      $('sd-multisig-descriptor-file').value = '';
      const metadata = {
        network: $('sd-multisig-descriptor-network').value,
        chain: Number($('sd-multisig-descriptor-chain').value),
        index: nonNegativeInteger($('sd-multisig-descriptor-index').value, 'Address index'),
      };
      if (ctx.multisigDescriptorRestorePending && typeof ctx.persistMultisigDescriptor === 'function') {
        await ctx.persistMultisigDescriptor(result, metadata);
        ctx.multisigDescriptorRestorePending = false;
        status('Multisig descriptor restored for the active wallet.');
      } else {
        status('Multisig descriptor validated and imported.');
      }
      ctx.renderMultisigResult(result, metadata, false);
    } catch (error) { status(String(error), true); }
  };

  $('sd-covenant-submit').onclick = async () => {
    try {
      const bytes = await readFile($('sd-covenant-file'), 'a covenant restore file');
      const result = parse(api('kaskold_vault_normalize_covenant_backup')(bytes));
      renderPublicImport('Covenant Restore', result);
      $('sd-covenant-file').value = '';
      status('Covenant restore payload validated against the shared hardware format.');
    } catch (error) { status(String(error), true); }
  };

  $('wallet-switch').onclick = () => {
    if (typeof openWalletPicker !== 'function') return status('Shared wallet picker is unavailable.', true);
    void openWalletPicker({ fromMenu: true });
  };
  $('wallet-add-open').onclick = () => show('wallet-add');

  $('backup-encrypted-sd').onclick = () => show('portable-backup');
  $('portable-download').onclick = () => {
    try {
      const password = requireMatchingPasswords($('portable-password').value, $('portable-password-confirm').value);
      const bytes = api('kaskold_vault_portable_backup')(password);
      downloadBytes(bytes, 'kaskold-vault-backup.kwp');
      $('portable-password').value = '';
      $('portable-password-confirm').value = '';
      status('Encrypted portable backup created. Store it separately from the password.');
    } catch (error) { status(String(error), true); }
  };

  $('advanced-stego').onclick = () => show('stego-backup');
  $('stego-download').onclick = async () => {
    try {
      const carrier = await readFile($('stego-carrier'), 'a baseline JPEG carrier');
      const password = requireMatchingPasswords($('stego-password').value, $('stego-password-confirm').value);
      const encoded = api('kaskold_vault_stego_backup')(carrier, password);
      downloadBytes(encoded, 'kaskold-steganographic-backup.jpg', 'image/jpeg');
      $('stego-password').value = '';
      $('stego-password-confirm').value = '';
      status('Steganographic encrypted backup created.');
    } catch (error) { status(String(error), true); }
  };

  $('import-public-save').onclick = () => {
    try {
      if (!latestPublicImport?.value) throw new Error('No imported public value is available.');
      const slug = latestPublicImport.title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'public';
      downloadBytes(new TextEncoder().encode(`${latestPublicImport.value}\n`), `kaskold-${slug}.txt`, 'text/plain');
      status(`${latestPublicImport.title} saved locally.`);
    } catch (error) { status(String(error), true); }
  };

  ctx.openPortableBackup = () => show('portable-backup');
  ctx.downloadBytes = downloadBytes;
  ctx.renderSecretText = renderSecretText;
  ctx.stageRecoveryScan = stageRecoveryScan;
  ctx.renderPublicImport = renderPublicImport;
  ctx.latestPublicImport = () => latestPublicImport;
}
