import { applyScriptTerms } from '../../model/script_terms.js';
import { el, setContent, span } from '../../../../core/ui/elements.js';
import { covenantState, navigationState, networkState } from '../../../../app/state/index.js';
import { hideLoading, showLoading } from '../../../../app/navigation.js';
import { resolveNodeUrl } from '../../../../core/node/resolver.js';
import { toast } from '../../../../core/ui/toast.js';
import { getCovFee } from '../../payload/state.js';
import { openPsktReview } from '../../../transactions/pskt_multisig/review.js';
import { address_script_public_key, fetch_utxos_for_address_js, pskb_encode_document, ship_escrow_terms } from '../../../../wasm/api.js';
// KasKold Companion Web — features/covenants/spending/standard/shipment
import { byId } from '../../../../core/dom.js';
import { exactJsonStringify, exactUnsigned } from '../../../../core/exact.js';
import { sompiToKasString } from '../../../../core/amounts.js';


// ── Shipment-escrow covenant: parse params from redeem, refresh panel, spend ──

const SHIP_STATE0_BRANCHES = new Set(['pickup', 'state0-arb-refund', 'state0-timeout']);
const ESCROW_DISPUTE_ROLE_BYTES = new Map([['buyer-dispute', '01'], ['seller-dispute', '02']]);
const ESCROW_RELEASE_BRANCHES = new Set(['buyer-release', 'arbiter-award-seller']);

function shipParams(terms) {
    return {
        total: BigInt(terms.total_sompi), rem: BigInt(terms.rem_sompi), fee: BigInt(terms.fee_sompi),
        cltv1: BigInt(terms.cltv1_deadline), cltv2: BigInt(terms.cltv2_deadline),
        sellerAddr: terms.seller_addr, delivererAddr: terms.deliverer_addr, buyerAddr: terms.buyer_addr,
    };
}


// Prefer in-session metadata (exact, from create); otherwise Kaspa Portal reads
// the terms back from the redeem, so a second device needs only address + redeem.
function getShipParams(covAddr, redeemHex) {
    const L = covenantState.lastCovenantResult;
    if (L && L.type === 'ship-escrow' && L.address === covAddr && L.total_sompi != null) {
        return shipParams(L);
    }
    return shipParams(JSON.parse(ship_escrow_terms(redeemHex, networkState.network)));
}

async function fetchCovenantUtxos(covAddr) {
    const wsUrl = await resolveNodeUrl();
    return JSON.parse(await fetch_utxos_for_address_js(covAddr, wsUrl));
}

function mkOutput(amount, scriptPublicKey) {
    return { amount, scriptPublicKey, bip32Derivations: [], proprietaries: [] };
}

function mkShipOutput(amt, spk) {
    return mkOutput(exactUnsigned(amt, 'output sompi'), spk);
}

function setDisplay(element, shown) {
    if (element) element.style.display = shown ? '' : 'none';
}

function setStateText(stateEl, text) {
    if (stateEl) stateEl.textContent = text;
}

// ── Panel refresh ──

function fillIfBlank(id, value) {
    const input = byId(id);
    if (input && !input.value.trim()) input.value = value;
}

function prefillShipFields() {
    const L = covenantState.lastCovenantResult;
    if (!L || L.type !== 'ship-escrow') return;
    fillIfBlank('cov-ship-addr', L.address || '');
    fillIfBlank('cov-ship-script', L.redeem_script_hex || '');
}

function readShipPanelInputs() {
    const addrEl = byId('cov-ship-addr');
    const scriptEl = byId('cov-ship-script');
    return {
        covAddr: addrEl ? addrEl.value.trim() : '',
        redeemHex: scriptEl ? scriptEl.value.trim() : '',
    };
}

function renderShipState0(stateEl, s0, P) {
    setDisplay(s0, true);
    setContent(stateEl, span('u-text-teal', `State 0: funded (${sompiToKasString(P.total)} KAS), awaiting pickup.`), el('br'),
        span('u-text-11px-text-text-dim', `Pickup releases ${sompiToKasString(P.total - P.rem)} KAS to seller, continues at ${sompiToKasString(P.rem)} KAS.`));
}

function renderShipState1(stateEl, s1, P) {
    setDisplay(s1, true);
    setContent(stateEl, span('u-text-teal', `State 1: in transit (${sompiToKasString(P.rem)} KAS), awaiting delivery.`), el('br'),
        span('u-text-11px-text-text-dim', `Delivery pays deliverer ${sompiToKasString(P.fee)} KAS, the rest to seller.`));
}

function renderShipUnmatched(stateEl, amt, P) {
    setContent(stateEl, span('u-text-text-dim', `UTXO ${sompiToKasString(amt)} KAS matches neither state 0 (${sompiToKasString(P.total)}) nor state 1 (${sompiToKasString(P.rem)}).`));
}

function renderShipUtxoState(stateEl, s0, s1, amt, P) {
    if (amt === P.total) renderShipState0(stateEl, s0, P);
    else if (amt === P.rem) renderShipState1(stateEl, s1, P);
    else renderShipUnmatched(stateEl, amt, P);
}

async function loadShipPanelState(stateEl, s0, s1, covAddr, P) {
    try {
        const utxos = await fetchCovenantUtxos(covAddr);
        if (!utxos.length) {
            setContent(stateEl, span('u-text-text-dim', `Not funded. Fund with ${sompiToKasString(P.total)} KAS (product + fee) to start.`));
            return;
        }
        renderShipUtxoState(stateEl, s0, s1, BigInt(utxos[0].amount), P);
    } catch (e) { setStateText(stateEl, 'Error loading state: ' + e); }
}

export async function shipPanelRefresh() {
    const stateEl = byId('cov-ship-state');
    const s0 = byId('cov-ship-s0-actions'), s1 = byId('cov-ship-s1-actions');
    prefillShipFields();
    const { covAddr, redeemHex } = readShipPanelInputs();
    setDisplay(s0, false);
    setDisplay(s1, false);
    if (!covAddr || !redeemHex) { setStateText(stateEl, 'Enter covenant address and redeem script.'); return; }
    let P;
    try { P = getShipParams(covAddr, redeemHex); } catch (e) { setStateText(stateEl, 'Parse error: ' + e); return; }
    setStateText(stateEl, 'Loading state...');
    await loadShipPanelState(stateEl, s0, s1, covAddr, P);
}

// ── Ship-escrow spend ──

function shipPayoutsDerivable(P) {
    return Boolean(P.sellerAddr && P.delivererAddr && P.buyerAddr);
}

function readShipSpendParams(covAddr, redeemHex) {
    if (!covAddr) { toast('Enter covenant address', 'error'); return null; }
    if (!redeemHex) { toast('Enter redeem script hex', 'error'); return null; }
    let P;
    try { P = getShipParams(covAddr, redeemHex); } catch (e) { toast('Could not parse covenant: ' + e, 'error'); return null; }
    if (!shipPayoutsDerivable(P)) { toast('Could not derive payout addresses', 'error'); return null; }
    return P;
}

async function resolveShipUtxo(covAddr, expectAmt, isState0) {
    const utxos = await fetchCovenantUtxos(covAddr);
    if (!utxos.length) throw 'No UTXOs at covenant address';
    const u = utxos.find(x => BigInt(x.amount) === expectAmt) || utxos[0];
    const inAmt = BigInt(u.amount);
    if (inAmt !== expectAmt) {
        throw 'UTXO ' + sompiToKasString(inAmt) + ' KAS does not match the ' + (isState0 ? 'state-0 total' : 'state-1') + ' amount ' + sompiToKasString(expectAmt) + ' KAS for this branch';
    }
    return { u, inAmt };
}

function shipScriptPublicKeys(covAddr, P) {
    return {
        cov: '0000' + address_script_public_key(covAddr),
        seller: '0000' + address_script_public_key(P.sellerAddr),
        deliverer: '0000' + address_script_public_key(P.delivererAddr),
        buyer: '0000' + address_script_public_key(P.buyerAddr),
    };
}

function shipPlan(outputs, timelock) {
    return timelock === null
        ? { outputs, minSig: 1, locktime: 0n }
        : { outputs, minSig: 0, locktime: timelock };
}

function planPickup({ P, inAmt, txfee, spk }) {
    const sellerAmt = inAmt - P.rem - txfee;
    if (sellerAmt <= 0n) throw 'Fee too high for pickup';
    // out0 continues @ rem (exact)
    return shipPlan([mkShipOutput(P.rem, spk.cov), mkShipOutput(sellerAmt, spk.seller)], null);
}

function planDelivery({ P, inAmt, txfee, spk }, branch) {
    const sellerAmt = inAmt - P.fee - txfee;
    if (sellerAmt <= 0n) throw 'Fee too high for delivery';
    // out1 = deliverer fee (exact)
    const outputs = [mkShipOutput(sellerAmt, spk.seller), mkShipOutput(P.fee, spk.deliverer)];
    return shipPlan(outputs, branch === 'state1-timeout' ? exactUnsigned(P.cltv2, 'ship escrow CLTV2') : null);
}

function planRefund({ P, inAmt, txfee, spk }, branch) {
    const buyerAmt = inAmt - txfee;
    if (buyerAmt <= 0n) throw 'Fee too high for refund';
    const outputs = [mkShipOutput(buyerAmt, spk.buyer)];
    return shipPlan(outputs, branch === 'state0-timeout' ? exactUnsigned(P.cltv1, 'ship escrow CLTV1') : null);
}

const SHIP_BRANCH_PLANNERS = new Map([
    ['pickup', planPickup],
    ['delivery', planDelivery],
    ['state1-arb-award', planDelivery],
    ['state1-timeout', planDelivery],
    ['state0-arb-refund', planRefund],
    ['state0-timeout', planRefund],
    ['state1-arb-refund', planRefund],
]);

function planShipBranch(ctx, branch) {
    const planner = SHIP_BRANCH_PLANNERS.get(branch);
    if (!planner) throw 'Unknown branch ' + branch;
    return planner(ctx, branch);
}

function buildShipInput(u, inAmt, covSpkHex, redeemHex, minSig) {
    return {
        previousOutpoint: { transactionId: u.tx_id, index: u.index },
        // sigOpCount buys script compute budget on tx v1 (1 sigop = 10
        // budget units = 100K script units on top of the 9,999 free).
        // 0 here made every signed branch blow the free allowance:
        // "script units exceeded ... used=100763, limit=9999".
        sequence: 0, sigOpCount: minSig,
        utxoEntry: { amount: inAmt, scriptPublicKey: covSpkHex, blockDaaScore: 0n, isCoinbase: false },
        redeemScript: redeemHex, partialSigs: {}, minimumSignatures: minSig,
        bip32Derivations: [], proprietaries: {}, finalScriptSig: null, minTime: 0,
    };
}

function buildShipPskt(branch, { outputs, locktime }, inputs) {
    return {
        global: {
            txVersion: 1,
            fallbackLockTime: locktime > 0n ? locktime : null,
            inputsModifiableFlag: false, outputsModifiableFlag: false,
            inputCount: 1, outputCount: outputs.length,
            bip32Derivations: [],
            proprietaries: { shipBranch: branch },
        },
        inputs, outputs,
    };
}

async function buildShipSpendPskb(covAddr, branch, P, redeemHex) {
    const isState0 = SHIP_STATE0_BRANCHES.has(branch);
    const expectAmt = isState0 ? P.total : P.rem;
    const txfee = getCovFee();
    const { u, inAmt } = await resolveShipUtxo(covAddr, expectAmt, isState0);
    const spk = shipScriptPublicKeys(covAddr, P);
    const plan = planShipBranch({ P, inAmt, txfee, spk }, branch);
    const inputs = [buildShipInput(u, inAmt, spk.cov, redeemHex, plan.minSig)];
    const pskbHex = pskb_encode_document(exactJsonStringify(buildShipPskt(branch, plan, inputs)));
    return { pskbHex, plan };
}

export async function handleShipEscrowSpend(branch) {
    const covAddr = byId('cov-ship-addr').value.trim();
    const redeemHex = byId('cov-ship-script').value.trim();
    const P = readShipSpendParams(covAddr, redeemHex);
    if (!P) return;

    showLoading('Building ship-escrow ' + branch + ' TX...');
    try {
        const { pskbHex, plan } = await buildShipSpendPskb(covAddr, branch, P, redeemHex);
        hideLoading();
        console.log('[Companion] ShipEscrow ' + branch + ' PSKB: ' + pskbHex.length + ' hex chars, minSig=' + plan.minSig + ', locktime=' + plan.locktime);
        covenantState._covPayloadHex = '';
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Ship-escrow ' + branch + ' failed: ' + e, 'error', 5000);
        console.error('[Companion] Ship-escrow spend error:', e);
    }
}

// ── Escrow spend ──

function readEscrowContext() {
    const L = covenantState.lastCovenantResult;
    if (!L) { toast('No covenant loaded', 'error'); return null; }
    const covAddr = L.address;
    const redeemHex = L.redeem_script_hex;
    if (!covAddr || !redeemHex) { toast('Missing covenant data', 'error'); return null; }
    try {
        return { covAddr, redeemHex, terms: applyScriptTerms(L) };
    } catch (e) {
        toast('Escrow script rejected: ' + (e.message || e), 'error');
        return null;
    }
}

// Payout destinations are the addresses the script itself pays to.
function resolveEscrowDestination(branch, covAddr, terms) {
    if (ESCROW_DISPUTE_ROLE_BYTES.has(branch)) return covAddr;
    return ESCROW_RELEASE_BRANCHES.has(branch) ? terms.bob_address : terms.alice_address;
}

// Dispute heartbeat: attach "ESCD" + role payload so all watchers detect it.
function disputeHeartbeatPayload(branch) {
    const roleByte = ESCROW_DISPUTE_ROLE_BYTES.get(branch);
    return roleByte ? '4553434400' + roleByte : ''; // "ESCD\0" + role (6 bytes)
}

function buildEscrowInputs(utxos, covSpkHex, redeemHex) {
    return utxos.map(u => ({
        previousOutpoint: { transactionId: u.tx_id, index: u.index },
        // Every escrow branch verifies exactly one signature; sigOpCount 1
        // commits compute_budget 10 (109,999 units) on tx v1. 0 capped the
        // input at the 9,999 free units and the node rejected the spend.
        sequence: 0, sigOpCount: 1,
        utxoEntry: { amount: exactUnsigned(u.amount, 'escrow input sompi'), scriptPublicKey: covSpkHex, blockDaaScore: exactUnsigned(u.block_daa_score ?? 0n, 'escrow input DAA'), isCoinbase: false },
        redeemScript: redeemHex, partialSigs: {}, minimumSignatures: 1,
        bip32Derivations: [],
        proprietaries: {},
        finalScriptSig: null, minTime: 0,
    }));
}

function buildEscrowPskt(branch, inputs, outputs, txPayload) {
    // tx_version 1 for covenant introspection on TN10
    return {
        global: {
            txVersion: 1, fallbackLockTime: null,
            inputsModifiableFlag: false, outputsModifiableFlag: false,
            inputCount: inputs.length, outputCount: outputs.length,
            bip32Derivations: [],
            proprietaries: { escrowBranch: branch },
            txPayload: txPayload || undefined,
        },
        inputs, outputs,
    };
}

async function buildEscrowSpendPskb(branch, covAddr, redeemHex, destAddr) {
    const fee = getCovFee();
    const utxos = await fetchCovenantUtxos(covAddr);
    if (!utxos.length) throw 'No UTXOs at escrow address';

    const total = utxos.reduce((s, u) => s + BigInt(u.amount), 0n);
    if (total <= fee) throw 'Balance too low: ' + sompiToKasString(total) + ' KAS';

    const sendAmount = total - fee;
    const covSpkHex = '0000' + address_script_public_key(covAddr);
    const destSpkHex = '0000' + address_script_public_key(destAddr);
    const inputs = buildEscrowInputs(utxos, covSpkHex, redeemHex);
    const outputs = [mkOutput(sendAmount, destSpkHex)];
    return pskb_encode_document(exactJsonStringify(buildEscrowPskt(branch, inputs, outputs, disputeHeartbeatPayload(branch))));
}

export async function handleEscrowSpend(branch) {
    const ctx = readEscrowContext();
    if (!ctx) return;
    const destAddr = resolveEscrowDestination(branch, ctx.covAddr, ctx.terms);

    showLoading('Building escrow ' + branch + ' TX...');
    try {
        const pskbHex = await buildEscrowSpendPskb(branch, ctx.covAddr, ctx.redeemHex, destAddr);
        hideLoading();
        console.log('[Companion] Escrow ' + branch + ' PSKB: ' + pskbHex.length + ' hex chars, dest=' + destAddr);
        covenantState._covPayloadHex = ''; // Clear stale deposit payload
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Escrow spend failed: ' + e, 'error', 5000);
        console.error('[Companion] Escrow spend error:', e);
    }
}
