import { downloadBytes, nonNegativeInteger } from '../vault_bytes.js';
import {
  deleteMultisigDescriptor,
  listMultisigDescriptors,
  saveMultisigDescriptor,
} from '../vault_storage.js';

export function installMultisigWorkflows(ctx) {
  const { $, api, parse, show, status, openScanner, stopCamera } = ctx;
  let latestMultisigResult = null;
  let latestMultisigMetadata = null;

  function activeOwnerKpub() {
    const result = parse(api('kaskold_vault_multisig_kpub')());
    if (!result?.kpub) throw new Error('The active wallet cannot manage multisig descriptors.');
    return result.kpub;
  }

  async function persistDescriptor(result, metadata = {}) {
    if (!result?.descriptor) throw new Error('No multisig descriptor is available.');
    await saveMultisigDescriptor(activeOwnerKpub(), {
      descriptor: result.descriptor,
      address: result.address || '',
      network: metadata.network || 'mainnet',
      chain: metadata.chain || 0,
      index: metadata.index || 0,
    });
  }

  function renderMultisigResult(result, metadata = null, askToSave = false) {
    latestMultisigResult = result;
    latestMultisigMetadata = metadata;
    $('multisig-result-address').textContent = result.address;
    $('multisig-result-address-qr').innerHTML = result.addressSvg;
    $('multisig-result-descriptor').textContent = result.descriptor;
    $('multisig-result-descriptor-qr').innerHTML = result.descriptorSvg;
    $('multisig-descriptor-save-prompt').classList.toggle('hidden', !askToSave);
    show('multisig-result');
  }

  function stageMultisigDescriptor(descriptor) {
    $('multisig-import-descriptor').value = descriptor;
    show('multisig');
    status('Multisig descriptor scanned. Review network, chain, and index before importing.');
  }

  function descriptorSummary(record) {
    const address = record.address ? `\n${record.address}` : '';
    return `${record.descriptor}${address}`;
  }

  async function renderDescriptorList(mode) {
    const list = $('multisig-descriptor-list');
    list.replaceChildren();
    const owner = activeOwnerKpub();
    const records = await listMultisigDescriptors(owner);
    if (!records.length) {
      const empty = document.createElement('p');
      empty.className = 'muted';
      empty.textContent = 'No saved descriptors belong to the active wallet.';
      list.append(empty);
      show('multisig-descriptors');
      return;
    }
    for (const record of records) {
      const row = document.createElement('div');
      row.className = 'wallet-row';
      const value = document.createElement('div');
      value.className = 'mono break selectable';
      value.textContent = descriptorSummary(record);
      row.append(value);
      if (mode === 'backup') {
        const button = document.createElement('button');
        button.textContent = 'Backup';
        button.onclick = () => downloadBytes(
          new TextEncoder().encode(`${record.descriptor}\n`),
          'kaskold-multisig-descriptor.txt',
          'text/plain',
        );
        row.append(button);
      } else if (mode === 'delete') {
        const button = document.createElement('button');
        button.textContent = 'Delete';
        button.className = 'danger';
        button.onclick = async () => {
          try {
            await deleteMultisigDescriptor(owner, record.descriptor);
            await renderDescriptorList('delete');
            status('Multisig descriptor deleted.');
          } catch (error) { status(String(error), true); }
        };
        row.append(button);
      }
      list.append(row);
    }
    show('multisig-descriptors');
  }

  $('wallet-multisig').onclick = () => show('multisig');
  $('multisig-descriptors-open').onclick = () => {
    renderDescriptorList('view').catch(error => status(String(error), true));
  };
  $('multisig-descriptors-view').onclick = () => {
    renderDescriptorList('view').catch(error => status(String(error), true));
  };
  $('multisig-descriptors-backup').onclick = () => {
    renderDescriptorList('backup').catch(error => status(String(error), true));
  };
  $('multisig-descriptors-restore').onclick = () => {
    ctx.multisigDescriptorRestorePending = true;
    show('sd-multisig-descriptor');
  };
  $('multisig-descriptors-delete').onclick = () => {
    renderDescriptorList('delete').catch(error => status(String(error), true));
  };
  $('multisig-show-kpub').onclick = () => {
    try {
      const result = parse(api('kaskold_vault_multisig_kpub')());
      $('multisig-kpub').textContent = result.kpub;
      $('multisig-kpub-qr').innerHTML = result.svg;
      $('multisig-kpub-panel').classList.remove('hidden');
    } catch (error) { status(String(error), true); }
  };
  $('multisig-create-open').onclick = () => show('multisig-create');
  $('multisig-cosigner-scan').onclick = () => {
    openScanner('Scan Multisig kpub', 'Scan another participant\'s dedicated multisig kpub.', bytes => {
      try {
        stopCamera();
        const text = new TextDecoder().decode(bytes).trim();
        if (!text) throw new Error('Scanned cosigner kpub is empty.');
        const existing = $('multisig-cosigners').value.trim();
        $('multisig-cosigners').value = existing ? `${existing}\n${text}` : text;
        show('multisig-create');
        status('Cosigner kpub added.');
      } catch (error) { status(String(error), true); }
    });
  };
  $('multisig-create-submit').onclick = () => {
    try {
      const threshold = nonNegativeInteger($('multisig-threshold').value, 'Threshold', 5);
      const chain = Number($('multisig-chain').value);
      const index = nonNegativeInteger($('multisig-index').value, 'Address index');
      const network = $('multisig-network').value;
      const result = parse(api('kaskold_vault_create_multisig')(
        threshold,
        $('multisig-cosigners').value,
        network,
        chain,
        index,
      ));
      renderMultisigResult(result, { network, chain, index }, true);
    } catch (error) { status(String(error), true); }
  };
  $('multisig-import-scan').onclick = () => {
    openScanner('Scan Multisig Descriptor', 'Scan a multi_hd45, multi_hd, or multi descriptor.', bytes => {
      try {
        stopCamera();
        const descriptor = new TextDecoder().decode(bytes).trim();
        if (!descriptor) throw new Error('Scanned descriptor is empty.');
        stageMultisigDescriptor(descriptor);
      } catch (error) { status(String(error), true); }
    });
  };
  $('multisig-import-submit').onclick = () => {
    try {
      const descriptor = $('multisig-import-descriptor').value.trim();
      if (!descriptor) throw new Error('Enter a multisig descriptor.');
      const network = $('multisig-import-network').value;
      const chain = Number($('multisig-import-chain').value);
      const index = nonNegativeInteger($('multisig-import-index').value, 'Address index');
      const result = parse(api('kaskold_vault_import_multisig')(
        descriptor,
        network,
        chain,
        index,
      ));
      renderMultisigResult(result, { network, chain, index }, true);
    } catch (error) { status(String(error), true); }
  };
  $('multisig-store-descriptor').onclick = async () => {
    try {
      await persistDescriptor(latestMultisigResult, latestMultisigMetadata || {});
      $('multisig-descriptor-save-prompt').classList.add('hidden');
      status('Descriptor saved for the active wallet.');
    } catch (error) { status(String(error), true); }
  };
  $('multisig-skip-descriptor').onclick = () => {
    $('multisig-descriptor-save-prompt').classList.add('hidden');
    status('Descriptor was not saved to browser storage.');
  };
  $('multisig-save-address').onclick = () => {
    try {
      if (!latestMultisigResult?.address) throw new Error('No multisig address is available.');
      downloadBytes(new TextEncoder().encode(`${latestMultisigResult.address}\n`), 'kaskold-multisig-address.txt', 'text/plain');
      status('Multisig address saved.');
    } catch (error) { status(String(error), true); }
  };
  $('multisig-save-descriptor').onclick = () => {
    try {
      if (!latestMultisigResult?.descriptor) throw new Error('No multisig descriptor is available.');
      downloadBytes(new TextEncoder().encode(`${latestMultisigResult.descriptor}\n`), 'kaskold-multisig-descriptor.txt', 'text/plain');
      status('Multisig descriptor backup created.');
    } catch (error) { status(String(error), true); }
  };

  ctx.renderMultisigResult = renderMultisigResult;
  ctx.stageMultisigDescriptor = stageMultisigDescriptor;
  ctx.persistMultisigDescriptor = persistDescriptor;
}
