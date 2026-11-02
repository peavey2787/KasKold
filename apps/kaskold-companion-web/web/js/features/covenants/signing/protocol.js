import { bytesToHex, hexToBytes } from '../../../core/bytes.js';

const REQUEST_MAGIC = new TextEncoder().encode('CVSG');
const REVEAL_MAGIC = new TextEncoder().encode('CVRV');
const RESPONSE_MAGIC = new TextEncoder().encode('CVSR');
const VERSION = 1;
const SESSION_LEN = 16;
const REQUEST_HEADER = 156;
const REVEAL_LEN = 117;
const RESPONSE_LEN = 247;
const MAX_SCRIPT = 3072;
const MAX_CONTEXT = 1024;
const HOST_COMMIT_DOMAIN = new TextEncoder().encode('KaspaPortal/anti-klepto/host-commit/v1');

export const CovenantRequestKind = Object.freeze({ KEY_INFO: 0, KNOWN: 1, OPAQUE: 2, BIND: 3 });
export const CovenantKnownScheme = Object.freeze({ NONE: 0, SHA256_PREIMAGE: 1, ORACLE_V1: 2 });
export const CovenantBinding = Object.freeze({ NONE: 0, KEY_PRESENT: 1, FIXED_CHECKSIGFROMSTACK: 2 });

export async function createCovenantSigningChallenge() {
    const hostSecret = randomNonzeroBytes(32);
    const sessionId = randomNonzeroBytes(SESSION_LEN);
    const material = new Uint8Array(HOST_COMMIT_DOMAIN.length + hostSecret.length);
    material.set(HOST_COMMIT_DOMAIN); material.set(hostSecret, HOST_COMMIT_DOMAIN.length);
    const hostCommitment = new Uint8Array(await crypto.subtle.digest('SHA-256', material));
    return {
        sessionId: bytesToHex(sessionId),
        hostSecret: bytesToHex(hostSecret),
        hostCommitment: bytesToHex(hostCommitment),
    };
}

/** Device allocates the covenant key ID; the host cannot select/reuse one. */
export function covenantKeyRequestHex() {
    return encodeRequest({
        kind: CovenantRequestKind.KEY_INFO, scheme: CovenantKnownScheme.NONE, binding: CovenantBinding.NONE,
        sessionIdHex: '00'.repeat(SESSION_LEN), hostCommitmentHex: '00'.repeat(32), keyIdHex: '00'.repeat(32),
        bindingTokenHex: '00'.repeat(32), commitmentHex: '00'.repeat(32), scriptHex: '', context: new Uint8Array(),
    });
}

/**
 * Bind one freshly device-allocated key ID to this exact redeem script. The
 * returned binding record is portable non-secret metadata and must be kept
 * with the covenant for all future signing requests.
 */
export function covenantBindRequestHex({
    keyIdHex, scriptHex, context = new Uint8Array(), commitmentHex = '00'.repeat(32),
    scheme = CovenantKnownScheme.NONE, verifyDirectKeyBinding = false,
}) {
    const binding = scheme === CovenantKnownScheme.NONE
        ? (verifyDirectKeyBinding ? CovenantBinding.KEY_PRESENT : CovenantBinding.NONE)
        : knownBinding(scheme);
    return encodeRequest({
        kind: CovenantRequestKind.BIND, scheme, binding,
        sessionIdHex: '00'.repeat(SESSION_LEN), hostCommitmentHex: '00'.repeat(32), keyIdHex,
        bindingTokenHex: '00'.repeat(32), commitmentHex, scriptHex,
        context: context instanceof Uint8Array ? context : new TextEncoder().encode(String(context)),
    });
}

export function covenantKnownRequestHex({
    sessionIdHex, hostCommitmentHex, keyIdHex, bindingTokenHex, commitmentHex, scriptHex, context,
    scheme = CovenantKnownScheme.SHA256_PREIMAGE,
}) {
    return encodeRequest({
        kind: CovenantRequestKind.KNOWN, scheme, binding: knownBinding(scheme),
        sessionIdHex, hostCommitmentHex, keyIdHex, bindingTokenHex, commitmentHex, scriptHex,
        context: context instanceof Uint8Array ? context : new TextEncoder().encode(String(context)),
    });
}

export function covenantOpaqueRequestHex({
    sessionIdHex, hostCommitmentHex, keyIdHex, bindingTokenHex, commitmentHex, scriptHex,
    verifyDirectKeyBinding = false,
}) {
    return encodeRequest({
        kind: CovenantRequestKind.OPAQUE, scheme: CovenantKnownScheme.NONE,
        binding: verifyDirectKeyBinding ? CovenantBinding.KEY_PRESENT : CovenantBinding.NONE,
        sessionIdHex, hostCommitmentHex, keyIdHex, bindingTokenHex, commitmentHex, scriptHex,
        context: new Uint8Array(),
    });
}

export function covenantRevealHex({ sessionId, keyId, commitment, hostSecret }) {
    const out = new Uint8Array(REVEAL_LEN);
    out.set(REVEAL_MAGIC, 0); out[4] = VERSION;
    out.set(nonzeroFixedHex(sessionId, SESSION_LEN, 'covenant session'), 5);
    out.set(nonzeroFixedHex(keyId, 32, 'covenant key ID'), 21);
    out.set(fixedHex(commitment, 32, 'covenant commitment'), 53);
    out.set(fixedHex(hostSecret, 32, 'host secret'), 85);
    return bytesToHex(out);
}

export function parseCovenantResponse(raw) {
    const bytes = normalizeResponseBytes(raw);
    if (bytes.length !== RESPONSE_LEN || !equalPrefix(bytes, RESPONSE_MAGIC) || bytes[4] !== VERSION) {
        throw new Error('Not a current KasKold covenant response');
    }
    const kind = ['key', 'nonce', 'signature', 'binding'][bytes[5]];
    if (!kind) throw new Error('Unknown covenant response kind');
    const response = {
        kind,
        sessionId: bytesToHex(bytes.slice(6, 22)),
        keyId: bytesToHex(bytes.slice(22, 54)),
        pubkey: bytesToHex(bytes.slice(54, 86)),
        bindingToken: bytesToHex(bytes.slice(86, 118)),
        commitment: bytesToHex(bytes.slice(118, 150)),
        noncePoint: bytesToHex(bytes.slice(150, 183)),
        signature: bytesToHex(bytes.slice(183, 247)),
    };
    validateResponse(response);
    return response;
}

export function covenantSignatureResponseHex({
    sessionId, keyId, pubkey, bindingToken, commitment, noncePoint, signature,
}) {
    const out = new Uint8Array(RESPONSE_LEN);
    out.set(RESPONSE_MAGIC, 0); out[4] = VERSION; out[5] = 2;
    out.set(nonzeroFixedHex(sessionId, SESSION_LEN, 'covenant session'), 6);
    out.set(nonzeroFixedHex(keyId, 32, 'covenant key ID'), 22);
    out.set(nonzeroFixedHex(pubkey, 32, 'covenant public key'), 54);
    out.set(nonzeroFixedHex(bindingToken, 32, 'covenant binding record'), 86);
    out.set(fixedHex(commitment, 32, 'covenant commitment'), 118);
    const nonce = fixedHex(noncePoint, 33, 'covenant nonce point');
    if (nonce[0] !== 0x02) throw new Error('Covenant nonce point must use even-Y encoding');
    out.set(nonce, 150);
    const sig = nonzeroFixedHex(signature, 64, 'covenant signature'); out.set(sig, 183);
    return bytesToHex(out);
}

export async function sha256Commitment(data) {
    const bytes = data instanceof Uint8Array ? data : new TextEncoder().encode(String(data));
    return bytesToHex(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)));
}

export async function covenantScriptFingerprint(scriptHex) {
    return sha256Commitment(hexToBytes(scriptHex || ''));
}

function encodeRequest({
    kind, scheme, binding, sessionIdHex, hostCommitmentHex, keyIdHex, bindingTokenHex,
    commitmentHex, scriptHex, context,
}) {
    const sessionId = fixedHex(sessionIdHex, SESSION_LEN, 'covenant session');
    const hostCommitment = fixedHex(hostCommitmentHex, 32, 'host commitment');
    const keyId = fixedHex(keyIdHex, 32, 'covenant key ID');
    const bindingToken = fixedHex(bindingTokenHex, 32, 'covenant binding record');
    const commitment = fixedHex(commitmentHex, 32, 'covenant commitment');
    const script = hexToBytes(scriptHex || '');
    const contextBytes = context instanceof Uint8Array ? context : new Uint8Array(context || []);
    if (script.length > MAX_SCRIPT || contextBytes.length > MAX_CONTEXT) {
        throw new Error('Covenant request field exceeds protocol limit');
    }
    validateShape({ kind, scheme, binding, sessionId, hostCommitment, keyId, bindingToken, commitment, script, reviewContext: contextBytes });
    const out = new Uint8Array(REQUEST_HEADER + script.length + contextBytes.length);
    out.set(REQUEST_MAGIC, 0); out[4] = VERSION; out[5] = kind; out[6] = scheme; out[7] = binding;
    out.set(sessionId, 8); out.set(hostCommitment, 24); out.set(keyId, 56); out.set(bindingToken, 88); out.set(commitment, 120);
    out[152] = (script.length >>> 8) & 0xff; out[153] = script.length & 0xff;
    out[154] = (contextBytes.length >>> 8) & 0xff; out[155] = contextBytes.length & 0xff;
    let offset = REQUEST_HEADER; out.set(script, offset); offset += script.length; out.set(contextBytes, offset);
    return bytesToHex(out);
}

const REQUEST_VALIDATORS = new Map([
    [CovenantRequestKind.KEY_INFO, validateKeyRequest],
    [CovenantRequestKind.BIND, validateBindRequest],
    [CovenantRequestKind.KNOWN, validateKnownRequest],
    [CovenantRequestKind.OPAQUE, validateOpaqueRequest],
]);

function validateShape(request) {
    const validate = REQUEST_VALIDATORS.get(request.kind);
    if (!validate) throw new Error('Unknown covenant request kind');
    validate(request);
}

function validateKeyRequest({ scheme, binding, sessionId, hostCommitment, keyId, bindingToken, commitment, script, reviewContext }) {
    const allFieldsZero = [sessionId, hostCommitment, keyId, bindingToken, commitment].every(allZero);
    if (scheme !== CovenantKnownScheme.NONE || binding !== CovenantBinding.NONE || !allFieldsZero || script.length || reviewContext.length) {
        throw new Error('Invalid key request');
    }
}

function validateBindRequest({ scheme, binding, sessionId, hostCommitment, keyId, bindingToken, commitment, script, reviewContext }) {
    const bindingShape = scheme !== CovenantKnownScheme.NONE
        ? binding === knownBinding(scheme) && reviewContext.length > 0
        : isDirectBinding(binding) && allZero(commitment) && reviewContext.length === 0;
    if (!allZero(sessionId) || !allZero(hostCommitment) || allZero(keyId) || !allZero(bindingToken) || !script.length || !bindingShape) {
        throw new Error('Invalid covenant binding request');
    }
}

function validateKnownRequest({ scheme, binding, sessionId, hostCommitment, keyId, bindingToken, script, reviewContext }) {
    if (scheme === CovenantKnownScheme.NONE || binding !== knownBinding(scheme)
        || anyZero(sessionId, hostCommitment, keyId, bindingToken) || !script.length || !reviewContext.length) {
        throw new Error('Invalid known covenant request');
    }
}

function validateOpaqueRequest({ scheme, binding, sessionId, hostCommitment, keyId, bindingToken, script, reviewContext }) {
    if (scheme !== CovenantKnownScheme.NONE || !isDirectBinding(binding)
        || anyZero(sessionId, hostCommitment, keyId, bindingToken) || !script.length || reviewContext.length) {
        throw new Error('Invalid opaque covenant request');
    }
}

function isDirectBinding(binding) {
    return binding === CovenantBinding.NONE || binding === CovenantBinding.KEY_PRESENT;
}

function knownBinding(scheme) {
    if (scheme === CovenantKnownScheme.SHA256_PREIMAGE || scheme === CovenantKnownScheme.ORACLE_V1) return CovenantBinding.FIXED_CHECKSIGFROMSTACK;
    throw new Error('Unknown known-covenant scheme');
}

function validateResponse(response) {
    if (isZeroHex(response.keyId) || isZeroHex(response.pubkey)) throw new Error('Invalid covenant response key');
    const zero = zeroFieldsOf(response);
    if (response.kind === 'key') return validateKeyResponse(zero);
    if (response.kind === 'binding') return validateBindingResponse(zero);
    validateTranscriptResponse(response, zero);
}

function zeroFieldsOf(response) {
    return {
        session: isZeroHex(response.sessionId), binding: isZeroHex(response.bindingToken),
        commitment: isZeroHex(response.commitment), nonce: isZeroHex(response.noncePoint),
        signature: isZeroHex(response.signature),
    };
}

function validateKeyResponse(zero) {
    if (!zero.session || !zero.binding || !zero.commitment || !zero.nonce || !zero.signature) throw new Error('Malformed covenant key response');
}

function validateBindingResponse(zero) {
    if (!zero.session || zero.binding || zero.commitment || !zero.nonce || !zero.signature) throw new Error('Malformed covenant binding response');
}

function validateTranscriptResponse(response, zero) {
    if (zero.session || zero.binding || !response.noncePoint.startsWith('02')) throw new Error('Malformed covenant nonce transcript');
    if (response.kind === 'nonce' && !zero.signature) throw new Error('Nonce response must not contain a final signature');
    if (response.kind === 'signature' && zero.signature) throw new Error('Final covenant response is missing its signature');
}

function isZeroHex(hex) { return /^0+$/.test(hex); }

function fixedHex(value, length, label) {
    const bytes = hexToBytes(String(value || '').trim());
    if (bytes.length !== length) throw new Error(`${label} must be exactly ${length} bytes`);
    return bytes;
}
function nonzeroFixedHex(value, length, label) {
    const bytes = fixedHex(value, length, label); if (allZero(bytes)) throw new Error(`${label} cannot be zero`); return bytes;
}
function randomNonzeroBytes(length) {
    const bytes = new Uint8Array(length); crypto.getRandomValues(bytes); if (allZero(bytes)) bytes[length - 1] = 1; return bytes;
}
function allZero(bytes) { return bytes.every(byte => byte === 0); }
function anyZero(...fields) { return fields.some(allZero); }
function normalizeResponseBytes(raw) {
    const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
    if (equalPrefix(bytes, RESPONSE_MAGIC)) return bytes;
    const text = new TextDecoder().decode(bytes).trim();
    if (/^[0-9a-f]+$/i.test(text) && text.length === RESPONSE_LEN * 2) return hexToBytes(text);
    throw new Error('Not a current KasKold covenant response');
}
function equalPrefix(bytes, prefix) {
    if (bytes.length < prefix.length) return false;
    return prefix.every((byte, index) => bytes[index] === byte);
}
