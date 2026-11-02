import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { setupHarness, teardownHarness, moduleUrl, element } from '../support/web_recovery_test_harness.mjs';

const ROOT = path.resolve('.');
const JS_ROOT = path.join(ROOT, 'apps/kaskold-companion-web/web/js');
const ADDRESS = 'kaspa:qz0runtimeboundaryfixture000000000000000000000000000000000000000000000000';
const HEX32 = '11'.repeat(32);
const HEX64 = '22'.repeat(64);
const TXID = 'aa'.repeat(32);
const PSKB = '50534b42';
const KSPT = '4b5350540401';
const wallet = { kpub: 'kpub-test', receive_addresses: [ADDRESS], change_addresses: [ADDRESS], next_receive_index: 0, next_change_index: 0 };
const utxo = { tx_id: TXID, transactionId: TXID, index: 0, amount: '250000000', block_daa_score: '900', covenant_id: HEX32, script_public_key: '000051' };
const covenant = { type: 'escrow', address: ADDRESS, redeem_script_hex: '51', covenant_id_hex: HEX32, locktime_daa: '800', cooldown_daa: '0', max_withdraw_sompi: '100000000', threshold_sompi: '100000000', alice_pk: HEX32, bob_pk: HEX32 };

function exportedParams(fn) {
  const src = Function.prototype.toString.call(fn);
  const m = src.match(/^[^(]*\(([^)]*)\)/) || src.match(/^\s*([^=()]+?)\s*=>/);
  if (!m) return [];
  return m[1].split(',').map(x => x.trim().replace(/=.*$/, '').replace(/^\.\.\./, '')).filter(Boolean);
}
// Ordered rules over the lowercased param name; the first matching pattern supplies the argument.
const ARG_RULES = [
  [/event|evt|e$/, () => ({ target: element('runtime-boundary-event'), key: 'Enter', preventDefault() {}, stopPropagation() {}, dataTransfer: { files: [] } })],
  [/wallet|session/, (_n, variant) => (variant ? JSON.stringify(wallet) : wallet)],
  [/utxo|entry|input/, (n, variant) => (/json/.test(n) ? JSON.stringify([utxo]) : variant ? [utxo] : utxo)],
  [/covenant|record|result|contract/, () => covenant],
  [/address|addr|dest|recipient|source/, () => ADDRESS],
  [/pskb|pskt/, () => PSKB],
  [/kspt|signed/, () => KSPT],
  [/script|redeem|payload|preimage|commitment|proof/, (_n, variant) => (variant ? '51' : HEX32)],
  [/pub|key|hash|txid|tx_id|id_hex|secret/, () => HEX32],
  [/signature|sig/, () => HEX64],
  [/network/, (_n, variant) => (variant ? 'testnet-10' : 'mainnet')],
  [/branch/, (_n, variant) => (variant ? 'buyer-release' : 'owner')],
  [/type|kind/, (_n, variant) => (variant ? 'escrow' : 'savings')],
  [/json|properties|metadata|context|request|options|params|state/, (_n, variant) => (variant ? JSON.stringify({}) : {})],
  [/amount|fee|sompi|daa|score|limit|count|index|idx|time|lock|threshold|goal|value|mass|rate/, (_n, variant) => (variant ? 1n : '1')],
  [/bytes|data|frame|raw/, (_n, variant) => (variant ? new Uint8Array([1, 2, 3]) : '010203')],
  [/enabled|active|partial|manual|force|flag/, (_n, variant) => variant === 0],
];
function argFor(name, variant = 0) {
  const n = name.toLowerCase();
  const rule = ARG_RULES.find(([pattern]) => pattern.test(n));
  if (rule) return rule[1](n, variant);
  if (/callback|handler|fn|on[A-Z_]/.test(name)) return () => {};
  return variant ? '1' : {};
}
function privateMaterialPresent() {
  const dump = `${localStorage.getItem('companion_private_swap_v2') ?? ''}${sessionStorage.getItem('companion_private_swap_v2') ?? ''}`;
  return /(?:mnemonic|xprv|private[_-]?key|secret[_-]?key|mySecretKey|_swap_secret_key)/i.test(dump);
}

await setupHarness();
const originalGetElementById = document.getElementById.bind(document);
function validValueForId(id) {
  if (/addr|address|dest|recipient/i.test(id)) return ADDRESS;
  if (/script|redeem/i.test(id)) return '51';
  if (/pub|key/i.test(id)) return HEX32;
  if (/txid|hash|commit|preimage|secret/i.test(id)) return HEX32;
  if (/amount|fee|limit|cap|threshold|goal|target|product|price|kas/i.test(id)) return '1';
  if (/daa|lock|delay|duration|timeout|deadline|cltv|period|seq|start/i.test(id)) return '5000';
  if (/count|min-input|min-output|depth|index/i.test(id)) return '2';
  if (/name|label|campaign/i.test(id)) return 'Runtime boundary';
  if (/json|payload|properties/i.test(id)) return '{}';
  if (/mode/i.test(id)) return 'spend';
  if (/network/i.test(id)) return 'mainnet';
  return '';
}
document.getElementById = id => {
  const node = originalGetElementById(id);
  if (!node.__runtimeFilled) {
    node.__runtimeFilled = true;
    node.value = validValueForId(id);
    if (id === 'cov-owner-panel') node.dataset.covOwnerType = 'savings';
    if (id === 'cov-beneficiary-panel') node.dataset.covBeneType = 'timelocked-savings';
    if (/hash-display/.test(id)) node.textContent = 'BLAKE2B: ' + HEX32;
    if (/checkbox|toggle|enable|manual/i.test(id)) node.checked = true;
  }
  return node;
};
class BoundaryWebSocket {
  static OPEN = 1;
  constructor(url) { this.url = url; this.readyState = 1; queueMicrotask(() => this.onopen?.({})); }
  send() { this.onmessage?.({ data: new ArrayBuffer(0) }); }
  close() { this.readyState = 3; this.onclose?.({}); }
  addEventListener(type, fn) { if (type === 'open') queueMicrotask(() => fn({})); }
  removeEventListener() {}
}
globalThis.WebSocket = BoundaryWebSocket;
const unhandled = [];
const onUnhandled = reason => unhandled.push(reason);
process.on('unhandledRejection', onUnhandled);
try {
  const state = await import(moduleUrl('app/state/index.js'));
  state.walletSession.replace(wallet);
  state.networkState.network = 'mainnet';
  state.networkState.customNodeUrl = 'wss://runtime-boundary';
  state.networkState.lastFeeEstimate = { normal_sompi_per_gram: '1', low_sompi_per_gram: '1', priority_sompi_per_gram: '2' };
  state.networkState.cachedUtxos = [utxo];
  state.covenantState.lastCovenantResult = covenant;
  state.stealthState.stealthAnnouncementsR = [HEX32];
  state.stealthState._stealthResults = [];
  Object.assign(globalThis.__COMPANION_WASM_STUBS__, {
    fetch_utxos_for_address_js: () => JSON.stringify([utxo]), fetch_utxos: () => JSON.stringify([utxo]), fetch_utxos_complete: () => JSON.stringify([utxo]),
    fetch_balance: () => JSON.stringify({ total_kas: 2.5, total_sompi: 250000000, utxo_count: 1, funded_addresses: 1, funded_receive_indices: [0], funded_change_indices: [] }),
    get_virtual_daa_score: () => '1000', get_fee_estimate: () => JSON.stringify({ suggested_fee: '300000', normal_sompi_per_gram: '1', low_sompi_per_gram: '1', priority_sompi_per_gram: '2' }),
    parse_kpub: () => JSON.stringify({ account_pubkey: HEX32 }), import_kpub: () => JSON.stringify(wallet), extend_addresses: () => JSON.stringify(wallet),
    decode_address: () => JSON.stringify({ payload: HEX32, version: 0 }), encode_p2pk_address: () => ADDRESS,
    pskt_detect: h => String(h).startsWith(PSKB) ? 'pskb' : '', pskt_summary: () => JSON.stringify({ format:'pskb',tx_version:0,input_count:1,output_count:1,fee_sompi:'1000',total_in_sompi:'250000000',total_out_sompi:'249999000',finalize_ready:true,inputs:[],outputs:[] }),
    pskt_relay_to_kspt: () => KSPT, pskt_finalize_and_broadcast: () => TXID, broadcast_signed: () => TXID,
    generate_qr_frames: () => JSON.stringify([{svg:'<svg></svg>'}]), generate_qr_svg_text: () => '<svg></svg>', decode_qr_frame: () => '', decoder_progress: () => JSON.stringify({total:1,count:0,bits:[false]}),
    create_send_pskb: () => PSKB, create_send_pskb_limited: () => PSKB, create_send_pskb_with_utxos: () => PSKB, create_consolidate_pskb: () => PSKB,
    create_covenant_pskb: () => PSKB, create_covenant_pskb_with_payload: () => PSKB, create_covenant_owner_spend: () => PSKB, create_covenant_borrower_spend: () => PSKB, create_covenant_beneficiary_spend: () => PSKB,     create_global_spending_limit_withdraw: () => PSKB, create_global_spending_limit_topup: () => PSKB, create_global_allowance_withdraw: () => PSKB, create_global_allowance_topup: () => PSKB,
    build_covenant_payload: () => '', parse_covenant_payload: () => JSON.stringify({type:'generic'}), sha256_hash: () => HEX32,
  });
  globalThis.fetch = async () => ({ ok: true, status: 200, async json(){ return []; }, async text(){ return '[]'; } });

  const files = [];
  async function walk(dir) { for (const ent of await fs.readdir(dir,{withFileTypes:true})) { const p=path.join(dir,ent.name); if(ent.isDirectory()) await walk(p); else if(ent.name.endsWith('.js')) files.push(p); } }
  await walk(JS_ROOT);
  let invoked = 0, successes = 0, controlled = 0;
  const skip = /(?:start|stop|watch|poll|loop|schedule|interval|socket|camera|download|upload|listen|bootstrap|initialize|init$|attach|bind|register)/i;
  for (const file of files.sort()) {
    const mod = await import(moduleUrl(path.relative(JS_ROOT,file).replaceAll(path.sep,'/')));
    for (const [name, value] of Object.entries(mod)) {
      if (typeof value !== 'function' || /^class\s/.test(Function.prototype.toString.call(value)) || skip.test(name)) continue;
      const params = exportedParams(value);
      for (let variant=0; variant<2; variant++) {
        invoked++;
        try {
          const args = params.map(p => argFor(p,variant));
          const result = value(...args);
          if (result && typeof result.then === 'function') await result;
          successes++;
        } catch (error) {
          controlled++;
          assert.ok(error instanceof Error || typeof error === 'string');
        }
        assert.equal(privateMaterialPresent(), false, `${name} persisted forbidden wallet-private material`);
      }
    }
  }
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(unhandled.length, 0, `unhandled rejections: ${unhandled.map(String).join(' | ')}`);
  assert.ok(invoked > 700, `expected broad valid boundary sweep, got ${invoked}`);
  assert.ok(successes > 200, `expected substantial successful boundaries, got ${successes}`);
  console.log(`PASS: typed valid-boundary sweep (${invoked} calls; ${successes} success; ${controlled} controlled errors)`);
} finally {
  process.off('unhandledRejection', onUnhandled);
  teardownHarness();
}
