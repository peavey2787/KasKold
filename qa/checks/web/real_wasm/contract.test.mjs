// The Companion's other web tests stub WASM. These hold those stubs to the real
// build: same names, same answers on canonical input, no trap on hostile input.
import assert from 'node:assert/strict';
import { after, test } from 'node:test';
import { PORTAL_WASM_FAKES } from '../support/portal_wasm_fakes.mjs';
import {
    ACCOUNT_KEY, ACCOUNT_PUBKEY, CHANGE_0, KEY_1G, KEY_1G_ADDRESS, KEY_2G, KEY_3G, RECEIVE_0,
    RECEIVE_0_PAYLOAD, RECEIVE_1, called, explicitUtxo, exportSignatures, real, spendFromReceive0, unrecorded, webModule,
} from './support.mjs';

const signatures = exportSignatures();
const { generatedWasmExportNames } = await import(webModule('wasm/api.js'));

/** These cannot finish without a node; hostile arguments are all they meet here. */
const NEEDS_A_NODE = new Set([
    'broadcast_signed', 'create_commit_reveal_spend', 'create_consolidate_pskb',
    'create_covenant_beneficiary_spend', 
    'create_covenant_oracle_v1_claim', 'create_covenant_owner_spend',
    'create_covenant_payjoin_claim', 'create_covenant_pskb', 'create_covenant_pskb_with_payload',
    'create_covenant_timelocked_savings_claim', 'create_crowdfund_sweep',
    'create_global_allowance_topup', 'create_global_allowance_withdraw', 'create_global_spending_limit_topup',
    'create_global_spending_limit_withdraw', 'create_merkle_whitelist_spend', 'create_multisig_pskb',
    'create_multisig_pskb_multi_js', 'create_multisig_pskb_selected', 'create_oracle_mb_publish', 'create_send_pskb',
    'create_send_pskb_limited', 'create_send_pskb_selected', 'create_stealth_spend', 'fetch_balance', 'fetch_utxos',
    'fetch_utxos_complete', 'fetch_utxos_for_address_js', 'get_fee_estimate',
    'get_virtual_daa_score', 'inspect_crowdfund_contributions', 'pskt_finalize_and_broadcast',
    'scan_multisig_branch_js', 'split_vault_genesis_pskb', 'split_vault_spend_pskb', 'stealth_create_payment_lane',
    'tagged_vault_genesis_pskb', 'tagged_vault_spend_pskb',
]);

const HOSTILE = {
    string: ['', 'zz', '00', '{}', '[]', 'kaspa:', 'mainnet', '\u0000', 'ff'.repeat(5000), '{"a":'],
    number: [0, 4294967295],
    bigint: [0n, 18446744073709551615n],
    boolean: [false, true],
    Uint8Array: [new Uint8Array(0), new Uint8Array(300).fill(255)],
};

function rejected(operation, pattern) {
    assert.throws(operation, pattern);
}

after(() => real.reset_qr_decoder());

test('the browser boundary names exactly the exports the build generates', () => {
    assert.deepEqual([...generatedWasmExportNames()].sort(), [...signatures.keys()].sort());
});

test('every Portal fake stands in for a real export and takes no extra argument', () => {
    for (const [name, fake] of Object.entries(PORTAL_WASM_FAKES)) {
        assert.ok(signatures.has(name), `${name} is faked but the build does not export it`);
        assert.ok(fake.length <= signatures.get(name).parameters.length, `${name} fake takes too many arguments`);
    }
});

test('no export traps on hostile arguments', async () => {
    for (const [name, { parameters }] of signatures) {
        const rounds = Math.max(1, ...parameters.map(type => HOSTILE[type].length));
        for (let round = 0; round < rounds; round += 1) {
            const args = parameters.map(type => HOSTILE[type][round % HOSTILE[type].length]);
            try {
                await unrecorded[name](...args);
            } catch (error) {
                assert.ok(!(error instanceof WebAssembly.RuntimeError), `${name} round ${round} trapped: ${error}`);
            }
        }
    }
});

test('an account key derives the independently worked out addresses', () => {
    const wallet = JSON.parse(real.import_kpub(ACCOUNT_KEY, 'mainnet'));
    assert.equal(wallet.kpub, ACCOUNT_KEY);
    assert.deepEqual(wallet.receive_addresses.slice(0, 2), [RECEIVE_0, RECEIVE_1]);
    assert.equal(wallet.change_addresses[0], CHANGE_0);
    assert.equal(wallet.receive_addresses.length, 20);
    const raw = JSON.parse(real.import_kpub_raw(Buffer.from(ACCOUNT_KEY.slice('kpub1:'.length), 'hex'), 'testnet-10'));
    assert.equal(raw.receive_addresses[0], 'kaspatest:qqd6e65yefepe9wk0m9vuxdufxd80sphy67gwwd0vdaumzdt4tc9ssxd5s7gn');
    assert.deepEqual(JSON.parse(real.parse_kpub(ACCOUNT_KEY)), { account_pubkey: ACCOUNT_PUBKEY });
    const extended = JSON.parse(real.extend_addresses(JSON.stringify(wallet), 2, 1, 'mainnet'));
    assert.equal(extended.receive_addresses.length, 22);
    assert.equal(extended.change_addresses.length, 21);
    rejected(() => real.import_kpub(`kpub1:${'11'.repeat(78)}`, 'mainnet'), /./);
});

test('addresses encode and decode to known answers', () => {
    assert.equal(real.encode_p2pk_address(KEY_1G, 'mainnet'), KEY_1G_ADDRESS);
    assert.equal(real.encode_p2pk_address(KEY_1G), KEY_1G_ADDRESS);
    assert.deepEqual(JSON.parse(real.decode_address(RECEIVE_0)), { payload: RECEIVE_0_PAYLOAD, version: 0 });
    assert.equal(real.script_public_key_address(`20${RECEIVE_0_PAYLOAD}ac`, 'mainnet'), RECEIVE_0);
    assert.equal(real.script_public_key_address('00', 'mainnet'), '');
});

test('hashes answer the published vectors', () => {
    assert.equal(real.sha256_hash(''), 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
    assert.equal(real.sha256_hash('616263'), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
});

test('the Portal fakes answer as the real exports do on canonical input', () => {
    for (const network of ['mainnet', 'testnet-10', 'testnet-12', 'devnet', 'simnet', 'nope']) {
        assert.equal(PORTAL_WASM_FAKES.network_address_prefix(network), real.network_address_prefix(network), network);
    }
    const testnet = real.encode_p2pk_address(KEY_1G, 'testnet-12');
    for (const [address, selected] of [[RECEIVE_0, 'mainnet'], [testnet, 'testnet-12'], [testnet, 'testnet-10']]) {
        assert.equal(PORTAL_WASM_FAKES.address_network(address, selected), real.address_network(address, selected));
    }
    for (const text of [RECEIVE_0, testnet, 'bitcoin:abc', '']) {
        assert.equal(PORTAL_WASM_FAKES.is_kaspa_address(text), real.is_kaspa_address(text), text);
    }
    assert.equal(PORTAL_WASM_FAKES.private_swap_max_fee_sompi(), real.private_swap_max_fee_sompi());
    assert.equal(PORTAL_WASM_FAKES.covenant_spend_path('00', '00'), real.covenant_spend_path('00', '00'));
    assert.equal(PORTAL_WASM_FAKES.script_public_key_address('00', 'mainnet'), real.script_public_key_address('00', 'mainnet'));
    const descriptor = `multi(2,${KEY_1G},${KEY_2G})`;
    assert.deepEqual(
        JSON.parse(PORTAL_WASM_FAKES.multisig_descriptor_summary(descriptor)),
        JSON.parse(real.multisig_descriptor_summary(descriptor)),
    );
    rejected(() => real.multisig_descriptor_summary('nope'), /Descriptor must be/);
    rejected(() => PORTAL_WASM_FAKES.multisig_descriptor_summary('nope'), /not a multisig descriptor/);
    assert.equal(
        PORTAL_WASM_FAKES.thread_withdraw_fee(10_000_000_000n, 100_000_000n, 300_000n),
        real.thread_withdraw_fee(10_000_000_000n, 100_000_000n, 300_000n, 1n),
    );
    assert.equal(
        PORTAL_WASM_FAKES.thread_maximum_withdraw(10_000_000_000n, 1_000_000_000n, 300_000n, 1n, 100_000n),
        real.thread_maximum_withdraw(10_000_000_000n, 1_000_000_000n, 300_000n, 1n, 100_000n),
    );
});

const COVENANTS = {
    dms: () => real.covenant_dms(KEY_1G, KEY_2G, 1000n, 'mainnet'),
    'timelocked-savings': () => real.covenant_timelocked_savings(KEY_1G, KEY_2G, 1000n, 'mainnet'),
    additive: () => real.covenant_additive_address(KEY_1G, 1000n, 1000n, 'mainnet'),
    escrow: () => real.covenant_escrow(KEY_1G, KEY_2G, KEY_3G, RECEIVE_0, RECEIVE_1, 'mainnet'),
    'global-allowance': () => real.covenant_global_allowance(KEY_1G, KEY_2G, 1000n, 10n, 0n, 'mainnet'),
    'global-spending-limit': () => real.covenant_global_spending_limit(KEY_1G, 1000n, 10n, 'mainnet'),
    payjoin: () => real.covenant_payjoin(KEY_1G, KEY_2G, 1000n, 2n, 2n, 'mainnet'),
};

test('every invite-loadable covenant family reads its terms back from its own script', () => {
    for (const [type, build] of Object.entries(COVENANTS)) {
        const covenant = JSON.parse(build());
        const redeem = covenant.redeem_script_hex;
        assert.equal(real.redeem_script_address(redeem, 'mainnet'), covenant.address, type);
        const terms = JSON.parse(real.covenant_script_terms(type, redeem, covenant.address, 'mainnet', ''));
        const fake = JSON.parse(PORTAL_WASM_FAKES.covenant_script_terms(type, redeem, covenant.address));
        for (const field of ['type', 'address', 'redeem_script_hex']) assert.equal(terms[field], fake[field], `${type} ${field}`);
        rejected(() => real.covenant_script_terms(type, redeem, KEY_1G_ADDRESS, 'mainnet', ''), /does not match its redeem script/);
    }
    rejected(() => real.covenant_script_terms('nope', '00', RECEIVE_0, 'mainnet', ''), /Unsupported covenant type/);
    rejected(() => PORTAL_WASM_FAKES.covenant_script_terms('nope', '00', RECEIVE_0), /Unsupported covenant type/);
});

test('covenant builders that have no invite still produce a script at their address', () => {
    const covenants = {
        commit: real.covenant_commit_reveal(KEY_1G, 'ab'.repeat(32), 1000n, 'mainnet'),
        merkle: real.covenant_merkle_whitelist(KEY_1G, 'ab'.repeat(32), 1, 1000n, 'mainnet'),
        swap: real.covenant_private_swap(KEY_1G, KEY_2G, RECEIVE_0, 1000n, '11'.repeat(16), 'mainnet'),
    };
    for (const [name, text] of Object.entries(covenants)) {
        const covenant = JSON.parse(text);
        assert.equal(real.redeem_script_address(covenant.redeem_script_hex, 'mainnet'), covenant.address, name);
    }
    rejected(() => real.covenant_crowdfund(KEY_1G, RECEIVE_0, 1000n, 1000n, 'zz', 'mainnet'), /./);
    rejected(() => real.covenant_oracle_v1(KEY_1G, KEY_2G, KEY_3G, '00'.repeat(32), 'it rained', 1000n, 'mainnet'), /key ID cannot be zero/);
    rejected(() => real.covenant_ship_escrow('{}'), /missing field `seller_pubkey_hex`/);
    rejected(() => real.ship_escrow_terms('00', 'mainnet'), /unrecognized ship-escrow redeem/);
    rejected(() => real.covenant_oracle_mb('{}'), /missing field `genesis_price`/);
});

test('an offline spend travels PSKB to summary to KSPT to QR frames and back', async () => {
    const pskb = await spendFromReceive0();
    assert.equal(real.pskt_detect(pskb), 'pskb');
    const summary = JSON.parse(real.pskt_summary(pskb, 'mainnet'));
    assert.equal(summary.input_count, 1);
    assert.equal(summary.output_count, 2);
    assert.equal(summary.inputs[0].amount_sompi, '50000000');
    const kspt = real.pskt_relay_to_kspt(pskb, 'mainnet');
    assert.equal(real.kspt_signature_status(kspt), 'unsigned');

    real.reset_qr_decoder();
    let decoded = '';
    const frames = JSON.parse(real.generate_qr_frames(kspt));
    assert.ok(frames.length > 1);
    for (const frame of frames) decoded = real.decode_qr_frame(frame.frame_hex);
    assert.equal(decoded, kspt);
    assert.equal(real.decoder_progress(), '0/0');

    const fee = `20${JSON.parse(real.decode_address(KEY_1G_ADDRESS)).payload}ac`;
    const appended = real.pskb_append_output(pskb, 1000n, `0000${fee}`);
    assert.equal(JSON.parse(real.pskt_summary(appended, 'mainnet')).output_count, 3);
    rejected(() => real.pskb_append_output(pskb, 1000n, fee), /must be version 0/);

    const prepared = JSON.parse(real.kaskold_sdk_prepare(pskb, 'mainnet'));
    assert.equal(prepared.originalPsktHex, pskb);
    rejected(() => real.kaskold_sdk_complete(pskb, '00', 'mainnet'), /./);
    assert.deepEqual(
        JSON.parse(real.kaskold_sdk_limits()).qrMaxFrames,
        64,
    );
});

test('anti-klepto and swap signing refuse anything but their exact inputs', async () => {
    const kspt = real.pskt_relay_to_kspt(await spendFromReceive0(), 'mainnet');
    const { hostSecretHex, requestHex } = JSON.parse(real.anti_klepto_begin(kspt));
    assert.equal(hostSecretHex.length, 64);
    assert.match(requestHex, /^4b414b50/);
    rejected(() => real.anti_klepto_accept_commitment(requestHex, '00', '00'), /host secret must be 32 bytes/);
    rejected(() => real.anti_klepto_verify_signed(requestHex, '00', kspt, hostSecretHex), /./);

    assert.equal(real.private_swap_claim_sighash(kspt).length, 64);
    assert.match(real.private_swap_key_request(), /^50535747/);
    const zero = '00'.repeat(32);
    const zeroSignature = '00'.repeat(64);
    rejected(() => real.private_swap_parse_response('00'), /Invalid Private Swap response/);
    rejected(() => real.private_swap_bind_request(zero, zero, '00'), /./);
    rejected(() => real.private_swap_presign_request(zero, zero, zero, kspt, zero), /./);
    rejected(() => real.private_swap_complete_request(zero, zero, zero, kspt, zeroSignature, false), /./);
    rejected(() => real.private_swap_reveal_request('00', zero, zero, zero), /./);
    rejected(() => real.private_swap_complete_public(zeroSignature, false, zero), /./);
    rejected(() => real.private_swap_extract_secret(zeroSignature, false, zeroSignature), /./);
    rejected(() => real.private_swap_insert_completed_signature('00', KEY_1G, zeroSignature), /./);
    assert.equal(real.private_swap_verify_completed(KEY_1G, zero, zeroSignature), false);
    assert.equal(real.private_swap_verify_presignature(KEY_1G, zero, zeroSignature, false, KEY_2G), false);
    assert.equal(real.private_swap_verify_host_relation(KEY_1G, zero, KEY_2G, '00', zero, '00', '00'), false);
    rejected(() => real.verify_covenant_anti_klepto(KEY_1G, zero, '00', zeroSignature, '00', zero), /./);
    assert.equal(real.verify_oracle_v1_attestation(KEY_1G, zeroSignature, zero), false);
    await assert.rejects(real.create_private_swap_claim(RECEIVE_0, RECEIVE_1, '00', '{}', 1n), /./);
});

test('explicit-coin spends reach their fail-closed checks without a node', async () => {
    const coins = JSON.stringify([explicitUtxo()]);
    rejected(() => real.create_covenant_owner_spend_selected(RECEIVE_0, RECEIVE_1, '00', coins, 1n, 'owner'), /does not match the supplied redeem script/);
    rejected(() => real.create_covenant_beneficiary_spend_selected(RECEIVE_0, RECEIVE_1, '00', 1n, coins, 1n), /does not match the supplied redeem script/);
    rejected(() => real.create_covenant_timelocked_savings_claim_selected(RECEIVE_0, RECEIVE_1, '00', 1n, 0, coins, 1n), /does not match the supplied redeem script/);
    rejected(() => real.create_private_swap_refund(RECEIVE_0, RECEIVE_1, '00', coins, 1n), /does not match the supplied redeem script/);
    await assert.rejects(real.create_send_pskb_with_utxos('{', RECEIVE_1, 1n, 1n, coins), /Bad wallet/);
});

test('a covenant spend names a path its script can run, or it is refused', () => {
    const covenant = JSON.parse(real.covenant_dms(KEY_1G, KEY_2G, 1000n, 'mainnet'));
    const request = (path, amount) => JSON.stringify({
        covenant_address: covenant.address, redeem_script_hex: covenant.redeem_script_hex, path, tx_version: 0,
        lock_time: '0', payload_hex: '', utxos: [explicitUtxo()], outputs: [{ address: RECEIVE_0, amount }],
    });
    const summary = JSON.parse(real.pskt_summary(real.covenant_spend_document(request('owner', '40000000')), 'mainnet'));
    assert.equal(summary.outputs[0].amount_sompi, '40000000');
    assert.equal(summary.finalize_ready, false);
    rejected(() => real.covenant_spend_document(request('pickup', '40000000')), /no pickup path/);
    rejected(() => real.covenant_spend_document(request('owner', '50000001')), /exceed its inputs/);
    rejected(() => real.covenant_spend_document('{}'), /Bad covenant spend JSON/);
});

test('stealth, merkle, crowdfund and payload helpers answer consistently', () => {
    const meta = JSON.parse(real.stealth_meta_from_kpub(ACCOUNT_KEY));
    assert.equal(meta.spend_pubkey, ACCOUNT_PUBKEY);
    assert.equal(meta.meta_address, meta.scan_pubkey + meta.spend_pubkey);
    const payment = JSON.parse(real.stealth_generate_payment(meta.meta_address, '01'.repeat(32), 'mainnet'));
    assert.equal(payment.address.split(':')[0], 'kaspa');
    assert.equal(real.is_kaspa_address(payment.address), true);
    assert.equal(real.is_kaspa_address(real.stealth_announcement_address('mainnet')), true);
    assert.equal(real.derive_covenant_payload_key(ACCOUNT_KEY).length, 64);

    const addresses = JSON.stringify([RECEIVE_0, KEY_1G_ADDRESS]);
    const merkle = JSON.parse(real.merkle_root_from_addresses(addresses));
    assert.equal(merkle.leaf_count, 2);
    const proof = JSON.parse(real.merkle_proof_for_address(addresses, RECEIVE_0));
    assert.equal(proof.leaf_index, 0);
    assert.equal(proof.leaf_spk_hex, `000020${RECEIVE_0_PAYLOAD}ac`);

    assert.equal(real.crowdfund_campaign_id(RECEIVE_0, 1000n, 1000n, 'ab'.repeat(32)).length, 64);
    assert.deepEqual(
        JSON.parse(real.parse_covenant_payload(real.build_covenant_payload(1, '00'))),
        { covenant_type: 1, params_hex: '00', version: 1 },
    );
    rejected(() => real.read_commit_reveal_reveal('03616263'), /not a commit-reveal reveal/);
    assert.ok(BigInt(real.estimate_covenant_fee(1, 100, 0, 0)) > 0n);
    assert.equal(real.version(), 'KasKold Companion Web');
    assert.match(real.generate_qr_svg_text('hello'), /^<svg /);
    assert.match(Buffer.from(real.build_vcc_subscribe_request(1n)).toString('hex'), /^0101/);

    const setup = JSON.parse(real.zk_crowdfund_setup());
    assert.equal(setup.vk_hex.length / 2, setup.vk_len);
    const proved = JSON.parse(real.zk_crowdfund_prove(setup.pk_hex, setup.vk_hex, JSON.stringify(['1000', '2000'])));
    assert.ok(proved.proof_hex.length > 0);
});

// Last in the file: the tests above ran in order and recorded what they called.
test('every export is exercised with meaningful arguments unless it needs a node', async () => {
    const unexercised = [...signatures.keys()].filter(name => !called.has(name));
    assert.deepEqual(unexercised.filter(name => !NEEDS_A_NODE.has(name)), []);
    for (const name of NEEDS_A_NODE) assert.ok(signatures.get(name)?.isAsync, `${name} is not an async export`);
});
