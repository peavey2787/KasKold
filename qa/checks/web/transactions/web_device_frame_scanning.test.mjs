import assert from 'node:assert/strict';
import { setupHarness, teardownHarness, moduleUrl } from '../support/web_recovery_test_harness.mjs';

const RESPONSE_LEN = 247;
const response = new Uint8Array(RESPONSE_LEN);
response.set([0x43, 0x56, 0x52, 0x53, 1, 0]);
const responseHex = Buffer.from(response).toString('hex');
const frame = index => Uint8Array.from([0x4b, 0x51, index]);

await setupHarness();
try {
  const { parseCovenantResponse } = await import(moduleUrl('features/covenants/signing/protocol.js'));
  const { wholePayload } = await import(moduleUrl('features/stealth/index/whole_payload.js'));

  const seen = [];
  const scan = wholePayload(raw => { seen.push(raw); return true; });

  // A payload that fit in one QR reaches the scanner's owner untouched.
  assert.equal(scan(response), true);
  assert.deepEqual([...seen[0]], [...response]);
  seen.length = 0;

  // Partial frames report progress and keep scanning without reaching the parser.
  const wire = [];
  Object.assign(globalThis.__COMPANION_WASM_STUBS__, {
    decode_qr_frame: hex => { wire.push(hex); return wire.length === 3 ? responseHex : ''; },
    decoder_progress: () => JSON.stringify({ total: 3, count: wire.length, bits: [true, false, false] }),
  });
  assert.equal(scan(frame(0)), false);
  assert.equal(scan(frame(1)), false);
  assert.equal(seen.length, 0);

  // The last frame hands over the assembled payload, which a parser that refuses frames accepts.
  assert.throws(() => parseCovenantResponse(frame(0)), /Not a current KasKold covenant response/);
  assert.equal(scan(frame(2)), true);
  assert.equal(seen.length, 1);
  assert.deepEqual([...seen[0]], [...response]);
  assert.deepEqual(wire, ['4b5100', '4b5101', '4b5102']);

  // A corrupt frame keeps the scanner open rather than reaching the parser.
  seen.length = 0;
  globalThis.__COMPANION_WASM_STUBS__.decode_qr_frame = () => { throw new Error('bad frame'); };
  assert.equal(scan(frame(0)), false);
  assert.equal(seen.length, 0);

  console.log('PASS: device frame sequences reach covenant parsers whole');
} finally {
  teardownHarness();
}
