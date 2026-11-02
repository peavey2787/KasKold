// The covenant spends the Companion builds in JavaScript, through the real WASM
// boundary: only the node is stood in for, so the verified relay decides whether
// each document names a path the script can run and a signer can sign.
import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { element, moduleUrl, setupHarness, teardownHarness } from '../support/web_recovery_test_harness.mjs';
import { KEY_1G, KEY_2G, KEY_3G, RECEIVE_0, RECEIVE_1, real } from './support.mjs';

const BOUNDARY = [
    'covenant_script_terms', 'covenant_spend_document', 'create_private_swap_refund',
    'network_address_prefix', 'address_network', 'is_kaspa_address', 'pskt_detect',
    'pskt_summary', 'pskt_relay_to_kspt', 'ship_escrow_terms', 'script_public_key_address',
];
const KEY_4G = 'e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13';
const TOTAL = 400_000_000n;

let state;
let shipment;
let owner;
let currentUtxos = [];

function utxo(amount) {
    return { tx_id: '11'.repeat(32), index: 0, amount: String(amount), block_daa_score: '0', script_public_key: '' };
}

function documentAfter(run) {
    state.transactionState._psktReviewHex = null;
    return run().then(() => {
        const wire = state.transactionState._psktReviewHex;
        assert.ok(wire, element('toast').textContent);
        return { wire, summary: JSON.parse(real.pskt_summary(wire, 'mainnet')) };
    });
}

function shipEscrow() {
    const covenant = JSON.parse(real.covenant_ship_escrow(JSON.stringify({
        seller_pubkey_hex: KEY_1G, deliverer_pubkey_hex: KEY_2G, buyer_pubkey_hex: KEY_3G, arbiter_pubkey_hex: KEY_4G,
        product_sompi: '400000000', fee_sompi: '50000000', cltv1_deadline: '1000', cltv2_deadline: '2000', network: 'mainnet',
    })));
    return { covenant, terms: JSON.parse(real.ship_escrow_terms(covenant.redeem_script_hex, 'mainnet')) };
}

before(async () => {
    await setupHarness();
    const stubs = globalThis.__COMPANION_WASM_STUBS__;
    for (const name of BOUNDARY) stubs[name] = (...args) => real[name](...args);
    stubs.fetch_utxos_for_address_js = () => JSON.stringify(currentUtxos);
    state = await import(moduleUrl('app/state/index.js'));
    state.networkState.network = 'mainnet';
    state.networkState.customNodeUrl = 'wss://unused';
    shipment = await import(moduleUrl('features/covenants/spending/standard/shipment.js'));
    owner = await import(moduleUrl('features/covenants/spending/standard/spend_paths/owner.js'));
});

after(() => teardownHarness());

test('every shipment path relays, and only the timeouts need no signer', async () => {
    const { covenant, terms } = shipEscrow();
    element('cov-ship-addr').value = covenant.address;
    element('cov-ship-script').value = covenant.redeem_script_hex;
    const paths = [
        ['pickup', terms.total_sompi, false], ['state0-arb-refund', terms.total_sompi, false], ['state0-timeout', terms.total_sompi, true],
        ['delivery', terms.rem_sompi, false], ['state1-arb-award', terms.rem_sompi, false],
        ['state1-timeout', terms.rem_sompi, true], ['state1-arb-refund', terms.rem_sompi, false],
    ];
    for (const [path, amount, keyless] of paths) {
        currentUtxos = [utxo(amount)];
        const { wire, summary } = await documentAfter(() => shipment.handleShipEscrowSpend(path));
        assert.equal(summary.finalize_ready, keyless, path);
        if (!keyless) assert.equal(real.kspt_signature_status(real.pskt_relay_to_kspt(wire, 'mainnet')), 'unsigned', path);
    }
});

test('every escrow path relays to the signer it names', async () => {
    const covenant = JSON.parse(real.covenant_escrow(KEY_1G, KEY_2G, KEY_3G, RECEIVE_0, RECEIVE_1, 'mainnet'));
    state.covenantState.lastCovenantResult = { ...covenant, type: 'escrow' };
    currentUtxos = [utxo(TOTAL)];
    const paths = ['buyer-release', 'seller-refund', 'arbiter-award-seller', 'arbiter-refund-buyer', 'buyer-dispute', 'seller-dispute'];
    for (const path of paths) {
        const { wire, summary } = await documentAfter(() => shipment.handleEscrowSpend(path));
        assert.equal(summary.finalize_ready, false, path);
        assert.equal(real.kspt_signature_status(real.pskt_relay_to_kspt(wire, 'mainnet')), 'unsigned', path);
    }
});

test('a partial owner withdrawal keeps its change at the covenant and relays', async () => {
    const covenant = JSON.parse(real.covenant_timelocked_savings(KEY_1G, KEY_2G, 1000n, 'mainnet'));
    state.covenantState.lastCovenantResult = { ...covenant, type: 'timelocked-savings' };
    element('cov-owner-addr').value = covenant.address;
    element('cov-owner-script').value = covenant.redeem_script_hex;
    element('cov-owner-dest').value = RECEIVE_0;
    element('cov-owner-amount').value = '1';
    element('cov-owner-panel').dataset.covOwnerType = 'timelocked-savings';
    currentUtxos = [utxo(TOTAL)];
    const { wire, summary } = await documentAfter(() => owner.handleCovOwnerSpend());
    assert.equal(summary.output_count, 2);
    assert.equal(summary.outputs[1].address, covenant.address);
    assert.equal(real.kspt_signature_status(real.pskt_relay_to_kspt(wire, 'mainnet')), 'unsigned');
});

test('a covenant path the script cannot run never reaches a document', async () => {
    const { covenant, terms } = shipEscrow();
    element('cov-ship-addr').value = covenant.address;
    element('cov-ship-script').value = covenant.redeem_script_hex;
    currentUtxos = [utxo(terms.rem_sompi)];
    state.transactionState._psktReviewHex = null;
    await shipment.handleShipEscrowSpend('buyer-release');
    assert.equal(state.transactionState._psktReviewHex, null);
    assert.match(element('toast').textContent, /Unknown branch/);
    assert.throws(() => real.covenant_spend_document(JSON.stringify({
        covenant_address: covenant.address, redeem_script_hex: covenant.redeem_script_hex, path: 'buyer-release',
        tx_version: 1, lock_time: '0', payload_hex: '', utxos: [utxo(TOTAL)], outputs: [{ address: RECEIVE_0, amount: '1' }],
    })), /no buyer-release path/);
});
