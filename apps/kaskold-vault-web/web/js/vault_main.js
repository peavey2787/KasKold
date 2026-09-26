import { installWalletWorkflows } from './vault_wallet_workflows.js';
import { installMultisigWorkflows } from './workflows/vault_multisig_workflows.js';
import { installAdvancedWorkflows } from './vault_advanced_workflows.js';
import { installSettings, shouldLockOnPageHide } from './vault_settings.js';
import { installTransactionWorkflows } from './vault_transaction_workflows.js';
import { installOnboarding } from './vault_onboarding.js';
import { installCovenantWorkflows } from './vault_covenant_workflows.js';
import { installPrivateSwapWorkflows } from './vault_private_swap_workflows.js';
import { installLegacyWorkflows } from './vault_legacy_workflows.js';
import { installScanWorkflows } from './vault_scan_workflows.js';
import { downloadText } from './vault_bytes.js';
import { createRecoveryWordPager } from './workflows/vault_recovery_words.js';
import { installFrameHexInput } from './workflows/vault_frame_hex_input.js';
let runtime = null;
function api(name) {
  if (!runtime) throw new Error('Vault runtime is not loaded.');
  const value = runtime[name];
  if (typeof value !== 'function') throw new Error(`Vault runtime export is missing: ${name}`);
  return value;
}
const $ = id => document.getElementById(id);
const screens = Array.from(
  document.querySelectorAll('.screen[id^="screen-"]'),
  screen => screen.id.slice('screen-'.length),
);
const history = [];
let currentScreen = 'locked';
let homeReached = false;
let stream = null;
let scanFrame = null;
let afterBackup = null;
let cameraConsumer = () => {};
let cameraCancel = () => {};
let recoveryWords = null;
let refreshSavedWalletList = () => Promise.resolve();
function show(name, push = true) {
  if (!screens.includes(name)) throw new Error(`Unknown Vault screen: ${name}`);
  if (push && currentScreen !== name) history.push(currentScreen);
  currentScreen = name;
  for (const screen of screens) $(`screen-${screen}`)?.classList.toggle('hidden', screen !== name);
  updateScreenHomeButtons();
}
function goBack() {
  clearDisplayedSecrets();
  const previous = history.pop() || (api('kaskold_vault_is_unlocked')() ? 'home' : 'locked');
  show(previous, false);
}
function goHome() {
  let unlocked = false;
  try { unlocked = Boolean(api('kaskold_vault_is_unlocked')()); } catch {}
  history.length = 0;
  if (unlocked) renderHome(false);
  else show('locked', false);
}
function updateScreenHomeButtons() {
  document.querySelectorAll('.screen-home-shortcut').forEach(button => {
    const row = button.closest('.screen-nav-row');
    const back = row?.querySelector('[data-back]');
    const backHidden = Boolean(back?.classList.contains('hidden'));
    button.classList.toggle('hidden', !homeReached || currentScreen === 'home' || backHidden);
    row?.classList.toggle('hidden', backHidden);
  });
}
function installScreenNavigation() {
  document.querySelectorAll('[data-back]').forEach(button => {
    if (button.dataset.navDecorated === 'true') return;
    button.dataset.navDecorated = 'true';
    button.classList.add('nav-back');
    const label = button.textContent.trim() || 'Back';
    button.replaceChildren();
    const icon = document.createElement('span');
    icon.className = 'nav-icon';
    icon.setAttribute('aria-hidden', 'true');
    icon.textContent = '←';
    const text = document.createElement('span');
    text.textContent = label;
    button.append(icon, text);
    const home = document.createElement('button');
    home.type = 'button';
    home.className = 'screen-home-shortcut hidden';
    home.setAttribute('aria-label', 'Home');
    home.title = 'Home';
    const image = document.createElement('img');
    image.src = 'img/hardware/icon_home_24.png';
    image.alt = '';
    const homeLabel = document.createElement('span');
    homeLabel.textContent = 'Home';
    home.append(image, homeLabel);
    home.onclick = goHome;
    const parent = button.parentElement;
    if (parent?.classList.contains('row')) {
      button.insertAdjacentElement('afterend', home);
      parent.classList.add('screen-nav-row');
    } else if (parent) {
      const row = document.createElement('div');
      row.className = 'screen-nav-row';
      parent.insertBefore(row, button);
      row.append(button, home);
    }
  });
  updateScreenHomeButtons();
}
function status(message, error = false) {
  const node = $('status-card');
  if (!node) return;
  if (!error) {
    node.textContent = '';
    node.classList.add('hidden');
    if (message) console.info(message);
    return;
  }
  node.textContent = message;
  node.style.color = 'var(--danger)';
  node.classList.remove('hidden');
}
function parse(result) { return JSON.parse(result); }
function clearSecretInputs() {
  for (const id of [
    'restore-phrase', 'restore-passphrase', 'recovery-material', 'recovery-passphrase',
    'recovery-seedqr-text', 'recovery-seedqr-passphrase', 'recovery-portable-password',
    'recovery-stego-password', 'recovery-raw-key', 'sd-wallet-password', 'wallet-add-phrase',
    'wallet-add-passphrase', 'portable-password', 'portable-password-confirm',
    'stego-password', 'stego-password-confirm', 'xprv-password', 'xprv-password-confirm',
    'multisig-cosigners',
    'multisig-import-descriptor', 'last-word-prefix', 'sign-message-value',
    'commit-secret-value', 'decrypt-secret-payload', 'frame-hex', 'onboarding-passphrase',
    'onboarding-passphrase-confirm', 'storage-credential', 'storage-credential-confirm', 'unlock-saved-credential',
  ]) {
    const node = $(id);
    if (node) node.value = '';
  }
  $('export-key-index').value = '0';
  document.querySelectorAll('input[type="file"]').forEach(input => { input.value = ''; });
}
function clearDisplayedSecrets() {
  for (const id of [
    'backup-word', 'backup-all-words', 'reveal-phrase', 'secret-text-value', 'export-key-value',
    'bip85-result', 'last-word-result', 'sign-message-signature', 'sign-message-digest',
    'commit-secret-hash', 'commit-secret-payload', 'commit-secret-preview-value', 'decrypt-secret-result',
    'sign-message-preview-value',
  ]) {
    const node = $(id);
    if (node) node.textContent = '';
  }
  for (const id of [
    'export-key-value', 'bip85-result', 'last-word-result', 'sign-message-qr',
    'commit-secret-qr', 'decrypt-secret-result',
  ]) {
    $(id)?.classList.add('hidden');
  }
  for (const id of [
    'seedqr-output', 'secret-text-qr', 'sign-message-qr', 'commit-secret-qr', 'decrypt-secret-result-qr', 'multisig-kpub-qr',
    'multisig-result-address-qr', 'multisig-result-descriptor-qr',
  ]) {
    $(id)?.replaceChildren();
  }
  $('multisig-kpub').textContent = '';
  $('multisig-result-address').textContent = '';
  $('multisig-result-descriptor').textContent = '';
  $('multisig-kpub-panel')?.classList.add('hidden');
}
function renderHome(push = true) {
  clearDisplayedSecrets();
  homeReached = true;
  show('home', push);
}
function renderConnect() {
  const { kpub } = parse(api('kaskold_vault_export_kpub')());
  $('home-kpub').textContent = kpub;
  $('kpub-qr').innerHTML = api('kaskold_vault_kpub_qr_svg')();
  show('connect');
}
function renderWalletDetails() {
  const wallets = parse(api('kaskold_vault_wallets')());
  const wallet = wallets.find(entry => entry.active);
  if (!wallet) throw new Error('No active wallet.');
  $('details-name').textContent = wallet.name;
  $('details-name-input').value = wallet.name;
  $('details-kind').textContent = wallet.kind;
  $('details-fingerprint').textContent = wallet.fingerprint || 'Unavailable for legacy wallet metadata';
  $('details-kpub').textContent = wallet.kpub || '';
  $('details-kpub-panel').classList.toggle('hidden', !wallet.kpub);
  $('details-delete').dataset.walletIndex = String(wallet.index);
  show('wallet-details');
}
function renderSeedQr(kind) {
  const calls = {
    standard: ['SeedQR Backup', 'kaskold_vault_seedqr_svg'],
    compact: ['Compact SeedQR', 'kaskold_vault_compact_seedqr_svg'],
    plain: ['Plain-text SeedQR', 'kaskold_vault_plain_seedqr_svg'],
  };
  const entry = calls[kind];
  if (!entry) throw new Error('Unknown SeedQR backup type.');
  $('seedqr-title').textContent = entry[0];
  $('seedqr-output').innerHTML = api(entry[1])();
  show('seedqr');
}
function renderSecretText(title, warning, value, svg = '') {
  $('secret-text-title').textContent = title;
  $('secret-text-warning').textContent = warning;
  $('secret-text-value').textContent = value;
  $('secret-text-qr').innerHTML = svg;
  $('secret-text-qr').classList.toggle('hidden', !svg);
  show('secret-text');
}
function openScanner(title, instruction, consumer, cancel = () => {}) {
  stopCamera();
  cameraConsumer = consumer;
  cameraCancel = cancel;
  $('scan-title').textContent = title;
  $('scan-progress').textContent = instruction;
  $('frame-hex').value = '';
  show('scan');
}

async function startCamera() {
  stopCamera();
  if (!navigator.mediaDevices?.getUserMedia) throw new Error('Camera access is unavailable in this browser.');
  stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: 'environment' } }, audio: false });
  const video = $('camera');
  video.srcObject = stream;
  await video.play();
  const canvas = $('camera-canvas');
  const context = canvas.getContext('2d', { willReadFrequently: true });
  const tick = () => {
    if (!stream) return;
    if (video.readyState >= 2 && video.videoWidth && video.videoHeight) {
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      context.drawImage(video, 0, 0, canvas.width, canvas.height);
      const image = context.getImageData(0, 0, canvas.width, canvas.height);
      const code = globalThis.jsQR?.(image.data, image.width, image.height, { inversionAttempts: 'attemptBoth' });
      if (code) {
        try {
          const bytes = code.binaryData?.length ? new Uint8Array(code.binaryData) : new TextEncoder().encode(code.data);
          cameraConsumer(bytes);
        } catch (error) { status(String(error), true); }
      }
    }
    scanFrame = requestAnimationFrame(tick);
  };
  scanFrame = requestAnimationFrame(tick);
}
function stopCamera() {
  if (scanFrame !== null) cancelAnimationFrame(scanFrame);
  scanFrame = null;
  if (stream) stream.getTracks().forEach(track => track.stop());
  stream = null;
  const video = $('camera');
  if (video) video.srcObject = null;
}

async function boot() {
  try {
    runtime = await import('../pkg/vault_web.js');
    await api('default')();
    status('Vault ready.');
  } catch (error) {
    status(`Vault runtime failed to load: ${error}. Build it with make vault-web, then serve target/kaskold-vault-web/site or apps/kaskold-vault-web/web.`, true);
    document.querySelectorAll('button').forEach(button => { button.disabled = true; });
    return;
  }

  recoveryWords = createRecoveryWordPager({ $, api, parse, show, goBack, history });
  recoveryWords.install();
  installScreenNavigation();
  document.querySelectorAll('[data-back]').forEach(button => { button.onclick = goBack; });
  $('brand-home').onclick = goHome;
  $('backup-ack-done').onclick = () => {
    const next = afterBackup;
    afterBackup = null;
    recoveryWords?.clear();
    history.length = 0;
    if (next) next(); else renderHome(false);
  };

  $('home-connect').onclick = renderConnect;
  $('home-wallet').onclick = () => show('wallet');
  $('home-settings').onclick = () => show('settings');
  $('copy-kpub').onclick = async () => {
    try { await navigator.clipboard.writeText($('home-kpub').textContent); status('kpub copied.'); }
    catch { status('Could not copy kpub.', true); }
  };
  $('save-kpub').onclick = () => {
    try {
      const kpub = $('home-kpub').textContent.trim();
      if (!kpub) throw new Error('No kpub is available to save.');
      downloadText(`${kpub}\n`, 'kaskold-watch-only.kpub');
      status('Watch-only kpub saved locally.');
    } catch (error) { status(String(error), true); }
  };

  $('wallet-backup').onclick = () => show('backup-methods');
  $('wallet-details').onclick = () => { try { renderWalletDetails(); } catch (error) { status(String(error), true); } };
  $('details-rename').onclick = () => {
    try {
      const index = Number($('details-delete').dataset.walletIndex);
      const name = $('details-name-input').value.trim();
      api('kaskold_vault_set_wallet_name')(index, name);
      renderWalletDetails();
      status('Wallet renamed.');
    } catch (error) { status(String(error), true); }
  };
  $('details-delete').onclick = () => {
    $('delete-wallet-confirm').dataset.walletIndex = $('details-delete').dataset.walletIndex;
    show('delete-wallet');
  };
  $('delete-wallet-cancel').onclick = goBack;
  $('delete-wallet-confirm').onclick = () => {
    try {
      const index = Number($('delete-wallet-confirm').dataset.walletIndex);
      if (!Number.isInteger(index)) throw new Error('Invalid active wallet.');
      api('kaskold_vault_delete_wallet')(index);
      clearDisplayedSecrets();
      history.length = 0;
      if (api('kaskold_vault_is_unlocked')()) renderHome(false);
      else { show('locked', false); status('Wallet deleted. Vault is locked because no wallets remain.'); }
    } catch (error) { status(String(error), true); }
  };
  $('wallet-advanced').onclick = () => show('wallet-advanced');

  $('backup-view-words').onclick = () => { try { recoveryWords.reveal(); } catch (error) { status(String(error), true); } };
  $('backup-seedqr').onclick = () => { try { renderSeedQr('standard'); } catch (error) { status(String(error), true); } };
  $('backup-methods-advanced').onclick = () => show('backup-advanced');
  $('advanced-compact-seedqr').onclick = () => { try { renderSeedQr('compact'); } catch (error) { status(String(error), true); } };
  $('advanced-plain-seedqr').onclick = () => { try { renderSeedQr('plain'); } catch (error) { status(String(error), true); } };
  $('advanced-xprv').onclick = () => show('xprv-export');
  $('xprv-show-qr').onclick = () => {
    try {
      const { value } = parse(api('kaskold_vault_backup_xprv')());
      const svg = api('kaskold_vault_payload_qr_svg')(new TextEncoder().encode(value));
      renderSecretText('XPrv Backup', 'This XPrv can control the wallet account. Keep it private and offline.', value, svg);
    } catch (error) { status(String(error), true); }
  };
  $('xprv-encrypt-sd').onclick = () => show('xprv-encrypt');
  $('xprv-download').onclick = () => {
    try {
      const password = $('xprv-password').value;
      if (!password) throw new Error('Enter a backup password.');
      if (password !== $('xprv-password-confirm').value) throw new Error('Backup passwords do not match.');
      const bytes = api('kaskold_vault_portable_xprv_backup')(password);
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = 'kaskold-account-xprv-backup.kwp';
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      $('xprv-password').value = '';
      $('xprv-password-confirm').value = '';
      status('Encrypted account-XPrv backup created.');
    } catch (error) { status(String(error), true); }
  };
  $('advanced-export-key').onclick = () => { $('export-key-value').textContent = ''; $('export-key-value').classList.add('hidden'); show('export-key'); };
  $('export-key-show').onclick = () => {
    try {
      const text = $('export-key-index').value.trim();
      if (!/^\d+$/.test(text)) throw new Error('Address index must be a non-negative integer.');
      const index = Number(text);
      if (!Number.isSafeInteger(index) || index > 65535) throw new Error('Address index must be between 0 and 65535.');
      const { value } = parse(api('kaskold_vault_export_receive_key')(index));
      $('export-key-value').textContent = value;
      $('export-key-value').classList.remove('hidden');
    } catch (error) { status(String(error), true); }
  };

  const transactionWorkflows = installTransactionWorkflows({
    $, api, parse, show, status, openScanner, stopCamera, renderHome,
  });
  cameraConsumer = transactionWorkflows.acceptFrame;
  const onboarding = installOnboarding({ $, api, parse, show, status, showCreatedPhrase, renderHome });
  refreshSavedWalletList = onboarding.refreshSavedWalletList;
  $('show-restore').onclick = onboarding.startRestore;
  const workflowContext = {
    $, api, parse, show, status, renderSecretText, renderReview: transactionWorkflows.renderReview,
    showCreatedPhrase, renderHome, lockVault, openScanner, stopCamera,
    acceptTransactionFrame: transactionWorkflows.acceptFrame,
    applyStagedRestoreName: onboarding.applyStagedRestoreName,
    startSeedTool: onboarding.startSeedTool,
    openWalletPicker: onboarding.openWalletPicker,
  };
  installWalletWorkflows(workflowContext);
  installMultisigWorkflows(workflowContext);
  installAdvancedWorkflows(workflowContext);
  installCovenantWorkflows(workflowContext);
  installPrivateSwapWorkflows(workflowContext);
  installLegacyWorkflows(workflowContext);
  const settings = installSettings({
    ...workflowContext,
    openPortableBackup: () => workflowContext.openPortableBackup(),
    deleteSavedWallet: onboarding.deleteSavedWallet,
  });
  workflowContext.showFirmwareUpdate = settings.showFirmwareUpdate;
  const scanWorkflows = installScanWorkflows(workflowContext);
  transactionWorkflows.setSpecialRouter(scanWorkflows.route);

  $('camera-start').onclick = () => { startCamera().catch(error => status(String(error), true)); };
  $('camera-stop').onclick = stopCamera;
  installFrameHexInput({
    input: $('frame-hex'),
    consume: bytes => cameraConsumer(bytes),
    reportError: message => status(message, true),
  });
  $('scan-cancel').onclick = () => {
    stopCamera();
    try { cameraCancel(); } catch {}
    cameraConsumer = transactionWorkflows.acceptFrame;
    cameraCancel = () => {};
    goBack();
  };
  $('lock-vault').onclick = lockVault;
}

function lockVault() {
  stopCamera();
  clearSecretInputs();
  clearDisplayedSecrets();
  api('kaskold_vault_lock')();
  homeReached = false;
  recoveryWords?.clear();
  $('home-kpub').textContent = '';
  $('details-kpub').textContent = '';
  $('kpub-qr').replaceChildren();
  history.length = 0;
  $('wallet-picker-back')?.classList.add('hidden');
  show('locked', false);
  void refreshSavedWalletList();
  status('Vault locked.');
}

function showCreatedPhrase(phrase, next = null) {
  afterBackup = next;
  recoveryWords.start(phrase, { acknowledgement: true, resetHistory: true });
  status('Wallet created. Record each recovery word in order before continuing.');
}

addEventListener('pagehide', () => {
  stopCamera();
  if (shouldLockOnPageHide()) { try { api('kaskold_vault_lock')(); } catch {} }
});
boot();
