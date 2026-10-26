// Default fakes for the Portal-backed WASM exports the Companion web tests do
// not exercise directly. They mirror the export contracts, not Portal's rules:
// tests that care about a result stub the export themselves.

const NETWORK_HRPS = Object.freeze({ mainnet: 'kaspa', devnet: 'kaspadev', simnet: 'kaspasim' });

function hrp(network) {
    const name = String(network ?? '');
    return name.startsWith('testnet') ? 'kaspatest' : NETWORK_HRPS[name];
}

const SCRIPT_TERM_TYPES = new Set([
    'dms', 'timelocked-savings', 'additive', 'escrow',
    'global-allowance', 'global-spending-limit', 'payjoin', 'oracle-v1',
]);

export const PORTAL_WASM_FAKES = Object.freeze({
    covenant_script_terms: (type, redeemScriptHex, address) => {
        if (!SCRIPT_TERM_TYPES.has(type)) throw new Error(`Unsupported covenant type: ${JSON.stringify(type)}`);
        return JSON.stringify({ type, address, redeem_script_hex: redeemScriptHex });
    },
    network_address_prefix: network => (hrp(network) ? `${hrp(network)}:` : ''),
    address_network: (address, selected = 'mainnet') => {
        const prefix = String(address ?? '').split(':')[0];
        if (prefix === 'kaspatest') return /^testnet-([1-9]\d?|1[01]\d|12[0-7])$/.test(String(selected)) ? selected : 'testnet-10';
        if (prefix === 'kaspa') return 'mainnet';
        if (prefix === 'kaspasim') return 'simnet';
        if (prefix === 'kaspadev') return 'devnet';
        throw new Error(`not a valid Kaspa address: ${JSON.stringify(address)}`);
    },
    is_kaspa_address: text => /^kaspa(test|sim|dev)?:./.test(String(text ?? '')),
    address_script_public_key: () => `20${'11'.repeat(32)}ac`,
    thread_withdraw_fee: (_thread, _withdraw, minimumFee) => BigInt(minimumFee),
    thread_maximum_withdraw: (thread, cap, minimumFee, _rate, minimumReturn) => {
        const room = BigInt(thread) - BigInt(minimumFee) - BigInt(minimumReturn);
        const limit = room > 0n ? room : 0n;
        return BigInt(cap) < limit ? BigInt(cap) : limit;
    },
    private_swap_max_fee_sompi: () => 500_000_000n,
    pskb_encode_document: json => Buffer.from(`PSKB${Buffer.from(`[${json}]`, 'utf8').toString('hex')}`, 'utf8').toString('hex'),
    pskb_append_output: wire => wire,
    redeem_script_address: (redeem, network) => `${hrp(network) ?? 'kaspa'}:p2sh-${String(redeem).slice(-12)}`,
    script_public_key_address: () => '',
    covenant_spend_path: () => 'unknown',
    multisig_descriptor_summary: text => {
        const match = /^(multi|multi_hd|multi_hd45)\((\d+),(.*)\)$/.exec(String(text ?? '').split('\n')[0].trim());
        if (!match) throw new Error('not a multisig descriptor');
        const kind = { multi: 'static', multi_hd: 'hd44', multi_hd45: 'hd45' }[match[1]];
        return JSON.stringify({ kind, threshold: Number(match[2]), participants: match[3].split(',').length });
    },
});

// Generated WASM stub modules import the fakes through this line.
export const PORTAL_FAKES_IMPORT = `import { PORTAL_WASM_FAKES } from '${import.meta.url}';`;

// One generated export: the test's own stub first, then the Portal fake.
export function stubExport(name) {
    const key = JSON.stringify(name);
    return `export function ${name}(...args) { const hook = globalThis.__COMPANION_WASM_STUBS__?.[${key}] ?? PORTAL_WASM_FAKES[${key}]; return typeof hook === 'function' ? hook(...args) : ''; }`;
}
