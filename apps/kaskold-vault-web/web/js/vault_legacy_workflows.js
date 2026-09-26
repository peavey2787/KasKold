export function installLegacyWorkflows(ctx) {
  const { $, api, parse, show, status, renderSecretText, openScanner, stopCamera } = ctx;

  const click = id => () => $(id)?.click();
  const route = name => () => show(name);

  // Legacy menu surfaces are compatibility navigation only. Every action below
  // delegates to the same implementation used by the modern Wallet menus.
  $('seed-tools-new').onclick = () => ctx.startSeedTool('new');
  $('seed-tools-dice').onclick = () => ctx.startSeedTool('dice');
  $('seed-tools-touch').onclick = () => ctx.startSeedTool('touch');
  $('seed-tools-import').onclick = route('restore-source');
  $('seed-tools-address').onclick = click('wallet-receive');
  $('seed-tools-bip85').onclick = click('advanced-bip85');

  $('import-menu-sd').onclick = route('wallet-recovery');
  $('import-menu-stego').onclick = route('recovery-stego');
  $('import-menu-raw').onclick = route('recovery-raw');
  $('import-menu-covenant').onclick = route('sd-covenant');

  $('single-sign-tx').onclick = click('begin-scan');
  $('single-sign-message').onclick = click('advanced-sign-message');
  // single-covenant is owned by vault_covenant_workflows.js.
  $('single-commit').onclick = click('advanced-commit-secret');
  $('single-decrypt').onclick = click('advanced-decrypt-secret');

  $('export-choice-seed').onclick = route('seed-backup-menu');
  $('export-choice-watch').onclick = route('watch-only-menu');
  $('export-choice-keys').onclick = route('signing-keys-menu');
  $('export-choice-stego').onclick = click('advanced-stego');

  $('legacy-seed-words').onclick = click('backup-view-words');
  $('legacy-seed-qr').onclick = route('qr-export-menu');
  $('legacy-seed-sd').onclick = click('backup-encrypted-sd');

  $('legacy-kpub-qr').onclick = click('home-connect');
  $('legacy-kpub-file').onclick = () => {
    try {
      const { kpub } = parse(api('kaskold_vault_export_kpub')());
      const blob = new Blob([`${kpub}\n`], { type: 'text/plain;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = 'kaskold-watch-only.kpub';
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      status('Watch-only kpub saved.');
    } catch (error) { status(String(error), true); }
  };
  $('legacy-multisig-kpub').onclick = click('multisig-show-kpub');

  $('legacy-xprv').onclick = route('xprv-export');
  $('legacy-private-key').onclick = click('advanced-export-key');
  $('legacy-qr-compact').onclick = click('advanced-compact-seedqr');
  $('legacy-qr-standard').onclick = click('backup-seedqr');
  $('legacy-qr-plain').onclick = click('advanced-plain-seedqr');

  // Keep the legacy scanner entry useful for exact M5 compatibility.
  ctx.openLegacyTransactionFile = () => {
    openScanner('Sign TX', 'Scan a KSPT/PSKT signing request.', bytes => {
      stopCamera();
      ctx.acceptTransactionFrame(bytes);
    });
  };

  ctx.openLegacyMenus = {
    seedTools: () => show('seed-tools'),
    import: () => show('import-menu'),
    singleSig: () => show('single-sig'),
    exportChoice: () => show('export-choice'),
  };
}
