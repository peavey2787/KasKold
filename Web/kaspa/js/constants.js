/**
 * Kaspa Wallet Constants
 * Centralized location for all derivation paths and network constants
 */

// Default derivation paths
export const DEFAULT_ACCOUNT_PATH = "m/44'/111111'/0'";
export const DEFAULT_ADDRESS_PATH = "m/44'/111111'/0'/0/0";

// Network configurations
export const NETWORKS = {
    MAINNET: 'mainnet',
    TESTNET_10: 'testnet-10',
    TESTNET_11: 'testnet-11',
    DEVNET: 'devnet',
    SIMNET: 'simnet'
};

// Address types
export const ADDRESS_TYPES = {
    RECEIVE: 0,
    CHANGE: 1
};

// Default network for new installations
export const DEFAULT_NETWORK = NETWORKS.TESTNET_10;

// Default transaction amount in KAS
export const DEFAULT_TRANSACTION_AMOUNT = 0.2;

// Minimum transaction amount in KAS
export const MIN_TRANSACTION_AMOUNT = 0.2;

// Maximum transaction amount in KAS (safety limit)
export const MAX_TRANSACTION_AMOUNT = 1000000;

// Maximum number of UTXOs to include in a single transaction (to avoid storage mass limits)
export const MAX_UTXOS_PER_TRANSACTION = 100;

// Transaction storage mass limits
export const MAX_TRANSACTION_STORAGE_MASS = 100000;

// Recommended UTXO consolidation threshold
export const UTXO_CONSOLIDATION_THRESHOLD = 50;

// Kaspa coin type for BIP44
export const KASPA_COIN_TYPE = "111111'";

// BIP44 purpose
export const BIP44_PURPOSE = "44'";

// Default account index
export const DEFAULT_ACCOUNT_INDEX = "0'";

// Default change index
export const DEFAULT_CHANGE_INDEX = "0";

// Default address index
export const DEFAULT_ADDRESS_INDEX = "0";

// Build derivation path helper function
export function buildDerivationPath(accountIndex = DEFAULT_ACCOUNT_INDEX, changeIndex = DEFAULT_CHANGE_INDEX, addressIndex = DEFAULT_ADDRESS_INDEX) {
    return `m/${BIP44_PURPOSE}/${KASPA_COIN_TYPE}/${accountIndex}/${changeIndex}/${addressIndex}`;
}

// Build account path helper function
export function buildAccountPath(accountIndex = DEFAULT_ACCOUNT_INDEX) {
    return `m/${BIP44_PURPOSE}/${KASPA_COIN_TYPE}/${accountIndex}`;
}

// Default scanning parameters
export const SCANNING_DEFAULTS = {
    GAP_LIMIT: 20,
    BATCH_SIZE: 10,
    MAX_ADDRESSES: 200
}; 