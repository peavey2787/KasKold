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

  // Static detail pages live as <template> elements in index.html.
  const detail = (title, templateId, setup) => {
    $('settings-detail-title').textContent = title;
    $('settings-detail-body').replaceChildren($(templateId).content.cloneNode(true));
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

  bindClick('settings-storage', () => detail('Storage', 'settings-detail-storage', () => {
      const confirmation = $('setting-storage-delete-confirmation');
      $('setting-storage-backup').onclick = openPortableBackup;
      $('setting-storage-delete').onclick = () => confirmation.classList.remove('hidden');
      $('setting-storage-delete-cancel').onclick = () => confirmation.classList.add('hidden');
      $('setting-storage-delete-confirm').onclick = async () => {
        try {
          await deleteSavedWallet();
          confirmation.classList.add('hidden');
          status('Saved encrypted browser wallets deleted. The currently unlocked in-memory wallet is unchanged.');
        } catch (error) { status(String(error), true); }
      };
    }));

  const showFirmwareUpdate = () => detail('Firmware Update', 'settings-detail-firmware');

  bindClick('settings-about', () => detail('About', 'settings-detail-about'));

  return { showFirmwareUpdate };
}

export function shouldLockOnPageHide() { return get('pagehide-lock', '1') === '1'; }
