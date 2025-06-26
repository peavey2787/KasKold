// Kaspa Wallet Generator Module
import { getKaspa, isInitialized } from './init.js';
import { DEFAULT_ACCOUNT_PATH, DEFAULT_ADDRESS_PATH, NETWORKS } from './constants.js';

// Legacy aliases for backward compatibility
const NETWORK_TYPES = NETWORKS;
const ACCOUNT_PATH = DEFAULT_ACCOUNT_PATH;

// Generate a new wallet
function generateWallet(networkType, derivationPath = DEFAULT_ADDRESS_PATH, passphrase = null) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized. Call initKaspa() first.');
    }

    const kaspa = getKaspa();
    const { Mnemonic, XPrv } = kaspa;

    // Generate a new random mnemonic phrase
    const generatedMnemonic = Mnemonic.random().phrase;
    
    // Generate seed from mnemonic with optional passphrase
    const seed = passphrase 
        ? new Mnemonic(generatedMnemonic).toSeed(passphrase)
        : new Mnemonic(generatedMnemonic).toSeed();
    
    // Generate the hierarchical extended private key (xPrv) from the seed
    const xPrv = new XPrv(seed);
    
    // First derive to the account level to get the xpub
    const accountXPrv = xPrv.derivePath(ACCOUNT_PATH);
    const xpub = accountXPrv.toXPub();
    
    // Then derive to the full path for the first address
    const derivedXPrv = xPrv.derivePath(derivationPath);
    const privateKey = derivedXPrv.toPrivateKey();
    
    // Convert the public key into a Kaspa address
    const publicAddress = privateKey.toPublicKey().toAddress(networkType).toString();

    const walletData = {
        mnemonic: generatedMnemonic,
        publicAddress: publicAddress,
        privateKey: privateKey.toString(),
        networkType: networkType,
        derivationPath: ACCOUNT_PATH, // ✅ Store account-level path for HD wallet compatibility
        xpub: xpub.toString(),
        accountPath: ACCOUNT_PATH
    };
    
    return walletData;
}

// Export functions and constants
export {
    NETWORK_TYPES,
    DEFAULT_ADDRESS_PATH,
    ACCOUNT_PATH,
    generateWallet
}; 
