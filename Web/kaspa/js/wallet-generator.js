// Kaspa Wallet Generator Module
import { getKaspa, isInitialized } from './init.js';

// Network types configuration
const NETWORK_TYPES = {
    MAINNET: 'mainnet',
    TESTNET_10: 'testnet-10',
    TESTNET_11: 'testnet-11',
    DEVNET: 'devnet',
    SIMNET: 'simnet'
};

// Default derivation path
const DEFAULT_DERIVATION_PATH = "m/44'/111111'/0'/0/0";

// Generate a new wallet
function generateWallet(networkType, derivationPath, passphrase = null) {
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
    
    // Derive along the specified derivation path
    const derivedXPrv = xPrv.derivePath(derivationPath);
    const privateKey = derivedXPrv.toPrivateKey();
    
    // Convert the public key into a Kaspa address
    const publicAddress = privateKey.toPublicKey().toAddress(networkType).toString();

    const walletData = {
        mnemonic: generatedMnemonic,
        publicAddress: publicAddress,
        privateKey: privateKey.toString(),
        networkType: networkType,
        derivationPath: derivationPath
    };
    
    
    return walletData;
}

// Export functions and constants
export {
    NETWORK_TYPES,
    DEFAULT_DERIVATION_PATH,
    generateWallet
}; 
