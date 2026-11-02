// Parsed wallet session. JSON serialization is confined to WASM and persistence boundaries.
let walletJson = null;
let walletSnapshot = null;
let walletProfile = null;

function cloneWallet(value) {
    if (typeof structuredClone === 'function') return structuredClone(value);
    return JSON.parse(JSON.stringify(value));
}

function parseWallet(value) {
    if (typeof value === 'string') {
        const snapshot = JSON.parse(value);
        return { json: value, snapshot: cloneWallet(snapshot) };
    }
    if (value && typeof value === 'object') {
        const snapshot = cloneWallet(value);
        return { json: JSON.stringify(snapshot), snapshot };
    }
    throw new TypeError('wallet payload must be a JSON string or object');
}

const EMPTY_BY_TYPE = { string: '', bigint: 0n, number: 0, boolean: false };

function isScrubbableBinary(value) {
    return (ArrayBuffer.isView(value) && typeof value.fill === 'function') || value instanceof ArrayBuffer;
}

function scrubBinary(value) {
    try {
        if (ArrayBuffer.isView(value)) value.fill(0);
        else new Uint8Array(value).fill(0);
    } catch { /* A detached or frozen buffer holds nothing further to scrub. */ }
}

function scrubProperty(value, key, current) {
    try {
        if (Object.hasOwn(EMPTY_BY_TYPE, typeof current)) value[key] = EMPTY_BY_TYPE[typeof current];
        else if (current && typeof current === 'object') value[key] = null;
    } catch { /* A read-only property holds nothing further to scrub. */ }
}

function scrubObjectProperties(value, seen) {
    for (const key of Object.keys(value)) {
        const current = value[key];
        if (current && typeof current === 'object') bestEffortScrubMutable(current, seen);
        scrubProperty(value, key, current);
    }
}

export function bestEffortScrubMutable(value, seen = new WeakSet()) {
    if (!value || typeof value !== 'object' || seen.has(value)) return;
    seen.add(value);
    if (isScrubbableBinary(value)) {
        scrubBinary(value);
        return;
    }
    if (value instanceof Set || value instanceof Map) {
        value.clear();
        return;
    }
    scrubObjectProperties(value, seen);
    if (Array.isArray(value)) value.length = 0;
}

export const walletSession = Object.freeze({
    scrubMutable: bestEffortScrubMutable,
    hasWallet() {
        return walletSnapshot !== null;
    },
    current() {
        return walletSnapshot === null ? null : cloneWallet(walletSnapshot);
    },
    json() {
        return walletJson;
    },
    kpub() {
        return walletSnapshot?.kpub || '';
    },
    replace(value) {
        const parsed = parseWallet(value);
        walletJson = parsed.json;
        walletSnapshot = parsed.snapshot;
        return cloneWallet(walletSnapshot);
    },
    profile() {
        return walletProfile === null ? null : { ...walletProfile };
    },
    setProfile(profile) {
        if (profile === null || profile === undefined) {
            walletProfile = null;
            return null;
        }
        const id = String(profile.id || '').trim();
        const name = String(profile.name || '').trim();
        walletProfile = id && name ? { id, name } : null;
        return walletProfile === null ? null : { ...walletProfile };
    },
    clear() {
        bestEffortScrubMutable(walletSnapshot);
        bestEffortScrubMutable(walletProfile);
        walletJson = '';
        walletSnapshot = null;
        walletProfile = null;
    },
    primaryReceiveAddress() {
        return walletSnapshot?.receive_addresses?.[0] || '';
    },
});
