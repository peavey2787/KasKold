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
import { encodeCovenantSpend } from '/target/kaskold-companion-web/site/js/features/covenants/spending/standard/spend_document.js';

const KAS = 100_000_000n;
const FEE = 1_000_000n;
const FUNDING = 3n * KAS;
const POLL_MS = 2_000;
const COIN_TIMEOUT_MS = 120_000;
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
    const coins = JSON.parse(await node(wsUrl => wasm.fetch_utxos(walletJson, wsUrl)));
    const largest = coins.reduce((best, coin, index) => (
        BigInt(coin.amount) > BigInt(coins[best].amount) ? index : best
    ), 0);
    const document_ = await node(wsUrl => wasm.create_covenant_pskb(JSON.stringify({
        wallet_json: walletJson,
        covenant_address: address,
        covenant_type: covenantType,
        send_amount: String(amount),
        fee: String(FEE),
        change_address: wallet.change_addresses[0],
        utxo_indices_csv: String(largest),
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
        'dead-man-switch heir after inactivity': async () => {
            const covenant = parsed(wasm.covenant_dms(key(1), key(2), inactivity, network));
            const coin = await fund(covenant.address, 'dms');
            await untilDaaPasses(BigInt(coin.block_daa_score) + inactivity);
            return spendAndSee(await node(wsUrl => wasm.create_covenant_beneficiary_spend(
                covenant.address, destination(), covenant.redeem_script_hex, inactivity, FEE, wsUrl,
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
        'savings withdrawn in part, the change staying locked': async () => {
            const lock = await daa() + 100_000n;
            const covenant = parsed(wasm.covenant_timelocked_savings(key(0), key(1), lock, network));
            const coin = await fund(covenant.address, 'timelocked-savings');
            const change = BigInt(coin.amount) - KAS - FEE;
            const txid = await spendAndSee(encodeCovenantSpend({
                covAddr: covenant.address,
                redeemHex: covenant.redeem_script_hex,
                path: 'owner',
                utxos: [coin],
                outputs: [{ address: destination(), amount: String(KAS) }, { address: covenant.address, amount: String(change) }],
                txVersion: 0,
            }), destination());
            await coinFrom(covenant.address, txid);
            return spendAndSee(await node(wsUrl => wasm.create_covenant_owner_spend(
                covenant.address, destination(), covenant.redeem_script_hex, FEE, wsUrl, 'owner',
            )), destination());
        },
    };
}

function escrowCases() {
    const terms = { alice_address: wallet.receive_addresses[3], bob_address: wallet.receive_addresses[4] };
    const covenant = () => parsed(wasm.covenant_escrow(key(0), key(1), key(2), terms.alice_address, terms.bob_address, network));
    const spend = async branch => {
        const { address, redeem_script_hex: redeem } = covenant();
        const paidTo = resolveEscrowDestination(branch, address, terms);
        return spendAndSee(await buildEscrowSpendPskb(branch, address, redeem, paidTo), paidTo);
    };
    const cases = {};
    for (const branch of ESCROW_DISPUTES) {
        cases[`escrow ${branch}`] = async () => {
            await fund(covenant().address, 'escrow');
            return spend(branch);
        };
    }
    for (const branch of ESCROW_PAYOUTS) {
        cases[`escrow ${branch}`] = async () => {
            await fund(covenant().address, 'escrow');
            return spend(branch);
        };
    }
    return cases;
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
    const cases = { ...dmsCases(), ...savingsCases(), ...escrowCases(), ...shipmentCases(), ...privateSwapCases() };
    const spends = [];
    for (const [name, spend] of Object.entries(cases)) {
        if (input.only && !name.includes(input.only)) continue;
        try {
            spends.push({ name, txid: await spend() });
        } catch (error) {
            spends.push({ name, error: error instanceof Error ? error.message : String(error) });
        }
    }
    await finish('pass', { phase: 'covenants', network, spends });
}

try {
    await run();
} catch (error) {
    await finish('fail', { phase: 'covenants', message: error instanceof Error ? error.message : String(error) });
}
