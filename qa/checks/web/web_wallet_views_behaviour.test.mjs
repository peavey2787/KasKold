import assert from 'node:assert/strict';
import {
  setupDeepHarness, cleanupDeepHarness, moduleUrl, element, setConfirmResult, setFetchHook,
  wallet, ADDRESS, tick,
} from './web_runtime_deep_harness.mjs';

function response(json) {
  return { ok: true, status: 200, headers: { get() { return null; } }, async json() { return json; }, async text() { return JSON.stringify(json); } };
}

function historyTransactions(count) {
  return Array.from({ length: count }, (_, index) => ({
    transaction_id: index.toString(16).padStart(64, '0'),
    block_time: 1_700_000_000_000 + index,
    is_accepted: true,
    inputs: [{ previous_outpoint_amount: '1000', previous_outpoint_address: 'kaspa:sender' }],
    outputs: [{ amount: '900', script_public_key_address: ADDRESS }],
  }));
}

await setupDeepHarness();
try {
  const state = await import(moduleUrl('app/state/index.js'));
  const receive = await import(moduleUrl('features/transactions/send/receive.js'));
  const tools = await import(moduleUrl('features/wallet/tools.js'));
  const assets = await import(moduleUrl('features/assets/render.js'));
  const events = await import(moduleUrl('app/events/wallet/settings_and_wallet.js'));
  const kpub = await import(moduleUrl('features/wallet/kpub_manager/index.js'));
  const stubs = globalThis.__COMPANION_WASM_STUBS__;

  // Receive address: three balanced lines, longest first, joining back to the input.
  for (let length = 0; length <= 70; length += 1) {
    const address = 'k'.repeat(length);
    const lines = receive.splitReceiveAddressLines(address);
    assert.equal(lines.length, 3);
    assert.equal(lines.join(''), address);
    const sizes = lines.map(line => line.length);
    assert.ok(Math.max(...sizes) - Math.min(...sizes) <= 1, `length ${length}: ${sizes}`);
    assert.deepEqual([...sizes].sort((a, b) => b - a), sizes);
  }
  assert.deepEqual(receive.splitReceiveAddressLines('abcdefgh'), ['abc', 'def', 'gh']);

  // Rendering uses one span per line, and copy takes the raw address, not the wrapped text.
  // Index 0 is already used, so Receive shows the next fresh address.
  const fresh = wallet.receive_addresses[1];
  receive.showReceive();
  const display = element('receive-address');
  assert.equal(display.dataset.address, fresh);
  assert.deepEqual(display.children.map(row => row.className), ['address-line', 'address-line', 'address-line']);
  assert.equal(display.children.map(row => row.textContent).join(''), fresh);
  display.textContent = 'stale wrapped\ntext';
  const copied = [];
  navigator.clipboard.writeText = async value => { copied.push(value); };
  receive.copyAddress();
  await tick();
  assert.deepEqual(copied, [fresh]);

  // Verify Address: the QR is an <img> of the SVG, never injected markup.
  tools.showAddresses();
  const items = element('address-list').children.filter(row => row.classList.contains('addr-item') && row.dataset.addr.endsWith('-r'));
  assert.equal(items.length, wallet.receive_addresses.length);
  items[1].onclick();
  assert.equal(element('verify-address').textContent, wallet.receive_addresses[1]);
  assert.equal(element('verify-path').textContent, "m/44'/111111'/0'/0/1");
  const qr = element('verify-qr');
  assert.equal(qr.children.length, 1);
  const [image] = qr.children;
  assert.equal(image.tagName, 'IMG');
  assert.equal(image.className, 'qr-svg-image');
  assert.equal(image.alt, 'Address QR code');
  assert.equal(image.src, `data:image/svg+xml;charset=utf-8,${encodeURIComponent('<svg>1</svg>')}`);
  assert.equal(qr.innerHTML, '');
  const frames = stubs.generate_qr_frames;
  stubs.generate_qr_frames = () => JSON.stringify([{ svg: '' }]);
  items[0].onclick();
  assert.equal(qr.children.length, 0, 'an empty QR frame renders nothing');
  stubs.generate_qr_frames = frames;

  // Copy on Verify copies the shown address and stays on the Verify screen.
  events.bindSettingsAndWalletEvents();
  items[1].onclick();
  const screenBeforeCopy = state.navigationState.currentScreenName;
  copied.length = 0;
  await element('btn-verify-copy').onclick();
  assert.deepEqual(copied, [wallet.receive_addresses[1]]);
  assert.equal(element('toast').textContent, 'Address copied');
  assert.equal(state.navigationState.currentScreenName, screenBeforeCopy);
  assert.equal(screenBeforeCopy, 'verify');

  // History hides the redundant bottom Back/Home pair at four entries or fewer.
  for (const [count, compact] of [[0, true], [4, true], [5, false]]) {
    setFetchHook(async url => response(String(url).includes(`/addresses/${ADDRESS}/full-transactions`) ? historyTransactions(count) : []));
    state.walletState.historyEntries = [];
    tools.showHistory();
    await tick(); await tick();
    assert.equal(state.walletState.historyEntries.length, count);
    assert.equal(element('screen-history').classList.contains('history-compact'), compact, `${count} entries`);
  }
  setConfirmResult(true);
  tools.clearHistory();
  assert.equal(element('screen-history').classList.contains('history-compact'), true);

  // Assets apply the same rule across tokens, NFTs and domains together.
  const token = { balance: '100', decimals: 0 };
  for (const [inventory, compact] of [
    [{ tokens: new Map(), nfts: [], domains: [] }, true],
    [{ tokens: new Map([['A', token], ['B', token]]), nfts: [{ tick: 'N', tokenId: 1 }], domains: ['d.kas'] }, true],
    [{ tokens: new Map([['A', token], ['B', token]]), nfts: [{ tick: 'N', tokenId: 1 }], domains: ['d.kas', 'e.kas'] }, false],
  ]) {
    assets.renderWalletAssets(inventory);
    assert.equal(element('screen-tokens').classList.contains('tokens-compact'), compact);
  }

  // Welcome's add-wallet button opens Manage Wallets straight into the import form.
  const coreEvents = await import(moduleUrl('app/events/system/core.js'));
  coreEvents.bindCoreEvents();
  element('kpub-import-form').classList.add('hidden');
  element('btn-scan-kpub').onclick();
  assert.equal(state.navigationState.currentScreenName, 'kpub-manager');
  assert.equal(state.navigationState.kpubManagerReturnScreen, 'welcome');
  assert.equal(element('kpub-import-form').classList.contains('hidden'), false);

  // The startup status line only occupies space while it has something to say.
  const navigation = await import(moduleUrl('app/navigation.js'));
  navigation.setStartupStatus('Loaded saved wallet', 'ok');
  assert.equal(element('companion-startup-status').classList.contains('hidden'), false);
  navigation.setStartupStatus('   ');
  assert.equal(element('companion-startup-status').textContent, '');
  assert.equal(element('companion-startup-status').classList.contains('hidden'), true);

  // A saved kpub that fails validation leaves the current wallet loaded:
  // validation runs before the hardened cleanup that would clear it.
  const current = state.walletSession.current();
  const broken = kpub.kpubRepository.save({ name: 'Broken', kpub: 'kpub1:broken', network: 'mainnet' });
  const importKpub = stubs.import_kpub;
  stubs.import_kpub = () => { throw new Error('invalid kpub'); };
  setConfirmResult(true);
  assert.equal(kpub.loadSavedKpub(broken.id), false);
  assert.deepEqual(state.walletSession.current(), current);
  stubs.import_kpub = importKpub;
  kpub.kpubRepository.remove(broken.id);
} finally {
  await cleanupDeepHarness();
}
