// Network selection derived from wallet addresses; Kaspa Portal owns the
// network names and address prefixes behind the WASM boundary.

import { address_network, network_address_prefix } from '../wasm/api.js';

export function detectWalletNetwork(walletJson, selectedNetwork = 'mainnet') {
    if (!walletJson) return 'mainnet';
    const wallet = typeof walletJson === 'string' ? JSON.parse(walletJson) : walletJson;
    return address_network(wallet.receive_addresses?.[0] || '', selectedNetwork);
}

export function addressPrefix(network = 'mainnet') {
    return network_address_prefix(network);
}
