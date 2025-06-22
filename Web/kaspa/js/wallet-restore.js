// Kaspa Wallet Restoration Module
import { getKaspa, isInitialized } from './init.js';

// Restore wallet from mnemonic phrase
function restoreWalletFromMnemonic(mnemonicPhrase, networkType, derivationPath, passphrase = null) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { Mnemonic, XPrv } = kaspa;

        // Validate mnemonic phrase
        let mnemonic;
        try {
            mnemonic = new Mnemonic(mnemonicPhrase.trim());
        } catch (error) {
            throw new Error('Invalid mnemonic phrase. Please check your words and try again.');
        }

        // Generate seed from mnemonic with optional passphrase
        const seed = passphrase 
            ? mnemonic.toSeed(passphrase)
            : mnemonic.toSeed();
        
        // Generate the hierarchical extended private key (xPrv) from the seed
        const xPrv = new XPrv(seed);
        
        // Derive along the specified derivation path
        const derivedXPrv = xPrv.derivePath(derivationPath);
        const privateKey = derivedXPrv.toPrivateKey();
        
        // Convert the public key into a Kaspa address
        const publicAddress = privateKey.toPublicKey().toAddress(networkType).toString();

        return {
            success: true,
            mnemonic: mnemonicPhrase.trim(),
            publicAddress: publicAddress,
            privateKey: privateKey.toString(),
            networkType: networkType,
            derivationPath: derivationPath,
            restoredFrom: 'mnemonic'
        };

    } catch (error) {
        return {
            success: false,
            error: error.message,
            restoredFrom: 'mnemonic'
        };
    }
}

// Restore wallet from private key
function restoreWalletFromPrivateKey(privateKeyString, networkType) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { PrivateKey } = kaspa;

        // Validate and create private key
        let privateKey;
        try {
            privateKey = new PrivateKey(privateKeyString.trim());
        } catch (error) {
            throw new Error('Invalid private key format. Please check your private key and try again.');
        }
        
        // Generate the public address from private key
        const publicAddress = privateKey.toPublicKey().toAddress(networkType).toString();

        return {
            success: true,
            mnemonic: null, // No mnemonic when restoring from private key
            publicAddress: publicAddress,
            privateKey: privateKey.toString(),
            networkType: networkType,
            derivationPath: null, // No derivation path when importing private key
            restoredFrom: 'privateKey'
        };

    } catch (error) {
        return {
            success: false,
            error: error.message,
            restoredFrom: 'privateKey'
        };
    }
}

// Validate mnemonic phrase
function validateMnemonic(mnemonicPhrase) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { Mnemonic } = kaspa;
        
        const words = mnemonicPhrase.trim().split(/\s+/);
        
        // Check word count (should be 12, 15, 18, 21, or 24 words)
        const validWordCounts = [12, 15, 18, 21, 24];
        if (!validWordCounts.includes(words.length)) {
            return {
                isValid: false,
                error: `Invalid word count. Expected 12, 15, 18, 21, or 24 words, got ${words.length}.`
            };
        }

        // Validate mnemonic with Kaspa SDK
        try {
            new Mnemonic(mnemonicPhrase.trim());
            return {
                isValid: true,
                wordCount: words.length
            };
        } catch (error) {
            return {
                isValid: false,
                error: 'Invalid mnemonic phrase. Please check your words and try again.'
            };
        }

    } catch (error) {
        return {
            isValid: false,
            error: error.message
        };
    }
}

// Validate private key
function validatePrivateKey(privateKeyString) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { PrivateKey } = kaspa;
        
        // Try to create a private key to validate format
        try {
            new PrivateKey(privateKeyString.trim());
            return {
                isValid: true
            };
        } catch (error) {
            return {
                isValid: false,
                error: 'Invalid private key format. Please check your private key and try again.'
            };
        }

    } catch (error) {
        return {
            isValid: false,
            error: error.message
        };
    }
}

// Get wallet information (for display)
function getWalletInfo(restorationResult) {
    if (!restorationResult.success) {
        return null;
    }

    return {
        address: restorationResult.publicAddress,
        privateKey: restorationResult.privateKey,
        mnemonic: restorationResult.mnemonic,
        networkType: restorationResult.networkType,
        derivationPath: restorationResult.derivationPath,
        restoredFrom: restorationResult.restoredFrom
    };
}

export {
    restoreWalletFromMnemonic,
    restoreWalletFromPrivateKey,
    validateMnemonic,
    validatePrivateKey,
    getWalletInfo
}; 