// @ts-check
import { byId, elementById } from '../../../core/dom.js';
import { showStaticPayloadQr } from '../../qr_display/static_payload.js';

export function showPrivateSwapProtocolQr(payload, title, info) {
    showStaticPayloadQr(payload, title, info || '');
}

export function showPrivateSwapJsonQr(value, title, info) {
    showPrivateSwapProtocolQr(JSON.stringify(value), title, info);
}

export function showPrivateSwapSection(section) {
    ['hub', 'create', 'join', 'dashboard']
        .forEach(name => byId(`private-swap-${name}`)?.classList.toggle('hidden', name !== section));
}

export function renderPrivateSwapUi(state, pendingDeviceAction) {
    byId('btn-private-swap-resume')?.classList.toggle('hidden', !state.role);
    if (!state.role) return;
    elementById('private-swap-role').textContent = state.role === 'alice' ? 'ALICE' : 'BOB';
    elementById('private-swap-my-address').textContent = state.myAddress || 'Not built yet';
    elementById('private-swap-counter-address').textContent = state.counterAddress || 'Not received yet';
    elementById('private-swap-status').textContent = statusText(state);
    byId('private-swap-actions')?.querySelectorAll('button').forEach(button => button.classList.add('hidden'));
    for (const id of actionIds(state)) byId(id)?.classList.remove('hidden');
    if (pendingDeviceAction) byId('btn-private-swap-scan-device')?.classList.remove('hidden');
}

// Each rule reveals its buttons when its predicate holds. Order is preserved from the original chain.
/** @type {[(state: any) => boolean, string[]][]} */
const ACTION_RULES = [
    [state => state.role === 'alice' && state.stage === 'alice-offer-ready', ['btn-private-swap-share-offer', 'btn-private-swap-scan-response']],
    [state => state.role === 'bob' && state.stage === 'bob-response-ready', ['btn-private-swap-share-response', 'btn-private-swap-scan-final']],
    [state => state.stage === 'alice-needs-binding' || state.stage === 'bob-needs-binding', ['btn-private-swap-bind']],
    [state => state.role === 'alice' && state.stage === 'alice-bound', ['btn-private-swap-share-final', 'btn-private-swap-fund']],
    [state => state.role === 'bob' && state.myBindingToken && !state.myPreSignature, ['btn-private-swap-presign']],
    [state => state.role === 'bob' && state.myPreSignature, ['btn-private-swap-fund', 'btn-private-swap-scan-presig']],
    [state => state.role === 'alice' && state.myBindingToken && state.myAddress && !state.myPreSignature, ['btn-private-swap-fund', 'btn-private-swap-presign']],
    [state => state.role === 'alice' && state.myPreSignature && !state.readyAckHash, ['btn-private-swap-share-presig', 'btn-private-swap-scan-ready']],
    [state => state.role === 'bob' && state.myPreSignature && state.counterPreSignature, ['btn-private-swap-share-ready']],
    [state => state.role === 'alice' && state.readyAckHash && !state.completed, ['btn-private-swap-complete']],
    [state => state.role === 'bob' && state.counterCompletedSignature && !state.completed, ['btn-private-swap-bob-claim']],
    [state => Boolean(state.myAddress) && !state.completed, ['btn-private-swap-refund']],
];

function actionIds(state) {
    const ids = ACTION_RULES.filter(([when]) => when(state)).flatMap(([, buttons]) => buttons);
    return [...new Set(ids)];
}

function statusText(state) {
    if (state.completed) return '✓ Private Swap completed.';
    const map = {
        'alice-offer-ready': 'Share the offer with Bob, then scan his response.',
        'bob-response-ready': 'Share Bob’s response with Alice, then scan her final handshake.',
        'alice-needs-binding': 'Bind Alice’s isolated claim key to Bob’s exact covenant.',
        'bob-needs-binding': 'Bind Bob’s isolated claim key to Alice’s exact covenant.',
        'alice-bound': 'Share the final handshake, then fund Alice’s long-timeout side first.',
        'bob-bound': 'Wait for Alice funding; prepare Bob’s exact claim pre-signature before funding Bob’s side.',
        'bob-presigned': 'Bob claim pre-signature is safe. After verifying Alice funding, fund Bob’s short-timeout side.',
        'alice-presigned': 'Share Alice’s exact claim pre-signature with Bob and wait for his cryptographic ready acknowledgement.',
        'bob-alice-presig-verified': 'Alice pre-signature verified. Send Bob’s ready acknowledgement; watcher will extract the secret after Alice claims.',
        'alice-bob-ready': 'Bob’s claim pre-signature is verified. Alice can safely complete and claim Bob’s side.',
    };
    return map[state.stage || 'idle'] || 'Follow the numbered Private Swap steps. Claim signatures are exact transaction sighashes; no preimage is used.';
}
