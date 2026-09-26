const preferences = new Map([
  ['auto-lock', '5'],
  ['pagehide-lock', '1'],
]);
let autoLockTimer = null;
let lockCallback = null;
let sessionNotBefore = '';
let sessionWeekly = '';

function get(name, fallback) {
  return preferences.get(name) ?? fallback;
}

function armAutoLock() {
  clearTimeout(autoLockTimer);
  autoLockTimer = null;
  const minutes = Number(get('auto-lock', '5'));
  if (!lockCallback || !Number.isFinite(minutes) || minutes <= 0) return;
  autoLockTimer = setTimeout(() => lockCallback('Vault auto-locked after inactivity.'), minutes * 60_000);
}

export function touchSecurityTimer() { armAutoLock(); }

export function installSettings({ $, api, show, status, lockVault, openPortableBackup, deleteSavedWallet }) {
  lockCallback = message => {
    lockVault();
    status(message);
  };
  armAutoLock();
  addEventListener('pointerdown', armAutoLock, { passive: true });
  addEventListener('keydown', armAutoLock, { passive: true });

  const bindClick = (id, handler) => {
    const node = $(id);
    if (!node) {
      console.warn(`Vault Settings control is unavailable: #${id}`);
      return;
    }
    node.onclick = handler;
  };

  const detail = (title, html, setup) => {
    $('settings-detail-title').textContent = title;
    $('settings-detail-body').innerHTML = html;
    show('settings-detail');
    setup?.();
  };

  bindClick('settings-security', () => show('security'));
  bindClick('security-duress', () => show('security-duress'));
  bindClick('security-no-sign', () => { $('security-no-sign-value').value = sessionNotBefore; show('security-no-sign'); });
  bindClick('security-weekly', () => { $('security-weekly-value').value = sessionWeekly; show('security-weekly'); });

  function applySessionPolicy(nextNotBefore, nextWeekly) {
    api('kaskold_vault_set_session_signing_policy')(nextNotBefore, nextWeekly);
    sessionNotBefore = nextNotBefore;
    sessionWeekly = nextWeekly;
    status('Session signing policy validated and applied inside the Rust Vault boundary.');
  }
  bindClick('security-no-sign-save', () => {
    try {
      const value = $('security-no-sign-value').value.trim();
      if (!/^\d{12}$/.test(value)) throw new Error('No-sign-before must use YYYYMMDDHHMM UTC.');
      applySessionPolicy(value, sessionWeekly);
    } catch (error) { status(String(error), true); }
  });
  bindClick('security-no-sign-clear', () => {
    try { applySessionPolicy('', sessionWeekly); $('security-no-sign-value').value = ''; }
    catch (error) { status(String(error), true); }
  });
  bindClick('security-weekly-save', () => {
    try {
      const value = $('security-weekly-value').value.trim();
      if (!value) throw new Error('Enter at least one weekly UTC signing window.');
      applySessionPolicy(sessionNotBefore, value);
    } catch (error) { status(String(error), true); }
  });
  bindClick('security-weekly-clear', () => {
    try { applySessionPolicy(sessionNotBefore, ''); $('security-weekly-value').value = ''; }
    catch (error) { status(String(error), true); }
  });

  bindClick('settings-storage', () => detail('Storage', `
    <p class="muted">Web Vault can save only authenticated password-encrypted KHV3 ciphertext in browser storage. Unlocked private material and credentials are never persisted. Portable encrypted backups use the same Rust custody/KDF boundary.</p>
    <div class="stack"><button id="setting-storage-backup" class="primary">Encrypted Backup</button><button id="setting-storage-delete" class="danger">Delete Saved Browser Wallets</button></div>`, () => {
      $('setting-storage-backup').onclick = openPortableBackup;
      $('setting-storage-delete').onclick = async () => {
        try {
          await deleteSavedWallet();
          status('Saved encrypted browser wallets deleted. The currently unlocked in-memory wallet is unchanged.');
        } catch (error) { status(String(error), true); }
      };
    }));

  const showFirmwareUpdate = () => detail('Firmware Update', `
    <p><strong>READY TO UPDATE</strong></p>
    <p>1. Connect the M5 signer to the computer over USB-C.</p>
    <p>2. From the KasKold repository run <code>make flash BOARD=m5stack</code>.</p>
    <p>3. Keep USB connected until flashing completes.</p>
    <p class="muted">The signer verifies the installed firmware after reboot. This Web Vault remains network-isolated and never flashes hardware from the browser.</p>`);

  bindClick('settings-about', () => detail('About', `
    <p><strong>KasKold Vault</strong></p>
    <p class="muted">Network-free signing shell. Private-key derivation, backup cryptography, transaction review/signing, message signing, multisig construction, and recovery execute in the shared Rust Vault runtime. Companion remains watch-only.</p>
    <p class="muted">Web removable-storage workflows use user-selected local files; portable encrypted backups use the shared KasKold Argon2id policy and authenticated KHV3 wallet seal.</p>`));

  return { showFirmwareUpdate };
}

export function shouldLockOnPageHide() { return get('pagehide-lock', '1') === '1'; }
