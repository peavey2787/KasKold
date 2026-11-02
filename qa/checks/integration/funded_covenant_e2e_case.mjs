// Every covenant spend the Companion can make, against a real public testnet
// node: fund the covenant from the funded wallet, spend it through the same code
// the screens run, have the signer sign it, and see the coin arrive. The page
// asks the QA server to sign because only the signer holds the key.
import * as wasm from '/target/kaskold-companion-web/site/js/wasm/api.js';
import { networkState } from '/target/kaskold-companion-web/site/js/app/state/index.js';
import { withNodeRetry } from '/target/kaskold-companion-web/site/js/features/wallet/core.js';
import {
    buildEscrowSpendPskb, buildShipSpendPskb, getShipParams, resolveEscrowDestination,
} from '/target/kaskold-companion-web/site/js/features/covenants/spending/standard/shipment.js';

const KAS = 100_000_000n;
const FEE = 1_000_000n;
const THREAD_FEE_FLOOR = 300_000n;
const FUNDING = 3n * KAS;
// The builder counts the wallet's coins largest first.
const LARGEST_COIN = '0';
const POLL_MS = 2_000;
const COIN_TIMEOUT_MS = 120_000;
const CASE_TIMEOUT_MS = 300_000;
/** DAA scores between "now" and a lock: long enough to fund before it passes. */
const LOCK_MARGIN = 60n;
const ROLE_KEYS = 3;
const ESCROW_DISPUTES = ['buyer-dispute', 'seller-dispute'];
const ESCROW_PAYOUTS = ['buyer-release', 'seller-refund', 'arbiter-award-seller', 'arbiter-refund-buyer'];

let wallet;
let walletJson;
let network;

async function finish(status, detail) {
    document.documentElement.dataset.qaStatus = status;
    const response = await fetch('/__qa_funded_result__', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ status, detail }),
    });
    if (!response.ok) throw new Error(`cannot publish covenant result: HTTP ${response.status}`);
}

async function sign(kspt) {
    const response = await fetch('/__qa_funded_sign__', { method: 'POST', body: kspt });
    if (!response.ok) throw new Error(await response.text());
    return response.text();
}

const pause = ms => new Promise(resolve => { setTimeout(resolve, ms); });

async function report(line) {
    await fetch('/__qa_funded_progress__', { method: 'POST', body: line });
}

/** A node call that never answers fails its case instead of the whole run. */
function withinCaseLimit(spend) {
    return Promise.race([
        spend(),
        pause(CASE_TIMEOUT_MS).then(() => { throw new Error(`no result within ${CASE_TIMEOUT_MS} ms`); }),
    ]);
}

const node = operation => withNodeRetry(operation);
const key = index => JSON.parse(wasm.decode_address(wallet.receive_addresses[index])).payload;
const daa = async () => BigInt(await node(wsUrl => wasm.get_virtual_daa_score(wsUrl)));
const coinsAt = async address => JSON.parse(await node(wsUrl => wasm.fetch_utxos_for_address_js(address, wsUrl)));

async function untilDaaPasses(score) {
    while (await daa() <= score) await pause(POLL_MS);
}

async function coinFrom(address, txid) {
    const deadline = Date.now() + COIN_TIMEOUT_MS;
    while (Date.now() < deadline) {
        const coin = (await coinsAt(address)).find(utxo => utxo.tx_id.toLowerCase() === txid.toLowerCase());
        if (coin) return coin;
        await pause(POLL_MS);
    }
    throw new Error(`${txid} was accepted but never paid ${address}`);
}

async function broadcast(document_) {
    const summary = JSON.parse(wasm.pskt_summary(document_, network));
    let wire = document_;
    if (!summary.finalize_ready) {
        const signed = await sign(wasm.pskt_relay_to_kspt(document_, network));
        wire = JSON.parse(wasm.kaskold_sdk_complete(document_, signed, network)).psktHex;
    }
    return node(wsUrl => wasm.pskt_finalize_and_broadcast(wire, wsUrl));
}

async function fund(address, covenantType, amount = FUNDING) {
    const document_ = await node(wsUrl => wasm.create_covenant_pskb(JSON.stringify({
        wallet_json: walletJson,
        covenant_address: address,
        covenant_type: covenantType,
        send_amount: String(amount),
        fee: String(FEE),
        change_address: wallet.change_addresses[0],
        utxo_indices_csv: LARGEST_COIN,
        ws_url: wsUrl,
    })));
    return coinFrom(address, await broadcast(document_));
}

async function spendAndSee(document_, destination) {
    const txid = await broadcast(document_);
    await coinFrom(destination, txid);
    return txid;
}

const destination = () => wallet.receive_addresses[5];
const parsed = text => JSON.parse(text);

const ownerSweep = (covenant, branch) => node(wsUrl => wasm.create_covenant_owner_spend(
    covenant.address, destination(), covenant.redeem_script_hex, FEE, wsUrl, branch,
));

/** A timelocked owner refund: fund it, let the lock pass, take it all back. */
async function refundAfterLock(covenant, lock, covenantType) {
    await fund(covenant.address, covenantType);
    await untilDaaPasses(lock);
    return spendAndSee(await ownerSweep(covenant, 'owner-time'), destination());
}

function dmsCases() {
    const inactivity = 30n;
    return {
        'dead-man-switch owner': async () => {
            const covenant = parsed(wasm.covenant_dms(key(0), key(1), inactivity, network));
            await fund(covenant.address, 'dms');
            return spendAndSee(await node(wsUrl => wasm.create_covenant_owner_spend(
                covenant.address, destination(), covenant.redeem_script_hex, FEE, wsUrl, 'owner',
            )), destination());
        },
    };
}

function savingsCases() {
    const claim = position => async () => {
        const lock = await daa() + LOCK_MARGIN;
        const covenant = parsed(wasm.covenant_timelocked_savings(key(0), key(1), lock, network));
        await fund(covenant.address, 'timelocked-savings');
        await untilDaaPasses(lock);
        return spendAndSee(await node(wsUrl => wasm.create_covenant_timelocked_savings_claim(
            covenant.address, destination(), covenant.redeem_script_hex, lock, position, FEE, wsUrl,
        )), destination());
    };
    return {
        'savings claimed by its first key': claim(0),
        'savings claimed by its second key': claim(1),
    };
}

function escrowCases() {
    const terms = { alice_address: wallet.receive_addresses[3], bob_address: wallet.receive_addresses[4] };
    // Every escrow is salted, so one covenant must serve both the funding and the spend.
    const cases = {};
    for (const branch of [...ESCROW_DISPUTES, ...ESCROW_PAYOUTS]) {
        cases[`escrow ${branch}`] = async () => {
            const covenant = parsed(wasm.covenant_escrow(key(0), key(1), key(2), terms.alice_address, terms.bob_address, network));
            await fund(covenant.address, 'escrow');
            const paidTo = resolveEscrowDestination(branch, covenant.address, terms);
            return spendAndSee(await buildEscrowSpendPskb(branch, covenant.address, covenant.redeem_script_hex, paidTo), paidTo);
        };
    }
    return cases;
}

function piggyCases() {
    return {
        'piggy bank broken with no conditions': async () => {
            const covenant = parsed(wasm.covenant_additive_address(key(0), 0n, 0n, network));
            await fund(covenant.address, 'additive');
            return spendAndSee(await ownerSweep(covenant, ''), destination());
        },
        'piggy bank broken at its goal': async () => {
            const covenant = parsed(wasm.covenant_additive_address(key(0), KAS, 0n, network));
            await fund(covenant.address, 'additive');
            return spendAndSee(await ownerSweep(covenant, ''), destination());
        },
        'piggy bank broken after its deadline': async () => {
            const lock = await daa() + LOCK_MARGIN;
            const covenant = parsed(wasm.covenant_additive_address(key(0), 0n, lock, network));
            return refundAfterLock(covenant, lock, 'additive');
        },
    };
}

async function threadWithdrawal(withdraw, covenant, coin, amount) {
    const fee = wasm.thread_withdraw_fee(BigInt(coin.amount), amount, THREAD_FEE_FLOOR, 1n);
    const txid = await spendAndSee(await withdraw(
        covenant.address, destination(), covenant.redeem_script_hex, coin.covenant_id || '', amount, fee, JSON.stringify([coin]),
    ), destination());
    return { txid, left: BigInt(coin.amount) - amount - fee };
}

function threadCases() {
    return {
        'spending limit withdrawn in part, then closed': async () => {
            const covenant = parsed(wasm.covenant_global_spending_limit(key(0), FUNDING, 0n, network));
            const coin = await fund(covenant.address, 'global-spending-limit');
            const { txid, left } = await threadWithdrawal(wasm.create_global_spending_limit_withdraw, covenant, coin, KAS);
            if (left <= 0n) throw new Error('the withdrawal left nothing to close');
            const thread = await coinFrom(covenant.address, txid);
            return (await threadWithdrawal(wasm.create_global_spending_limit_withdraw, covenant, thread, BigInt(thread.amount))).txid;
        },
        'allowance withdrawn by its beneficiary, then reclaimed by its owner': async () => {
            const covenant = parsed(wasm.covenant_global_allowance(key(0), key(1), KAS, 0n, 0n, network));
            const coin = await fund(covenant.address, 'global-allowance');
            const { txid } = await threadWithdrawal(wasm.create_global_allowance_withdraw, covenant, coin, KAS);
            await coinFrom(covenant.address, txid);
            return spendAndSee(await ownerSweep(covenant, ''), destination());
        },
    };
}

function timelockedRefundCases() {
    const hash = '11'.repeat(32);
    const lockedCase = build => async () => {
        const lock = await daa() + LOCK_MARGIN;
        const { covenant, type } = await build(lock);
        return refundAfterLock(covenant, lock, type);
    };
    return {
        'payjoin refunded to its owner after its lock': lockedCase(lock => ({
            covenant: parsed(wasm.covenant_payjoin(key(0), key(1), lock, 2n, 2n, network)), type: 'payjoin',
        })),
        'commit-reveal refunded to its owner after its lock': lockedCase(lock => ({
            covenant: parsed(wasm.covenant_commit_reveal(key(0), hash, lock, network)), type: 'commit-reveal',
        })),
        'whitelist refunded to its owner after its lock': lockedCase(lock => {
            const root = parsed(wasm.merkle_root_from_addresses(JSON.stringify(wallet.receive_addresses.slice(3, 6))));
            return { covenant: parsed(wasm.covenant_merkle_whitelist(key(0), root.root, root.depth, lock, network)), type: 'merkle-whitelist' };
        }),
        'oracle covenant refunded to its owner after its lock': lockedCase(lock => ({
            covenant: parsed(wasm.covenant_oracle_v1(key(0), key(1), key(2), hash, 'the release happened', lock, network)), type: 'oracle-v1',
        })),
        'crowdfund contribution refunded after its deadline': lockedCase(lock => {
            const { vk_hex: verifyingKey } = parsed(wasm.zk_crowdfund_setup());
            return {
                covenant: parsed(wasm.covenant_crowdfund(key(0), wallet.receive_addresses[3], FUNDING, lock, verifyingKey, network)),
                type: 'crowdfund',
            };
        }),
    };
}

async function attested(commitment) {
    const answer = await fetch('/__qa_funded_attest__', { method: 'POST', body: commitment });
    if (!answer.ok) throw new Error(await answer.text());
    const [oracleKey, signature] = (await answer.text()).trim().split(' ');
    return { oracleKey, signature };
}

function claimCases() {
    return {
        'oracle covenant claimed with its attestation': async () => {
            // The oracle's key is fixed, and the covenant commits to a statement it salts.
            const { oracleKey } = await attested('00'.repeat(32));
            const covenant = parsed(wasm.covenant_oracle_v1(key(0), key(1), oracleKey, '11'.repeat(32), 'the release happened', await daa() + 100_000n, network));
            const { signature } = await attested(covenant.message_commitment_hex);
            await fund(covenant.address, 'oracle-v1');
            return spendAndSee(await node(wsUrl => wasm.create_covenant_oracle_v1_claim(
                covenant.address, destination(), covenant.redeem_script_hex, oracleKey, signature, covenant.message_commitment_hex, FEE, wsUrl,
            )), destination());
        },
        'crowdfund swept at its goal': async () => {
            const { pk_hex: provingKey, vk_hex: verifyingKey } = parsed(wasm.zk_crowdfund_setup());
            const lock = await daa() + 100_000n;
            const covenant = parsed(wasm.covenant_crowdfund(key(0), destination(), KAS, lock, verifyingKey, network));
            const coin = await fund(covenant.address, 'crowdfund');
            const proof = parsed(wasm.zk_crowdfund_prove(provingKey, verifyingKey, JSON.stringify([String(coin.amount)])));
            const contribution = {
                address: covenant.address,
                contributor_pubkey_hex: key(0),
                redeem_script_hex: covenant.redeem_script_hex,
                crowdfund_salt_hex: covenant.crowdfund_salt_hex,
            };
            const txid = await node(wsUrl => wasm.create_crowdfund_sweep(
                JSON.stringify([contribution]), destination(), KAS, lock, verifyingKey, proof.proof_hex, proof.public_input_hex, FEE, wsUrl,
            ));
            await coinFrom(destination(), txid);
            return txid;
        },
        'private swap claimed by its claimant': async () => {
            const salt = crypto.getRandomValues(new Uint8Array(16)).reduce((text, byte) => text + byte.toString(16).padStart(2, '0'), '');
            const covenant = parsed(wasm.covenant_private_swap(key(0), key(1), destination(), await daa() + 100_000n, salt, network));
            const coin = await fund(covenant.address, 'private-swap');
            return spendAndSee(await wasm.create_private_swap_claim(
                covenant.address, destination(), covenant.redeem_script_hex, JSON.stringify([coin]), FEE,
            ), destination());
        },
        'whitelist paid to a listed address': async () => {
            const listed = wallet.receive_addresses.slice(3, 6);
            const root = parsed(wasm.merkle_root_from_addresses(JSON.stringify(listed)));
            const lock = await daa() + 100_000n;
            const covenant = parsed(wasm.covenant_merkle_whitelist(key(0), root.root, root.depth, lock, network));
            await fund(covenant.address, 'merkle-whitelist');
            const proof = parsed(wasm.merkle_proof_for_address(JSON.stringify(listed), destination()));
            return spendAndSee(await node(wsUrl => wasm.create_merkle_whitelist_spend(
                covenant.address, destination(), covenant.redeem_script_hex, JSON.stringify(proof.proof), KAS, FEE, wsUrl,
            )), destination());
        },
        'commitment revealed by its owner': async () => {
            const secret = crypto.getRandomValues(new Uint8Array(32)).reduce((text, byte) => text + byte.toString(16).padStart(2, '0'), '');
            const hash = await (await fetch('/__qa_funded_commitment__', { method: 'POST', body: secret })).text();
            const covenant = parsed(wasm.covenant_commit_reveal(key(0), hash, await daa() + 100_000n, network));
            await fund(covenant.address, 'commit-reveal');
            return spendAndSee(await node(wsUrl => wasm.create_commit_reveal_spend(JSON.stringify({
                covenant_address: covenant.address,
                dest_address: destination(),
                redeem_script_hex: covenant.redeem_script_hex,
                part_a_hex: secret,
                part_b_hex: '',
                payload_hex: `43523031${hash}`,
                fee: String(FEE),
                ws_url: wsUrl,
            }))), destination());
        },
        'payjoin claimed with a second input': async () => {
            const covenant = parsed(wasm.covenant_payjoin(key(0), key(1), await daa() + 100_000n, 2n, 2n, network));
            await fund(covenant.address, 'payjoin');
            return spendAndSee(await node(wsUrl => wasm.create_covenant_payjoin_claim(
                covenant.address, destination(), covenant.redeem_script_hex, destination(), FEE, wsUrl,
            )), destination());
        },
    };
}

async function shipment() {
    const now = await daa();
    const covenant = parsed(wasm.covenant_ship_escrow(JSON.stringify({
        seller_pubkey_hex: key(0),
        deliverer_pubkey_hex: key(1),
        buyer_pubkey_hex: key(2),
        arbiter_pubkey_hex: key(3),
        product_sompi: String(2n * KAS),
        fee_sompi: String(KAS),
        cltv1_deadline: String(now + LOCK_MARGIN),
        cltv2_deadline: String(now + 2n * LOCK_MARGIN),
        network,
    })));
    const terms = getShipParams(covenant.address, covenant.redeem_script_hex);
    await fund(covenant.address, 'ship-escrow', terms.total);
    return { covenant, terms };
}

async function shipStep({ covenant, terms }, branch) {
    const { pskbHex, plan } = await buildShipSpendPskb(covenant.address, branch, terms, covenant.redeem_script_hex);
    if (plan.locktime > 0n) await untilDaaPasses(plan.locktime);
    const [paid] = plan.outputs;
    return spendAndSee(pskbHex, paid.address);
}

function shipmentCases() {
    const inTransit = async () => {
        const held = await shipment();
        await shipStep(held, 'pickup');
        return held;
    };
    return {
        'shipment refunded by the arbiter before pickup': async () => shipStep(await shipment(), 'state0-arb-refund'),
        'shipment refunded by nobody after its first timeout': async () => shipStep(await shipment(), 'state0-timeout'),
        'shipment delivered': async () => shipStep(await inTransit(), 'delivery'),
        'shipment awarded by the arbiter in transit': async () => shipStep(await inTransit(), 'state1-arb-award'),
        'shipment paid out by nobody after its second timeout': async () => shipStep(await inTransit(), 'state1-timeout'),
        'shipment refunded by the arbiter in transit': async () => shipStep(await inTransit(), 'state1-arb-refund'),
    };
}

function privateSwapCases() {
    return {
        'private swap refunded after its lock': async () => {
            const lock = await daa() + LOCK_MARGIN;
            const salt = crypto.getRandomValues(new Uint8Array(16)).reduce((text, byte) => text + byte.toString(16).padStart(2, '0'), '');
            const covenant = parsed(wasm.covenant_private_swap(key(0), key(1), destination(), lock, salt, network));
            const coin = await fund(covenant.address, 'private-swap');
            await untilDaaPasses(lock);
            return spendAndSee(wasm.create_private_swap_refund(
                covenant.address, destination(), covenant.redeem_script_hex, JSON.stringify([coin]), FEE,
            ), destination());
        },
    };
}

async function run() {
    const input = await (await fetch('/__qa_funded_config__', { cache: 'no-store' })).json();
    await wasm.init();
    network = input.network;
    networkState.network = network;
    networkState.customNodeUrl = null;
    wallet = JSON.parse(wasm.import_kpub(input.kpub, network));
    if (wallet.receive_addresses.length <= ROLE_KEYS + 2) throw new Error('the wallet derived too few addresses');
    walletJson = JSON.stringify(wallet);
    const cases = {
        ...dmsCases(), ...savingsCases(), ...piggyCases(), ...threadCases(), ...timelockedRefundCases(), ...claimCases(),
        ...escrowCases(), ...shipmentCases(), ...privateSwapCases(),
    };
    const spends = [];
    for (const [name, spend] of Object.entries(cases)) {
        if (input.only && !name.includes(input.only)) continue;
        try {
            spends.push({ name, txid: await withinCaseLimit(spend) });
        } catch (error) {
            spends.push({ name, error: error instanceof Error ? error.message : String(error) });
        }
        const last = spends[spends.length - 1];
        await report(`${last.txid ? 'OK  ' : 'FAIL'} ${name} :: ${last.txid ?? last.error}`);
    }
    await finish('pass', { phase: 'covenants', network, spends });
}

try {
    await run();
} catch (error) {
    await finish('fail', { phase: 'covenants', message: error instanceof Error ? error.message : String(error) });
}
