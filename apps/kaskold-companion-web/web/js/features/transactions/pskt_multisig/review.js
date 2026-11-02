import { el, span } from '../../../core/ui/elements.js';
import {
    covenantState,
    networkState,
    oracleState,
    transactionState,
    walletSession,
    pskt_summary,
} from '../boundary.js';
import { showScreen } from '../../../app/navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { sompiToKasString } from '../../../core/amounts.js';
import { exactUnsigned } from '../../../core/exact.js';
// KasKold Companion Web — features/transactions/pskt_multisig/review
import { bytesToHex, hexToBytes } from '../../../core/bytes.js';
import { byId } from '../../../core/dom.js';
import { createPsktReviewActions } from './review_actions.js';
import { releasePendingStandardChange, standardChangeReservationMatchesSummary } from '../../wallet/core.js';

import { addressHighlightParts, shortenHex } from '../../../core/format.js';

export const {
    openRelayModal,
    closeRelayModal,
    handlePsktRelay,
    handlePsktRelayKasKoldStandard,
    handlePsktRelayCompact,
    handlePsktFinalize,
} = createPsktReviewActions();

// ─── PSKT / PSKB Review ───
//
// When a scan or paste yields a PSKB/PSKT envelope, we open a review
// screen showing inputs, outputs, fee, and multisig progress (M/N).
// From there the user can:
//   - Relay to next signer  (re-emit identical QR for the next device)
//   - Finalize + broadcast  (when all inputs meet their sig threshold)

// Stash the hex for the current review so both buttons can access it
// without re-parsing.
transactionState._psktReviewHex = null;

transactionState._lastPsktSummary = null;
transactionState._psktReviewContext = null;

function psktBodyKey(summary) {
    return JSON.stringify({
        txVersion: summary.tx_version,
        inputs: (summary.inputs || []).map(input => [
            input.prev_tx_id,
            input.prev_index,
            String(input.amount_sompi),
            input.script_hex || '',
        ]),
        outputs: (summary.outputs || []).map(output => [
            String(output.amount_sompi),
            output.script_hex || '',
            output.address || '',
        ]),
    });
}

function resolveReviewContext(summary, requestedContext) {
    const bodyKey = psktBodyKey(summary);
    if (requestedContext) {
        const context = { ...requestedContext, bodyKey };
        transactionState._psktReviewContext = context;
        return context;
    }
    const context = transactionState._psktReviewContext;
    if (context?.bodyKey === bodyKey) return context;
    transactionState._psktReviewContext = null;
    return null;
}

function classifyMultisigOutput(address, reviewContext) {
    if (address && address === reviewContext.destinationAddress) return 'DESTINATION';
    return 'MULTISIG CHANGE';
}

function classifyOrdinaryWalletOutput(address) {
    if (!address || !walletSession.hasWallet()) return 'DESTINATION';
    const wallet = walletSession.current();
    if (wallet.change_addresses?.includes(address)) return 'CHANGE';
    if (wallet.receive_addresses?.includes(address)) return 'OWN RECEIVE';
    return 'DESTINATION';
}

function classifyWalletOutput(output, reviewContext) {
    const address = output?.address || '';
    if (reviewContext?.kind === 'multisig-send') return classifyMultisigOutput(address, reviewContext);
    return classifyOrdinaryWalletOutput(address);
}

function reviewOwnershipTotals(outputs, reviewContext) {
    let external = 0n;
    let change = 0n;
    let ownReceive = 0n;
    for (const output of outputs) {
        const amount = exactUnsigned(output.amount_sompi, 'review output amount');
        const ownership = classifyWalletOutput(output, reviewContext);
        if (ownership === 'CHANGE' || ownership === 'MULTISIG CHANGE') change += amount;
        else if (ownership === 'OWN RECEIVE') ownReceive += amount;
        else external += amount;
    }
    return { external, change, ownReceive };
}

function primePsktReviewState(wireHex) {
    transactionState._psktReviewHex = wireHex;
    // A newly opened canonical PSKT/PSKB has no device-returned relay payload yet.
    // The signed-scan merge path sets this immediately after a successful merge.
    transactionState._lastKasKoldKsptHex = null;
    oracleState._oracleMbRollActive = false;   // cleared on every load; the oracle path re-arms it right after this call
    oracleState._oracleMbReturn = false;       // clear any stale "return to oracle card" flag; only a roll's success path re-sets it
}

function parsePsktSummary(wireHex) {
    try {
        return JSON.parse(pskt_summary(wireHex, networkState.network));
    } catch (e) {
        console.error('[Companion] PSKT parse error:', e);
        toast('Could not parse PSKT: ' + e, 'error', 5000);
        return null;
    }
}

function releaseStaleStandardChangeReservation(summary) {
    const reservedIndex = transactionState._standardChangeReservationIndex;
    if (Number.isSafeInteger(reservedIndex)
        && !standardChangeReservationMatchesSummary(reservedIndex, summary)) {
        releasePendingStandardChange(reservedIndex);
        transactionState._standardChangeReservationIndex = null;
    }
}

function renderPsktHeader(summary) {
    byId('pskt-format').textContent = summary.format.toUpperCase();
    byId('pskt-tx-version').textContent = summary.tx_version;
    byId('pskt-in-count').textContent = summary.input_count;
    byId('pskt-out-count').textContent = summary.output_count;
    byId('pskt-fee').textContent = sompiToKasString(summary.fee_sompi);
    byId('pskt-total-in').textContent = sompiToKasString(summary.total_in_sompi);
    byId('pskt-total-out').textContent = sompiToKasString(summary.total_out_sompi);
}

function renderOwnershipTotals(outputs, reviewContext) {
    const ownershipTotals = reviewOwnershipTotals(outputs, reviewContext);
    byId('pskt-send-total').textContent = sompiToKasString(ownershipTotals.external);
    byId('pskt-change-total').textContent = sompiToKasString(ownershipTotals.change);
}

function externalDestinationText(externalOutputs) {
    if (externalOutputs.length === 1 && externalOutputs[0].address) return externalOutputs[0].address;
    if (externalOutputs.length > 1) return `${externalOutputs.length} destinations — Inspect to view`;
    return 'No external address — Inspect to view';
}

function renderPsktToAddress(outputs, reviewContext) {
    const externalOutputs = outputs.filter(output => classifyWalletOutput(output, reviewContext) === 'DESTINATION');
    byId('pskt-to-address').textContent = externalDestinationText(externalOutputs);
}

function wireInspectToggle() {
    const inspectButton = byId('btn-pskt-inspect');
    const inspectDetails = byId('pskt-inspect-details');
    inspectDetails.classList.add('hidden');
    inspectButton.textContent = 'Inspect';
    inspectButton.setAttribute('aria-expanded', 'false');
    inspectButton.onclick = () => {
        const opening = inspectDetails.classList.contains('hidden');
        inspectDetails.classList.toggle('hidden', !opening);
        inspectButton.textContent = opening ? 'Hide details' : 'Inspect';
        inspectButton.setAttribute('aria-expanded', opening ? 'true' : 'false');
    };
}

// Payload verification hash: if covenant payload exists, show SHA-256[..8]
// User compares this with KasKold's "PL xxxxxxxx" on its review screen.
function renderPayloadHash() {
    const plHashEl = byId('pskt-payload-hash');
    if (!plHashEl) return;
    if (covenantState._covPayloadHex && covenantState._covPayloadHex.length > 0) {
        const plBytes = hexToBytes(covenantState._covPayloadHex);
        crypto.subtle.digest('SHA-256', plBytes.buffer).then(hashBuf => {
            const h = bytesToHex(new Uint8Array(hashBuf).slice(0, 8));
            plHashEl.textContent = 'PL ' + h;
            plHashEl.style.display = '';
        });
    } else {
        plHashEl.textContent = '';
        plHashEl.style.display = 'none';
    }
}

function inputNeedsSignature(input) {
    const required = input.multisig_m !== null && input.multisig_m !== undefined
        ? Number(input.multisig_m)
        : 1;
    return Number(input.sigs_present || 0) < required;
}

function inputIsMultisigRelay(input) {
    return input.multisig_m !== null
        && input.multisig_m !== undefined
        && Number(input.multisig_n || 0) > 1;
}

// Present one clear next action. An unsigned ordinary PSKB needs to be sent
// to a signer; a partially signed multisig envelope needs to be relayed to
// its next signer. Once ready, the signing action disappears and broadcast
// becomes the primary action.
function renderPsktNextAction(summary) {
    const needsMultisigRelay = (summary.inputs || [])
        .filter(inputNeedsSignature)
        .some(inputIsMultisigRelay);
    const btnFinalize = byId('btn-pskt-finalize');
    const relayButton = byId('btn-pskt-relay');
    btnFinalize.disabled = false;
    btnFinalize.textContent = 'Finalize + broadcast';
    btnFinalize.classList.toggle('hidden', !summary.finalize_ready);

    relayButton.textContent = needsMultisigRelay ? 'Relay to next signer' : 'Send to signer';
    relayButton.classList.toggle('hidden', summary.finalize_ready);

    if (summary.finalize_ready) closeRelayModal();
}

export function openPsktReview(wireHex, requestedContext = null) {
    primePsktReviewState(wireHex);
    const summary = parsePsktSummary(wireHex);
    if (!summary) return;

    console.log('[Companion] PSKT summary:', summary);
    releaseStaleStandardChangeReservation(summary);
    transactionState._lastPsktSummary = summary;
    const reviewContext = resolveReviewContext(summary, requestedContext);

    renderPsktHeader(summary);
    const outputs = summary.outputs || [];
    renderOwnershipTotals(outputs, reviewContext);
    renderPsktToAddress(outputs, reviewContext);
    wireInspectToggle();
    renderPayloadHash();

    byId('pskt-inputs').replaceChildren(...summary.inputs.map(inputRow));
    byId('pskt-outputs').replaceChildren(...summary.outputs.map((out, index) => outputRow(out, index, reviewContext)));

    renderPsktNextAction(summary);

    showScreen('pskt-review');
    return summary;
}
/// PSKB (any wallet) or compact KSPT v1 (KasKold devices only).

function signatureBadge(input) {
    const multisig = input.multisig_m !== null && input.multisig_m !== undefined;
    const ok = input.sigs_present >= (multisig ? input.multisig_m : 1);
    const label = multisig
        ? `${input.sigs_present}/${input.multisig_m}-of-${input.multisig_n}`
        : `${input.sigs_present} sig${input.sigs_present === 1 ? '' : 's'}`;
    return span(`pskt-sig-badge${ok ? ' ok' : ''}`, label);
}

function reviewRow(index, kind, badge, details) {
    return el('div', { className: 'pskt-row' },
        el('div', { className: 'pskt-row-head' }, span('pskt-idx', `#${index}`), span('pskt-kind', kind), badge),
        el('div', { className: 'pskt-row-body' }, details.flatMap(([label, value, mono]) => [
            el('div', { className: 'pskt-label' }, label),
            el('div', { className: mono ? 'pskt-value pskt-mono' : 'pskt-value' }, value),
        ])));
}

function inputRow(input, index) {
    return reviewRow(index, input.script_kind.toUpperCase(), signatureBadge(input), [
        ['Amount', `${sompiToKasString(input.amount_sompi)} KAS`],
        ['Prev TX', `${shortenHex(input.prev_tx_id)}:${input.prev_index}`, true],
    ]);
}

function highlightedAddress(address) {
    if (!address) return '(unrecognized script)';
    const parts = addressHighlightParts(address);
    if ('plain' in parts) return parts.plain;
    return [parts.prefix, span('addr-hl', parts.head), parts.middle, span('addr-hl', parts.tail)];
}

function outputRow(output, index, reviewContext) {
    const ownership = classifyWalletOutput(output, reviewContext);
    return reviewRow(index, `${ownership} · ${output.script_kind.toUpperCase()}`, null, [
        ['Amount', `${sompiToKasString(output.amount_sompi)} KAS`],
        ['To', highlightedAddress(output.address), true],
    ]);
}
