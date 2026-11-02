import { KASPLEX_API, KNS_LOOKUP, KRC721_API } from '../../core/config/services.js';
import { exactUnsigned } from '../../core/exact.js';

async function fetchJson(url) {
    const response = await fetch(url, { signal: AbortSignal.timeout(8000) });
    return response.ok ? response.json() : null;
}

async function forEachAddress(addresses, visit) {
    for (const address of addresses) {
        try {
            await visit(address);
        } catch (_) {}
    }
}

function parseKrc20Entry(token) {
    const tick = token.tick || token.ticker || '';
    const balance = exactUnsigned(token.balance ?? '0', 'KRC20 balance');
    const decimalValue = exactUnsigned(token.dec ?? '8', 'KRC20 decimals');
    if (decimalValue > 255n) throw new Error('KRC20 decimals exceed supported range');
    return { tick, balance, decimals: Number(decimalValue) };
}

function mergeKrc20Entry(tokens, { tick, balance, decimals }) {
    if (!tick || balance === 0n) return;
    const current = tokens.get(tick) || { balance: 0n, decimals };
    if (current.decimals !== decimals) throw new Error(`KRC20 ${tick} decimals changed across addresses`);
    current.balance += balance;
    tokens.set(tick, current);
}

async function collectKrc20Address(tokens, base, address) {
    const data = await fetchJson(`${base}/krc20/address/${address}/tokenlist`);
    for (const token of data?.result || []) {
        mergeKrc20Entry(tokens, parseKrc20Entry(token));
    }
}

async function collectKrc20(addresses, network) {
    const base = KASPLEX_API[network];
    const tokens = new Map();
    if (!base) return tokens;
    await forEachAddress(addresses, address => collectKrc20Address(tokens, base, address));
    return tokens;
}

async function fetchCollectionBuri(base, tick) {
    try {
        const collection = await fetchJson(`${base}/nfts/${tick}`);
        return collection?.result?.buri || '';
    } catch (_) {
        return '';
    }
}

async function resolveCollectionBuri(base, buriByTick, tick) {
    if (!buriByTick.has(tick)) buriByTick.set(tick, await fetchCollectionBuri(base, tick));
    return buriByTick.get(tick);
}

function nftIdentity(nft) {
    const tick = nft.tick || '';
    const tokenId = nft.tokenId || nft.token_id || '';
    return tick && tokenId ? { tick, tokenId } : null;
}

function nftMetadataUrl(buri, tokenId) {
    return buri ? `${buri}/${tokenId}.json` : '';
}

async function collectKrc721Address(nfts, base, buriByTick, address) {
    const data = await fetchJson(`${base}/address/${address}`);
    for (const nft of data?.result || []) {
        const identity = nftIdentity(nft);
        if (!identity) continue;
        const buri = await resolveCollectionBuri(base, buriByTick, identity.tick);
        nfts.push({ ...identity, metadataUrl: nftMetadataUrl(buri, identity.tokenId) });
    }
}

async function collectKrc721(addresses, network) {
    const base = KRC721_API[network];
    const nfts = [];
    if (!base) return nfts;
    const buriByTick = new Map();
    await forEachAddress(addresses, address => collectKrc721Address(nfts, base, buriByTick, address));
    return nfts;
}

function collectKns(addresses) {
    const addressSet = new Set(addresses);
    return Object.entries(KNS_LOOKUP)
        .filter(([, address]) => addressSet.has(address))
        .map(([domain]) => domain);
}

export async function fetchWalletAssets(wallet, network) {
    const addresses = [...wallet.receive_addresses, ...wallet.change_addresses];
    const [tokens, nfts] = await Promise.all([
        collectKrc20(addresses, network),
        collectKrc721(addresses, network),
    ]);
    return { tokens, nfts, domains: collectKns(addresses) };
}
