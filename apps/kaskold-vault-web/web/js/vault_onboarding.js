import { deleteEncryptedWallet, loadEncryptedWallet, saveEncryptedWallet, updateEncryptedWalletIdentity } from './vault_storage.js';

function encodeTouchSamples(samples) {
  const bytes = new Uint8Array(samples.length * 8);
  const view = new DataView(bytes.buffer);
  samples.forEach((sample, index) => {
    const offset = index * 8;
    view.setUint32(offset, sample.time, true);
    view.setUint16(offset + 4, sample.x, true);
    view.setUint16(offset + 6, sample.y, true);
  });
  return bytes;
}

function validCredential(type, value) {
  if (type === 'pin') return /^\d{6,12}$/.test(value); // legacy unlock metadata only
  return value.length >= 12 && /[A-Za-z]/.test(value) && /\d/.test(value);
}

export function installOnboarding({ $, api, parse, show, status, showCreatedPhrase, renderHome }) {
  const creationFlow = parse(api('kaskold_vault_creation_flow')());
  const diceTargets = Array.isArray(creationFlow.diceRollTargets) ? creationFlow.diceRollTargets : [];
  const touchTarget = Number(creationFlow.touchEntropyTarget);
  if (diceTargets.length !== 4 || diceTargets.some(target => !Number.isInteger(target) || target <= 0)) {
    throw new Error('Vault creation flow returned invalid dice targets.');
  }
  if (!Number.isInteger(touchTarget) || touchTarget <= 0) {
    throw new Error('Vault creation flow returned an invalid touch target.');
  }

  let wordCount = 24;
  let diceRolls = [];
  let diceTarget = 0;
  let touchSamples = [];
  let passphrase = '';
  let credentialType = 'password';
  let creationMode = 'fresh';
  let walletName = '';
  let stagedRestoreName = '';
  let nameContinuation = null;
  let entropyMode = 'choice';
  let afterCreationStorage = null;
  let selectedSavedWalletId = null;

  function resetEntropy() {
    diceRolls = [];
    diceTarget = 0;
    touchSamples = [];
    passphrase = '';
    $('onboarding-passphrase').value = '';
    $('onboarding-passphrase-confirm').value = '';
  }

  function updateDiceProgress() {
    $('onboarding-dice-progress').textContent = `${diceRolls.length}/${diceTarget} rolls collected.`;
  }

  function updateTouchProgress() {
    $('onboarding-touch-progress').textContent = `${touchSamples.length}/${touchTarget} movement samples collected.`;
  }

  function finishCreation() {
    const touch = encodeTouchSamples(touchSamples);
    try {
      const create = creationMode === 'add'
        ? 'kaskold_vault_add_create_with_entropy'
        : 'kaskold_vault_create_with_entropy';
      const created = parse(api(create)(wordCount, diceRolls.join(''), touch, passphrase));
      api('kaskold_vault_set_wallet_name')(parse(api('kaskold_vault_wallets')()).find(wallet => wallet.active).index, walletName);
      showCreatedPhrase(created.recoveryPhrase, () => show('storage-finalize', false));
    } finally {
      touch.fill(0);
      resetEntropy();
    }
  }

  async function refreshSavedWalletList() {
    const section = $('saved-wallets');
    const list = $('saved-wallet-list');
    list.replaceChildren();
    selectedSavedWalletId = null;
    try {
      const record = await loadEncryptedWallet();
      const wallets = Array.isArray(record?.wallets) ? record.wallets : [];
      wallets.forEach((wallet, index) => {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'saved-wallet-button';
        const name = document.createElement('span');
        name.textContent = wallet?.name || `Wallet ${index + 1}`;
        const action = document.createElement('small');
        action.textContent = wallet.credentialType === 'pin' ? 'Unlock legacy PIN wallet' : 'Unlock with password';
        button.append(name, action);
        button.onclick = () => {
          selectedSavedWalletId = wallet.id;
          $('unlock-saved-name').textContent = name.textContent;
          $('unlock-saved-credential').value = '';
          show('unlock-saved');
        };
        list.append(button);
      });
      section.classList.toggle('hidden', wallets.length === 0);
      $('wallet-picker-empty')?.classList.toggle('hidden', wallets.length !== 0);
    } catch {
      section.classList.add('hidden');
      list.replaceChildren();
      $('wallet-picker-empty')?.classList.remove('hidden');
    }
  }

  function chooseWordCount(count) {
    wordCount = count;
    if (entropyMode === 'none') return show('onboarding-passphrase-choice');
    if (entropyMode === 'dice') return show('onboarding-dice-count');
    if (entropyMode === 'touch') {
      touchSamples = [];
      updateTouchProgress();
      return show('onboarding-touch');
    }
    show('onboarding-dice-choice');
  }

  function startCreation(mode = 'fresh', additive = 'choice', next = null) {
    resetEntropy();
    creationMode = mode;
    entropyMode = additive;
    afterCreationStorage = next;
    const wallets = (() => { try { return parse(api('kaskold_vault_wallets')()); } catch { return []; } })();
    walletName = `Wallet ${wallets.length + 1}`;
    $('wallet-name-entry').value = walletName;
    nameContinuation = () => show('onboarding-word-count');
    show('wallet-name-entry');
  }

  function startRestore() {
    const wallets = (() => { try { return parse(api('kaskold_vault_wallets')()); } catch { return []; } })();
    walletName = `Wallet ${wallets.length + 1}`;
    $('wallet-name-entry').value = walletName;
    nameContinuation = () => { stagedRestoreName = walletName; show('restore-source'); };
    show('wallet-name-entry');
  }

  function applyStagedRestoreName() {
    if (!stagedRestoreName) return;
    const active = parse(api('kaskold_vault_wallets')()).find(wallet => wallet.active);
    if (active) api('kaskold_vault_set_wallet_name')(active.index, stagedRestoreName);
    stagedRestoreName = '';
  }

  $('onboarding-create').onclick = () => {
    let unlocked = false;
    try { unlocked = Boolean(api('kaskold_vault_is_unlocked')()); } catch {}
    startCreation(unlocked ? 'add' : 'fresh', 'choice', null);
  };
  $('wallet-name-continue').onclick = () => {
    const value = $('wallet-name-entry').value.trim();
    const bytes = new TextEncoder().encode(value);
    if (!value || bytes.length > 64 || /[\u0000-\u001f\u007f]/.test(value)) return status('Wallet name must be 1–64 UTF-8 bytes with no control characters.', true);
    walletName = value;
    const next = nameContinuation;
    nameContinuation = null;
    if (next) next(); else show('onboarding-word-count');
  };
  $('onboarding-12').onclick = () => chooseWordCount(12);
  $('onboarding-24').onclick = () => chooseWordCount(24);
  $('onboarding-no-dice').onclick = () => { diceRolls = []; show('onboarding-touch-choice'); };
  $('onboarding-add-dice').onclick = () => show('onboarding-dice-count');
  const diceTargetContainer = $('onboarding-dice-targets');
  diceTargetContainer.replaceChildren();
  diceTargets.forEach(target => {
    const button = document.createElement('button');
    button.dataset.diceTarget = String(target);
    button.textContent = `${target} Rolls`;
    button.onclick = () => {
      diceTarget = target;
      diceRolls = [];
      updateDiceProgress();
      show('onboarding-dice');
    };
    diceTargetContainer.append(button);
  });
  document.querySelectorAll('[data-die]').forEach(button => {
    button.onclick = () => {
      if (diceRolls.length >= diceTarget) return;
      diceRolls.push(Number(button.dataset.die));
      updateDiceProgress();
      if (diceRolls.length === diceTarget) show('onboarding-touch-choice');
    };
  });
  $('onboarding-dice-undo').onclick = () => { diceRolls.pop(); updateDiceProgress(); };
  $('onboarding-dice-reset').onclick = () => { diceRolls = []; updateDiceProgress(); };
  $('onboarding-no-touch').onclick = () => { touchSamples = []; show('onboarding-passphrase-choice'); };
  $('onboarding-add-touch').onclick = () => {
    touchSamples = [];
    updateTouchProgress();
    show('onboarding-touch');
  };
  $('onboarding-touch-reset').onclick = () => { touchSamples = []; updateTouchProgress(); };
  $('onboarding-touch-pad').onpointermove = event => {
    if (touchSamples.length >= touchTarget) return;
    const rect = $('onboarding-touch-pad').getBoundingClientRect();
    const x = Math.max(0, Math.min(65535, Math.round((event.clientX - rect.left) * 65535 / Math.max(1, rect.width))));
    const y = Math.max(0, Math.min(65535, Math.round((event.clientY - rect.top) * 65535 / Math.max(1, rect.height))));
    const previous = touchSamples[touchSamples.length - 1];
    if (previous && previous.x === x && previous.y === y) return;
    touchSamples.push({ time: Math.trunc(performance.now() * 1000) >>> 0, x, y });
    updateTouchProgress();
    if (touchSamples.length === touchTarget) show('onboarding-passphrase-choice');
  };
  $('onboarding-no-passphrase').onclick = () => { passphrase = ''; finishCreation(); };
  $('onboarding-use-passphrase').onclick = () => show('onboarding-passphrase');
  $('onboarding-passphrase-submit').onclick = () => {
    const value = $('onboarding-passphrase').value;
    if (value !== $('onboarding-passphrase-confirm').value) return status('BIP39 passphrases do not match.', true);
    passphrase = value;
    finishCreation();
  };

  function finishCreationStorage() {
    const next = afterCreationStorage;
    afterCreationStorage = null;
    if (next) next();
    else renderHome(false);
  }

  $('storage-session').onclick = finishCreationStorage;
  $('storage-save').onclick = () => show('storage-protection');
  $('storage-use-password').onclick = () => { credentialType = 'password'; $('storage-credential-title').textContent = 'Save Wallet Password'; show('storage-credential'); };
  $('storage-credential-submit').onclick = async () => {
    try {
      const credential = $('storage-credential').value;
      if (credential !== $('storage-credential-confirm').value) throw new Error('Credentials do not match.');
      if (credentialType !== 'password' || !validCredential('password', credential)) throw new Error('Password must be at least 12 characters and contain a letter and number.');
      const summaries = parse(api('kaskold_vault_wallets')());
      const active = summaries.find(wallet => wallet.active);
      if (!active) throw new Error('No active wallet is available to save.');
      const ciphertext = api('kaskold_vault_portable_backup')(credential);
      await saveEncryptedWallet({
        name: active.name,
        ciphertext,
        fingerprint: active.fingerprint || '',
        kind: active.kind || '',
      }, credentialType);
      $('storage-credential').value = '';
      $('storage-credential-confirm').value = '';
      status('Wallet saved as credential-encrypted local ciphertext. M5 hardware-bound protection is not claimed by Web Vault.');
      await refreshSavedWalletList();
      finishCreationStorage();
    } catch (error) { status(String(error), true); }
  };

  async function unlockSelectedSavedWallet() {
    try {
      const record = await loadEncryptedWallet();
      const wallets = Array.isArray(record?.wallets) ? record.wallets : [];
      if (!wallets.length) throw new Error('No saved encrypted wallet was found.');
      if (!selectedSavedWalletId) throw new Error('Choose a saved wallet before unlocking.');
      const saved = wallets.find(wallet => wallet.id === selectedSavedWalletId);
      if (!saved) throw new Error('The selected saved wallet is no longer available.');
      const credential = $('unlock-saved-credential').value;
      const restored = parse(api('kaskold_vault_restore_portable')(new Uint8Array(saved.ciphertext), credential));
      const name = saved.name || 'Wallet';
      api('kaskold_vault_set_wallet_name')(restored.index, name);
      const summary = parse(api('kaskold_vault_wallets')()).find(wallet => wallet.index === restored.index);
      try {
        await updateEncryptedWalletIdentity(saved.id, {
          name,
          fingerprint: summary?.fingerprint || saved.fingerprint,
          kind: summary?.kind || saved.kind,
        });
      } catch {
        console.warn('Saved-wallet metadata could not be refreshed after unlock.');
      }
      selectedSavedWalletId = null;
      $('unlock-saved-name').textContent = '';
      $('unlock-saved-credential').value = '';
      renderHome(false);
      status(`${name} unlocked.`);
    } catch (error) { status(String(error), true); }
  }
  $('unlock-saved-submit').onclick = unlockSelectedSavedWallet;
  $('unlock-saved-credential').onkeydown = event => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    void unlockSelectedSavedWallet();
  };

  async function openWalletPicker({ fromMenu = false } = {}) {
    $('wallet-picker-back')?.classList.toggle('hidden', !fromMenu);
    await refreshSavedWalletList();
    show('locked');
  }

  refreshSavedWalletList();
  return {
    deleteSavedWallet: async () => {
      await deleteEncryptedWallet();
      await refreshSavedWalletList();
    },
    refreshSavedWalletList,
    openWalletPicker,
    startCreation: (next = null) => startCreation('add', 'choice', next),
    startSeedTool: (mode, next) => startCreation('add', mode, next),
    startRestore,
    applyStagedRestoreName,
  };
}
