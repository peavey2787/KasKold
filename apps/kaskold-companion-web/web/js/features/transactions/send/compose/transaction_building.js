// @ts-check
import { networkState, transactionState } from '../../boundary.js';
import { KNS_LOOKUP } from '../../../../core/config/services.js';
import { hideLoading, showLoading, showScreen } from '../../../../app/navigation.js';
import { toast } from '../../../../core/ui/toast.js';
import { stopScanner } from '../../../stealth/index/camera.js';
import { openPsktReview } from '../../pskt_multisig/review.js';
import { readFeeSompi, setFeeSompi, syncThreadDepositAmount } from './send_form.js';
import { reserveStandardChangeFromSummary, walletWithFreshIndices } from '../../../wallet/core.js';
// KasKold Companion Web — features/transactions/send/compose/transaction_building
import { inputById } from '../../../../core/dom.js';
import { kasToSompi } from '../../../../core/amounts.js';
import { exactUnsigned } from '../../../../core/exact.js';
import { addressPrefix } from '../../../../core/network.js';
import { planTransaction } from './planners/index.js';




function parseWalletForSendIntent(walletJson) {
    try { return JSON.parse(walletJson); } catch { throw new Error('Wallet state could not be verified'); }
}

function readSendFigures(summary, amountSompi, feeSompi) {
    const figures = {
        amount: exactUnsigned(amountSompi, 'send amount'),
        fee: exactUnsigned(feeSompi, 'send fee'),
        totalIn: exactUnsigned(summary?.total_in_sompi, 'transaction input total'),
        summaryFee: exactUnsigned(summary?.fee_sompi, 'transaction fee'),
    };
    if (figures.summaryFee !== figures.fee) throw new Error('Built transaction fee does not match the requested fee');
    return figures;
}

function outputBranch(output) {
    return output.derivation_branch === null || output.derivation_branch === undefined
        ? null : Number(output.derivation_branch);
}

function isDestinationPayment(output, destination, amount) {
    return output.address === destination
        && exactUnsigned(output.amount_sompi, 'destination amount') === amount
        && outputBranch(output) !== 1;
}

function assertDestinationPayment(outputs, destination, amount) {
    const destinationOutputs = outputs.filter(output => isDestinationPayment(output, destination, amount));
    if (destinationOutputs.length !== 1) {
        throw new Error('Built transaction does not contain the exact requested destination payment');
    }
}

function assertNoChangeOutputs(outputs, changeOutputs) {
    if (changeOutputs.length !== 0 || outputs.length !== 1) {
        throw new Error('Built transaction contains an unexpected change output');
    }
}

function isExactChangeOutput(output, changeIndex, changeAddress, expectedChange) {
    return Number(output.derivation_index) === changeIndex
        && output.address === changeAddress
        && exactUnsigned(output.amount_sompi, 'change amount') === expectedChange;
}

function assertExactChangeOutput(outputs, changeOutputs, wallet, expectedChange) {
    const changeIndex = Number(wallet?.next_change_index);
    const changeAddress = wallet?.change_addresses?.[changeIndex];
    if (!Number.isSafeInteger(changeIndex) || changeIndex < 0 || !changeAddress) {
        throw new Error('Expected change address is unavailable');
    }
    const exactChange = changeOutputs.filter(output =>
        isExactChangeOutput(output, changeIndex, changeAddress, expectedChange));
    if (exactChange.length !== 1 || changeOutputs.length !== 1 || outputs.length !== 2) {
        throw new Error('Built transaction change does not match the reserved wallet address');
    }
}

export function assertStandardSendIntent(summary, walletJson, destination, amountSompi, feeSompi) {
    const wallet = parseWalletForSendIntent(walletJson);
    const outputs = summary?.outputs || [];
    const { amount, fee, totalIn } = readSendFigures(summary, amountSompi, feeSompi);
    assertDestinationPayment(outputs, destination, amount);
    const expectedChange = totalIn - amount - fee;
    if (expectedChange < 0n) throw new Error('Built transaction spends more than its inputs');
    const changeOutputs = outputs.filter(output => Number(output.derivation_branch) === 1);
    if (expectedChange === 0n) {
        assertNoChangeOutputs(outputs, changeOutputs);
        return;
    }
    assertExactChangeOutput(outputs, changeOutputs, wallet, expectedChange);
}

// ─── Destination QR scan ───

export function handleDestScan(data) {
    const text = typeof data === 'string' ? data : new TextDecoder().decode(new Uint8Array(data));
    const addr = text.trim();
    const expectedPrefix = addressPrefix(networkState.network);
    if ((expectedPrefix && addr.startsWith(expectedPrefix)) || addr.endsWith('.kas')) {
        stopScanner();
        inputById('input-dest').value = addr;
        showScreen('send');
        toast('Address scanned', 'ok', 1500);
        return;
    }
    if (/^kaspa(test|dev|sim)?:/.test(addr)) {
        toast(`Address is for a different network. Expected ${expectedPrefix}`, 'error', 3500);
    }
}
// Number / Math.round(x*1e8) loses a sompi on many decimals and all precision
// above ~2^53 sompi. wasm-bindgen marshals the returned BigInt to a Rust u64.
function clearPendingPskt() {
    transactionState._psktReviewHex = null;
    transactionState._lastPsktSummary = null;
    showScreen('send');
}

function resolveSendDestination(rawDest) {
    let dest = rawDest;
    // KNS resolution: if ends with .kas, look up address
    if (dest.endsWith('.kas')) {
        const resolved = KNS_LOOKUP[dest.toLowerCase()];
        if (!resolved) {
            toast('Unknown .kas domain: ' + dest, 'error');
            return null;
        }
        dest = resolved;
        toast('Resolved ' + rawDest + ' → address', 'ok', 2000);
    }
    const expectedPrefix = addressPrefix(networkState.network);
    if (!dest || !dest.startsWith(expectedPrefix)) {
        toast('Enter a valid ' + expectedPrefix + ' address or .kas domain', 'error');
        return null;
    }
    return dest;
}

function parseSendAmount(amountStr) {
    let amountSompi;
    try { amountSompi = kasToSompi(amountStr); } catch {
        toast('Enter a valid amount with at most 8 decimal places', 'error');
        return null;
    }
    if (amountSompi <= 0n) {
        toast('Enter an amount > 0', 'error');
        return null;
    }
    return amountSompi;
}

function resolveSendFee() {
    let requestedFee;
    try { requestedFee = readFeeSompi(); } catch {
        toast('Enter a valid fee in the selected unit', 'error');
        return null;
    }
    const fee = requestedFee < 300000n ? 300000n : requestedFee;
    if (fee !== requestedFee) setFeeSompi(fee);
    return fee;
}

function assertStandardPlanSafe(summary, freshWallet, dest, amountSompi, fee) {
    try {
        assertStandardSendIntent(summary, freshWallet, dest, amountSompi, fee);
    } catch (error) {
        clearPendingPskt();
        throw new Error(`Transaction safety check failed: ${error.message || error}`, { cause: error });
    }
}

function reserveStandardChangeOrClear(summary, freshWallet, amountSompi, fee) {
    transactionState._standardChangeReservationIndex = reserveStandardChangeFromSummary(
        freshWallet, summary,
    );
    const needsChange = exactUnsigned(summary.total_in_sompi, 'transaction input total')
        - amountSompi - fee > 0n;
    if (needsChange && !Number.isSafeInteger(transactionState._standardChangeReservationIndex)) {
        clearPendingPskt();
        throw new Error('Transaction safety check failed: change could not be reserved');
    }
}

function reviewStandardPlan(plan, summary, freshWallet, dest, amountSompi, fee) {
    if (plan.kind !== 'standard' || !summary) return;
    assertStandardPlanSafe(summary, freshWallet, dest, amountSompi, fee);
    reserveStandardChangeOrClear(summary, freshWallet, amountSompi, fee);
}

// Number / Math.round(x*1e8) loses a sompi on many decimals and all precision
// above ~2^53 sompi. wasm-bindgen marshals the returned BigInt to a Rust u64.
export async function handleCreateTx() {
    const rawDest = inputById('input-dest').value.trim();
    // Thread-covenant deposit: amount field is hidden; ensure it reflects the
    // selected UTXO total before the >0 check (also covers any picker desync).
    syncThreadDepositAmount();
    const amountStr = inputById('input-amount').value.trim();

    const dest = resolveSendDestination(rawDest);
    if (dest === null) return;
    const amountSompi = parseSendAmount(amountStr);
    if (amountSompi === null) return;
    const fee = resolveSendFee();
    if (fee === null) return;

    showLoading('Creating transaction...');
    try {
        const freshWallet = walletWithFreshIndices();

        const plan = await planTransaction({
            destination: dest,
            amountString: amountStr,
            fee,
            freshWallet,
        });
        if (!plan || plan.completed) return;
        const pskbHex = plan.pskbHex;

        hideLoading();
        console.log(`[Companion] PSKB created: ${pskbHex.length} hex chars`);
        // Route through the existing PSKT review screen — same flow as
        // multisig: Review → Relay (standard PSKB or compact KSPT v1
        // for KasKold) → Finalize & Broadcast.
        const summary = openPsktReview(pskbHex);
        reviewStandardPlan(plan, summary, freshWallet, dest, amountSompi, fee);
    } catch (e) {
        hideLoading();
        toast('TX creation failed: ' + e, 'error', 5000);
        console.error('TX creation failed:', e);
    }
}
