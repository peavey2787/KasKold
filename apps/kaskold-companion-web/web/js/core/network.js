// Network selection derived from wallet addresses; Kaspa Portal owns the
// network names and address prefixes behind the WASM boundary.

import { address_network, network_address_prefix } from '../wasm/api.js';

// The loaded wallet's network. Without a wallet the selection stands; a wallet
// whose first address does not decode is rejected rather than read as mainnet.
export function detectWalletNetwork(walletJson, selectedNetwork = 'mainnet') {
    if (!walletJson) return selectedNetwork;
    const wallet = typeof walletJson === 'string' ? JSON.parse(walletJson) : walletJson;
    const address = wallet.receive_addresses?.[0];
    if (!address) throw new Error('Wallet has no receive address');
    return address_network(address, selectedNetwork);
}

export function addressPrefix(network = 'mainnet') {
    return network_address_prefix(network);
}
