import { networkState, walletSession } from '../../../../app/state/index.js';
import { toast } from '../../../../core/ui/toast.js';
import { covenant_escrow, covenant_ship_escrow, encode_p2pk_address } from '../../../../wasm/api.js';
// escrow covenant builders.

import { byId } from '../../../../core/dom.js';
import { addressToXOnly } from '../../../../core/address.js';
import { kasToSompi } from '../../../../core/amounts.js';
export async function buildEscrow(ownerPk) {
    let resultJson;
    let extra = {};
    const theirPk = addressToXOnly(byId('cov-escrow-pk').value);
    const arbiterPk = addressToXOnly(byId('cov-escrow-arbiter-pk').value);
    if (!ownerPk) { toast('Load wallet first', 'error'); return; }
    if (!theirPk || theirPk.length !== 64) { toast('Enter seller pubkey (64 hex chars)', 'error'); return; }
    if (!arbiterPk || arbiterPk.length !== 64) { toast('Enter arbiter pubkey (64 hex chars)', 'error'); return; }
    // Derive addresses from pubkeys. Use /0/0 receive address for buyer (matches wallet tracking).
    const w = walletSession.current();
    const myAddr = w.receive_addresses && w.receive_addresses[0] ? w.receive_addresses[0] : encode_p2pk_address(ownerPk, networkState.network);
    const theirAddr = encode_p2pk_address(theirPk, networkState.network);
    resultJson = covenant_escrow(ownerPk, theirPk, arbiterPk, myAddr, theirAddr, networkState.network);
    extra.bob_pk = theirPk;
    extra.arbiter_pk = arbiterPk;
    return { resultJson, extra };
}

function isHex64(value) {
    return /^[0-9a-fA-F]{64}$/.test(value);
}

function validateShipPubkeys(ownerPk, sellerPk, delivPk, arbPk) {
    if (!ownerPk) { toast('Load wallet first (you are the buyer)', 'error'); return false; }
    if (!isHex64(sellerPk)) { toast('Enter seller pubkey (64 hex chars)', 'error'); return false; }
    if (!isHex64(delivPk)) { toast('Enter deliverer pubkey (64 hex chars)', 'error'); return false; }
    if (!isHex64(arbPk)) { toast('Enter arbiter pubkey (64 hex chars)', 'error'); return false; }
    return true;
}

// Reads product price and delivery fee in sompi. Returns null after toasting on failure.
function readShipAmounts() {
    let productSompi, feeSompi;
    try { productSompi = kasToSompi(byId('cov-ship-product').value.trim()); } catch { toast('Enter product price in KAS (up to 8 decimals)', 'error'); return null; }
    try { feeSompi = kasToSompi(byId('cov-ship-fee').value.trim()); } catch { toast('Enter delivery fee in KAS (up to 8 decimals)', 'error'); return null; }
    if (productSompi <= 0n) { toast('Enter product price in KAS', 'error'); return null; }
    if (feeSompi <= 0n) { toast('Enter delivery fee in KAS', 'error'); return null; }
    return { productSompi, feeSompi };
}

// Reads both DAA deadlines. Returns null after toasting when either is unset.
function readShipDeadlines() {
    const cltv1 = BigInt(byId('cov-ship-cltv1').value.trim() || '0');
    const cltv2 = BigInt(byId('cov-ship-cltv2').value.trim() || '0');
    if (cltv1 <= 0n || cltv2 <= 0n) { toast('Set both deadlines (DAA score)', 'error'); return null; }
    return { cltv1, cltv2 };
}

function buildShipPayload(pubkeys, amounts, deadlines) {
    return JSON.stringify({
        seller_pubkey_hex: pubkeys.seller,
        deliverer_pubkey_hex: pubkeys.deliverer,
        buyer_pubkey_hex: pubkeys.buyer,
        arbiter_pubkey_hex: pubkeys.arbiter,
        product_sompi: amounts.productSompi.toString(),
        fee_sompi: amounts.feeSompi.toString(),
        cltv1_deadline: deadlines.cltv1.toString(),
        cltv2_deadline: deadlines.cltv2.toString(),
        network: networkState.network,
    });
}

function shipEscrowExtra(pubkeys) {
    const extra = {};
    extra.seller_pk = pubkeys.seller;
    extra.deliverer_pk = pubkeys.deliverer;
    extra.buyer_pk = pubkeys.buyer;
    extra.arbiter_pk = pubkeys.arbiter;
    extra.seller_addr = encode_p2pk_address(pubkeys.seller, networkState.network);
    extra.deliverer_addr = encode_p2pk_address(pubkeys.deliverer, networkState.network);
    extra.buyer_addr = encode_p2pk_address(pubkeys.buyer, networkState.network);
    return extra;
}

export async function buildShipEscrow(ownerPk) {
    const pubkeys = {
        seller: addressToXOnly(byId('cov-ship-seller-pk').value),
        deliverer: addressToXOnly(byId('cov-ship-deliverer-pk').value),
        buyer: ownerPk,
        arbiter: addressToXOnly(byId('cov-ship-arbiter-pk').value),
    };
    if (!validateShipPubkeys(ownerPk, pubkeys.seller, pubkeys.deliverer, pubkeys.arbiter)) return;
    const amounts = readShipAmounts();
    if (!amounts) return;
    const deadlines = readShipDeadlines();
    if (!deadlines) return;
    const resultJson = covenant_ship_escrow(buildShipPayload(pubkeys, amounts, deadlines));
    return { resultJson, extra: shipEscrowExtra(pubkeys) };
}
