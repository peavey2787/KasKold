import { el, setContent, span } from '../../../../core/ui/elements.js';
import { covenantState, navigationState, networkState } from '../../../../app/state/index.js';
import { hideLoading, showLoading } from '../../../../app/navigation.js';
import { resolveNodeUrl } from '../../../../core/node/resolver.js';
import { toast } from '../../../../core/ui/toast.js';
import { getCovFee } from '../../payload_and_swaps/state.js';
import { ensureEscrowParams } from '../../watchers_and_ui/ui/metadata.js';
import { openPsktReview } from '../../../transactions/pskt_multisig/review.js';
import { address_script_public_key, encode_p2pk_address, fetch_utxos_for_address_js, pskb_encode_document, ship_escrow_terms } from '../../../../wasm/api.js';
// KasKold Companion Web — features/covenants/spending/standard/shipment
import { byId } from '../../../../core/dom.js';
import { exactJsonStringify, exactUnsigned } from '../../../../core/exact.js';
import { sompiToKasString } from '../../../../core/amounts.js';


// ── Shipment-escrow covenant: parse params from redeem, refresh panel, spend ──

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


export async function shipPanelRefresh() {
    const stateEl = byId('cov-ship-state');
    const s0 = byId('cov-ship-s0-actions'), s1 = byId('cov-ship-s1-actions');
    if (covenantState.lastCovenantResult && covenantState.lastCovenantResult.type === 'ship-escrow') {
        if (byId('cov-ship-addr') && !byId('cov-ship-addr').value.trim()) byId('cov-ship-addr').value = covenantState.lastCovenantResult.address || '';
        if (byId('cov-ship-script') && !byId('cov-ship-script').value.trim()) byId('cov-ship-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
    }
    const covAddr = byId('cov-ship-addr') ? byId('cov-ship-addr').value.trim() : '';
    const redeemHex = byId('cov-ship-script') ? byId('cov-ship-script').value.trim() : '';
    if (s0) s0.style.display = 'none';
    if (s1) s1.style.display = 'none';
    if (!covAddr || !redeemHex) { if (stateEl) stateEl.textContent = 'Enter covenant address and redeem script.'; return; }
    let P;
    try { P = getShipParams(covAddr, redeemHex); } catch (e) { if (stateEl) stateEl.textContent = 'Parse error: ' + e; return; }
    if (stateEl) stateEl.textContent = 'Loading state...';
    try {
        const wsUrl = await resolveNodeUrl();
        const utxos = JSON.parse(await fetch_utxos_for_address_js(covAddr, wsUrl));
        const fmt = (s) => sompiToKasString(s);
        if (!utxos.length) {
            setContent(stateEl, span('u-text-text-dim', `Not funded. Fund with ${fmt(P.total)} KAS (product + fee) to start.`));
            return;
        }
        const amt = BigInt(utxos[0].amount);
        if (amt === P.total) {
            if (s0) s0.style.display = '';
            setContent(stateEl, span('u-text-teal', `State 0: funded (${fmt(P.total)} KAS), awaiting pickup.`), el('br'),
                span('u-text-11px-text-text-dim', `Pickup releases ${fmt(P.total - P.rem)} KAS to seller, continues at ${fmt(P.rem)} KAS.`));
        } else if (amt === P.rem) {
            if (s1) s1.style.display = '';
            setContent(stateEl, span('u-text-teal', `State 1: in transit (${fmt(P.rem)} KAS), awaiting delivery.`), el('br'),
                span('u-text-11px-text-text-dim', `Delivery pays deliverer ${fmt(P.fee)} KAS, the rest to seller.`));
        } else {
            setContent(stateEl, span('u-text-text-dim', `UTXO ${fmt(amt)} KAS matches neither state 0 (${fmt(P.total)}) nor state 1 (${fmt(P.rem)}).`));
        }
    } catch (e) { if (stateEl) stateEl.textContent = 'Error loading state: ' + e; }
}
export async function handleShipEscrowSpend(branch) {
    const covAddr = byId('cov-ship-addr').value.trim();
    const redeemHex = byId('cov-ship-script').value.trim();
    if (!covAddr) { toast('Enter covenant address', 'error'); return; }
    if (!redeemHex) { toast('Enter redeem script hex', 'error'); return; }
    let P;
    try { P = getShipParams(covAddr, redeemHex); } catch (e) { toast('Could not parse covenant: ' + e, 'error'); return; }
    if (!P.sellerAddr || !P.delivererAddr || !P.buyerAddr) { toast('Could not derive payout addresses', 'error'); return; }

    const isState0 = (branch === 'pickup' || branch === 'state0-arb-refund' || branch === 'state0-timeout');
    const expectAmt = isState0 ? P.total : P.rem;

    showLoading('Building ship-escrow ' + branch + ' TX...');
    try {
        const txfee = getCovFee();
        const wsUrl = await resolveNodeUrl();
        const utxos = JSON.parse(await fetch_utxos_for_address_js(covAddr, wsUrl));
        if (!utxos.length) throw 'No UTXOs at covenant address';
        const u = utxos.find(x => BigInt(x.amount) === expectAmt) || utxos[0];
        const inAmt = BigInt(u.amount);
        if (inAmt !== expectAmt) {
            throw 'UTXO ' + sompiToKasString(inAmt) + ' KAS does not match the ' + (isState0 ? 'state-0 total' : 'state-1') + ' amount ' + sompiToKasString(expectAmt) + ' KAS for this branch';
        }
        const covSpkHex = '0000' + address_script_public_key(covAddr);
        const sellerSpk = '0000' + address_script_public_key(P.sellerAddr);
        const delivSpk = '0000' + address_script_public_key(P.delivererAddr);
        const buyerSpk = '0000' + address_script_public_key(P.buyerAddr);
        const mkOut = (amt, spk) => ({ amount: exactUnsigned(amt, 'output sompi'), scriptPublicKey: spk, bip32Derivations: [], proprietaries: [] });

        let outputs, minSig = 1, locktime = 0n;
        if (branch === 'pickup') {
            const sellerAmt = inAmt - P.rem - txfee;
            if (sellerAmt <= 0n) throw 'Fee too high for pickup';
            outputs = [mkOut(P.rem, covSpkHex), mkOut(sellerAmt, sellerSpk)]; // out0 continues @ rem (exact)
        } else if (branch === 'delivery' || branch === 'state1-arb-award' || branch === 'state1-timeout') {
            const sellerAmt = inAmt - P.fee - txfee;
            if (sellerAmt <= 0n) throw 'Fee too high for delivery';
            outputs = [mkOut(sellerAmt, sellerSpk), mkOut(P.fee, delivSpk)]; // out1 = deliverer fee (exact)
            if (branch === 'state1-timeout') { minSig = 0; locktime = exactUnsigned(P.cltv2, 'ship escrow CLTV2'); }
        } else if (branch === 'state0-arb-refund' || branch === 'state0-timeout' || branch === 'state1-arb-refund') {
            const buyerAmt = inAmt - txfee;
            if (buyerAmt <= 0n) throw 'Fee too high for refund';
            outputs = [mkOut(buyerAmt, buyerSpk)];
            if (branch === 'state0-timeout') { minSig = 0; locktime = exactUnsigned(P.cltv1, 'ship escrow CLTV1'); }
        } else {
            throw 'Unknown branch ' + branch;
        }

        const inputs = [{
            previousOutpoint: { transactionId: u.tx_id, index: u.index },
            // sigOpCount buys script compute budget on tx v1 (1 sigop = 10
            // budget units = 100K script units on top of the 9,999 free).
            // 0 here made every signed branch blow the free allowance:
            // "script units exceeded ... used=100763, limit=9999".
            sequence: 0, sigOpCount: minSig,
            utxoEntry: { amount: inAmt, scriptPublicKey: covSpkHex, blockDaaScore: 0n, isCoinbase: false },
            redeemScript: redeemHex, partialSigs: {}, minimumSignatures: minSig,
            bip32Derivations: [], proprietaries: {}, finalScriptSig: null, minTime: 0
        }];

        const pskt = {
            global: {
                txVersion: 1,
                fallbackLockTime: locktime > 0n ? locktime : null,
                inputsModifiableFlag: false, outputsModifiableFlag: false,
                inputCount: 1, outputCount: outputs.length,
                bip32Derivations: [],
                proprietaries: { shipBranch: branch }
            },
            inputs, outputs
        };

        const pskbHex = pskb_encode_document(exactJsonStringify(pskt));

        hideLoading();
        console.log('[Companion] ShipEscrow ' + branch + ' PSKB: ' + pskbHex.length + ' hex chars, minSig=' + minSig + ', locktime=' + locktime);
        covenantState._covPayloadHex = '';
        navigationState._broadcastReturnScreen = 'covenant';
        openPsktReview(pskbHex);
    } catch (e) {
        hideLoading();
        toast('Ship-escrow ' + branch + ' failed: ' + e, 'error', 5000);
        console.error('[Companion] Ship-escrow spend error:', e);
    }
}
export async function handleEscrowSpend(branch) {
    if (!covenantState.lastCovenantResult) { toast('No covenant loaded', 'error'); return; }
    const covAddr = covenantState.lastCovenantResult.address;
    const redeemHex = covenantState.lastCovenantResult.redeem_script_hex;
    if (!covAddr || !redeemHex) { toast('Missing covenant data', 'error'); return; }

    // Parse script to get destination addresses
    ensureEscrowParams(covenantState.lastCovenantResult);
    const alicePk = covenantState.lastCovenantResult.alice_spk_hex || covenantState.lastCovenantResult.alice_pk;
    const bobPk = covenantState.lastCovenantResult.bob_spk_hex || covenantState.lastCovenantResult.bob_pk;
    if (!alicePk || !bobPk) { toast('Could not parse escrow destinations from script', 'error'); return; }

    // Determine destination based on branch
    let destAddr;
    const isDispute = (branch === 'buyer-dispute' || branch === 'seller-dispute');
    if (isDispute) {
        destAddr = covAddr; // send back to same escrow address
    } else if (branch === 'buyer-release' || branch === 'arbiter-award-seller') {
        destAddr = encode_p2pk_address(bobPk, networkState.network); // funds go to seller
    } else {
        destAddr = encode_p2pk_address(alicePk, networkState.network); // funds go to buyer
    }

    showLoading('Building escrow ' + branch + ' TX...');
    try {
        const fee = getCovFee();
        const wsUrl = await resolveNodeUrl();
        const utxosJson = await fetch_utxos_for_address_js(covAddr, wsUrl);
        const utxos = JSON.parse(utxosJson);
        if (!utxos.length) throw 'No UTXOs at escrow address';

        const total = utxos.reduce((s, u) => s + BigInt(u.amount), 0n);
        if (total <= fee) throw 'Balance too low: ' + sompiToKasString(total) + ' KAS';

        const sendAmount = total - fee;
        const covSpkHex = '0000' + address_script_public_key(covAddr);
        const destSpkHex = '0000' + address_script_public_key(destAddr);

        const inputs = utxos.map(u => ({
            previousOutpoint: { transactionId: u.tx_id, index: u.index },
            // Every escrow branch verifies exactly one signature; sigOpCount 1
            // commits compute_budget 10 (109,999 units) on tx v1. 0 capped the
            // input at the 9,999 free units and the node rejected the spend.
            sequence: 0, sigOpCount: 1,
            utxoEntry: { amount: exactUnsigned(u.amount, 'escrow input sompi'), scriptPublicKey: covSpkHex, blockDaaScore: exactUnsigned(u.block_daa_score ?? 0n, 'escrow input DAA'), isCoinbase: false },
            redeemScript: redeemHex, partialSigs: {}, minimumSignatures: 1,
            bip32Derivations: [],
            proprietaries: {},
            finalScriptSig: null, minTime: 0
        }));

        const outputs = [{ amount: sendAmount, scriptPublicKey: destSpkHex, bip32Derivations: [], proprietaries: [] }];

        // Dispute heartbeat: attach "ESCD" + role payload so all watchers detect it
        let txPayload = '';
        if (isDispute) {
            const roleByte = (branch === 'buyer-dispute') ? '01' : '02';
            txPayload = '4553434400' + roleByte; // "ESCD\0" + role (6 bytes)
        }

        // tx_version 1 for covenant introspection on TN10
        const pskt = {
            global: {
                txVersion: 1, fallbackLockTime: null,
                inputsModifiableFlag: false, outputsModifiableFlag: false,
                inputCount: inputs.length, outputCount: outputs.length,
                bip32Derivations: [],
                proprietaries: { escrowBranch: branch },
                txPayload: txPayload || undefined
            },
            inputs, outputs
        };

        const pskbHex = pskb_encode_document(exactJsonStringify(pskt));

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
