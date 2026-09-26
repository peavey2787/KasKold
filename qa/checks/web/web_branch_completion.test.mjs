import assert from 'node:assert/strict';
import {
  setupDeepHarness, cleanupDeepHarness, moduleUrl, element, wallet, KSPT, PSKB,
} from './web_runtime_deep_harness.mjs';
import { FakeElement } from './web_recovery_test_harness.mjs';

const { state } = await setupDeepHarness();
try {
  const stubs = globalThis.__COMPANION_WASM_STUBS__;
  stubs.extend_addresses = walletJson => {
    const value = JSON.parse(walletJson);
    value.receive_addresses = [...(value.receive_addresses || []), `kaspa:receive-${(value.receive_addresses || []).length}`];
    value.change_addresses = [...(value.change_addresses || []), `kaspa:change-${(value.change_addresses || []).length}`];
    return JSON.stringify(value);
  };
  const addressState = await import(moduleUrl('features/wallet/core/address_state.js'));
  const navigation = await import(moduleUrl('app/navigation.js'));
  const reset = await import(moduleUrl('features/wallet/state_reset.js'));
  const catchup = await import(moduleUrl('features/stealth/index/scanning/catch_up.js'));
  const countdown = await import(moduleUrl('features/oracle/model_b/controller/proving/countdown.js'));
  const privateWatcher = await import(moduleUrl('features/covenants/private_swap/watcher.js'));
  const privateSwapStateModule = await import(moduleUrl('features/covenants/private_swap/state.js'));
  const { KSTL_SUBNET_HEX } = await import(moduleUrl('features/stealth/index/config.js'));

  // -----------------------------------------------------------------------
  // Change-address reservation / exact-output edge paths.
  // -----------------------------------------------------------------------
  addressState.clearStandardChangeReservations();
  const oneAddressWallet = {
    kpub: wallet.kpub,
    receive_addresses: ['kaspa:receive-0'],
    change_addresses: ['kaspa:change-0'],
    next_receive_index: 0,
    next_change_index: 0,
  };
  state.walletSession.replace(oneAddressWallet);
  state.walletState.fundedReceiveIndices = [];
  state.walletState.usedReceiveIndices = new Set();
  state.walletState.fundedChangeIndices = [];
  state.walletState.usedChangeIndices = new Set();

  assert.equal(addressState.reserveStandardChangeFromSummary('{', {}), null);
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify({ ...oneAddressWallet, next_change_index: -1 }), {}), null);
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify({ ...oneAddressWallet, next_change_index: 3 }), {}), null);
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify(oneAddressWallet), {
    outputs: [{ address: 'kaspa:not-change', derivation_branch: 1, derivation_index: 0, amount_sompi: '1' }],
  }), null);
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify(oneAddressWallet), {
    outputs: [{ address: 'kaspa:change-0', derivation_branch: 1, derivation_index: 0, amount_sompi: 'not-an-int' }],
  }), null);

  const changeSummary = {
    outputs: [{ address: 'kaspa:change-0', derivation_branch: 1, derivation_index: 0, amount_sompi: '1' }],
  };
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify(oneAddressWallet), changeSummary), 0);
  assert.equal(addressState.standardChangeReservationMatchesSummary(-1, changeSummary), false);
  assert.equal(addressState.standardChangeReservationMatchesSummary(0, {}), false);
  assert.equal(addressState.markStandardChangeBroadcast(0), true);
  assert.equal(addressState.releasePendingStandardChange(0), false);
  assert.equal(addressState.releasePendingStandardChange(-1), false);
  assert.equal(addressState.releasePendingStandardChange(999), false);
  addressState.clearStandardChangeReservations();
  assert.equal(addressState.reserveStandardChangeFromSummary(JSON.stringify(oneAddressWallet), changeSummary), 0);
  assert.equal(addressState.releasePendingStandardChange(0), true);

  // Null/undefined observation arrays are accepted as empty collections.
  state.walletState.fundedChangeIndices = null;
  state.walletState.usedChangeIndices = null;
  addressState.reconcileStandardChangeReservations();
  state.walletState.fundedReceiveIndices = null;
  state.walletState.usedReceiveIndices = null;
  assert.equal(addressState.expandAddressesIfNeeded(), false);

  // When every visible address is already observed and the runtime cannot grow
  // the pool further, selection deterministically falls back to the last index.
  state.walletState.fundedReceiveIndices = [0];
  state.walletState.usedReceiveIndices = null;
  state.walletState.fundedChangeIndices = [0];
  state.walletState.usedChangeIndices = null;
  const savedExtend = stubs.extend_addresses;
  stubs.extend_addresses = walletJson => walletJson;
  const exhausted = JSON.parse(addressState.walletWithFreshIndices());
  assert.equal(exhausted.next_receive_index, 0);
  assert.equal(exhausted.next_change_index, 0);
  stubs.extend_addresses = savedExtend;
  state.walletSession.clear();
  assert.equal(addressState.getNextReceiveIndex(), 0);
  assert.equal(addressState.walletWithFreshIndices(), '');
  state.walletSession.replace(oneAddressWallet);

  // -----------------------------------------------------------------------
  // Navigation/history and transient cleanup branches.
  // -----------------------------------------------------------------------
  state.walletSession.replace(oneAddressWallet);
  state.navigationState.currentScreenName = '';
  navigation.showScreen('dashboard');
  state.navigationState.screenHistory.splice(0, state.navigationState.screenHistory.length, ...Array.from({ length: 65 }, (_, i) => `old-${i}`));
  state.navigationState.currentScreenName = 'welcome';
  navigation.showScreen('dashboard');
  assert.ok(state.navigationState.screenHistory.length <= 64);

  const originalGetElementById = document.getElementById;
  document.getElementById = id => id === 'screen-definitely-missing' || id === 'companion-startup-status'
    ? null
    : originalGetElementById.call(document, id);
  assert.equal(navigation.showScreen('definitely-missing'), false);
  assert.equal(navigation.setStartupStatus('hidden', 'ready'), undefined);
  document.getElementById = originalGetElementById;
  state.walletSession.clear();
  assert.equal(navigation.navigateHome(), true);
  state.walletSession.replace(oneAddressWallet);

  // `clearTransientDom` has distinct missing-screen, checkbox/radio, and text
  // input branches. Exercise them through the public hardened cleanup entrypoint.
  const sendScreen = element('screen-send');
  const checkbox = new FakeElement('input'); checkbox.type = 'checkbox'; checkbox.checked = true;
  const radio = new FakeElement('input'); radio.type = 'radio'; radio.checked = true;
  const text = new FakeElement('input'); text.type = 'text'; text.value = 'secret-ish transient text';
  sendScreen.querySelectorAll = selector => selector === 'input, textarea' ? [checkbox, radio, text] : [];
  document.getElementById = id => id === 'screen-qr-display' ? null : originalGetElementById.call(document, id);
  globalThis.cancelAnimationFrame = () => {};
  state.scannerState.scanAnimFrame = 123;
  stubs.reset_qr_decoder = () => {};
  reset.hardenedWalletCleanup();
  assert.equal(checkbox.checked, false);
  assert.equal(radio.checked, false);
  assert.equal(text.value, '');
  assert.equal(state.scannerState.scanAnimFrame, null);
  document.getElementById = originalGetElementById;
  delete globalThis.cancelAnimationFrame;
  state.walletSession.replace(oneAddressWallet);

  // Every best-effort service stop must remain fail-closed even if a host timer
  // primitive or device-backed scanner close operation throws. Force each
  // independently wrapped cleanup branch to throw and verify cleanup continues.
  state.uiState.autoRefreshTimer = null;
  state.scannerState.qrCycleTimer = null;
  state.stealthState._stealthQrTimer = 101;
  state.covenantWatcherState._covWatcherTimer = 102;
  state.covenantWatcherState._covActiveWatcherTimer = 103;
  state.oracleState._oracleMbAgeTimer = 104;
  state.oracleState._oracleMbPollTimer = 105;
  state.crowdfundState.watcherTimer = 106;
  countdown.startOracleMbCountdown();
  Object.assign(privateSwapStateModule.privateSwapState, {
    role: 'bob', myAddress: 'kaspa:private-swap-watch', myAmountSompi: '1', counterCompletedSignature: '',
  });
  privateWatcher.startPrivateSwapWatcher(() => {});
  stubs.reset_qr_decoder = () => { throw new Error('decoder stop failed'); };
  const originalClearInterval = globalThis.clearInterval;
  globalThis.clearInterval = () => { throw new Error('timer stop failed'); };
  assert.doesNotThrow(() => reset.hardenedWalletCleanup());
  globalThis.clearInterval = originalClearInterval;
  stubs.reset_qr_decoder = () => '';
  state.walletSession.replace(oneAddressWallet);

  // -----------------------------------------------------------------------
  // Stealth REST catch-up retry/fallback branches.
  // -----------------------------------------------------------------------
  const originalFetch = globalThis.fetch;
  const originalGetForStealth = document.getElementById;

  // Sink fallback with a zero blue score must fail closed.
  globalThis.fetch = async url => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":0}'; } };
    if (u.endsWith('/info/blockdag')) return { async json() { return { sink: 'sink' }; } };
    if (u.endsWith('/blocks/sink')) return { async text() { return '{"blueScore":0}'; } };
    throw new Error(`unexpected ${u}`);
  };
  await assert.rejects(() => catchup.stealthRestCatchUp('https://runtime-api'), /no blueScore/);

  // Large tip covers lookback subtraction, 100-wide windows, batch sleeping,
  // missing subnetwork/payload, 0x payload normalization, zero-R rejection,
  // malformed payload rejection, dedupe, and UI-update catch blocks.
  const R = '12'.repeat(32);
  let searchCalls = 0;
  document.getElementById = id => {
    if (id === 'stealth-scan-status' || id === 'stealth-r-list') throw new Error('headless status node');
    return originalGetForStealth.call(document, id);
  };
  globalThis.fetch = async (url) => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":9105}'; } };
    if (u.includes('/transactions/search')) {
      searchCalls += 1;
      return {
        ok: true, status: 200, headers: { get() { return null; } },
        async json() {
          return [
            {},
            { subnetwork_id: KSTL_SUBNET_HEX },
            { subnetwork_id: KSTL_SUBNET_HEX, payload: '00' },
            { subnetwork_id: KSTL_SUBNET_HEX, payload: '0x01' + '00'.repeat(32) + '01' },
            { subnetwork_id: KSTL_SUBNET_HEX, payload: '0x01' + R + '01' },
            { subnetwork_id: KSTL_SUBNET_HEX, payload: '01' + R + '01' },
          ];
        },
      };
    }
    throw new Error(`unexpected ${u}`);
  };
  const caughtUp = await catchup.stealthRestCatchUp('https://runtime-api');
  assert.deepEqual(caughtUp, [R]);
  assert.ok(searchCalls > 3);

  // Throttle retry with Retry-After, terminal throttle, ordinary HTTP error,
  // and thrown transport errors all return safely without accepting data.
  document.getElementById = originalGetForStealth;
  let attempts = 0;
  globalThis.fetch = async url => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":1}'; } };
    if (u.includes('/transactions/search')) {
      attempts += 1;
      if (attempts === 1) return { ok:false, status:429, headers:{ get(){ return '1'; } }, async json(){ return []; } };
      return { ok:true, status:200, headers:{ get(){ return null; } }, async json(){ return []; } };
    }
    throw new Error(`unexpected ${u}`);
  };
  assert.deepEqual(await catchup.stealthRestCatchUp('https://runtime-api'), []);
  assert.equal(attempts, 2);

  globalThis.fetch = async url => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":1}'; } };
    if (u.includes('/transactions/search')) return { ok:false, status:503, headers:{ get(){ return '0'; } }, async json(){ return []; } };
    throw new Error(`unexpected ${u}`);
  };
  assert.deepEqual(await catchup.stealthRestCatchUp('https://runtime-api'), []);

  globalThis.fetch = async url => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":1}'; } };
    if (u.includes('/transactions/search')) return { ok:false, status:500, headers:{ get(){ return null; } }, async json(){ return []; } };
    throw new Error(`unexpected ${u}`);
  };
  assert.deepEqual(await catchup.stealthRestCatchUp('https://runtime-api'), []);

  globalThis.fetch = async url => {
    const u = String(url);
    if (u.includes('virtual-chain-blue-score')) return { async text() { return '{"blueScore":1}'; } };
    if (u.includes('/transactions/search')) throw new Error('transport down');
    throw new Error(`unexpected ${u}`);
  };
  assert.deepEqual(await catchup.stealthRestCatchUp('https://runtime-api'), []);

  globalThis.fetch = originalFetch;
  document.getElementById = originalGetForStealth;

  console.log('PASS: remaining wallet/storage/navigation/stealth branch completion');
} finally {
  await cleanupDeepHarness();
}
