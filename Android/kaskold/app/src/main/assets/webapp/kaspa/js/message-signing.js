import { getKaspa, isInitialized } from './init.js';

/**
 * Create an unsigned message data structure
 * @param {Object} messageData - Message data containing message, address, network, timestamp
 * @returns {Promise<Object>} - Result with success status and messageData
 */
async function createUnsignedMessage(messageData) {
    try {
        if (!messageData.message || !messageData.message.trim()) {
            throw new Error('Message cannot be empty');
        }
        
        if (!messageData.address) {
            throw new Error('Signing address is required');
        }
        
        // Generate a unique message ID for tracking
        const messageId = `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        const unsignedMessageData = {
            id: messageId,
            type: 'unsigned-message',
            message: messageData.message.trim(),
            address: messageData.address,
            network: messageData.network || 'mainnet',
            timestamp: messageData.timestamp || new Date().toISOString(),
            messageLength: messageData.message.trim().length,
            status: 'unsigned'
        };
        
        return {
            success: true,
            messageData: unsignedMessageData
        };
        
    } catch (error) {
        console.error("Error in createUnsignedMessage:", error);
        return {
            success: false,
            error: error.message || error.toString() || 'Unknown error creating unsigned message'
        };
    }
}

/**
 * Sign a message with the wallet's private key
 * @param {string} message - Message to sign
 * @param {string} privateKey - Private key in hex format
 * @param {string} address - Wallet address for verification
 * @param {string} networkType - Network type
 * @returns {Promise<Object>} - Signing result with signature and metadata
 */
async function signMessage(message, privateKey, address, networkType = 'mainnet') {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { signMessage, PrivateKey } = kaspa;
        
        if (!message || !message.trim()) {
            throw new Error('Message cannot be empty');
        }
        
        if (!privateKey) {
            throw new Error('Private key is required');
        }
        
        // Derive the public key from the private key for verification purposes
        let publicKeyHex = null;
        try {
            const privateKeyObj = new PrivateKey(privateKey);
            const publicKeyObj = privateKeyObj.toPublicKey();
            publicKeyHex = publicKeyObj.toString();
        } catch (keyError) {
            console.warn("Could not derive public key from private key:", keyError.message);
        }
        
        // Use the signMessage function from Kaspa SDK
        const signature = signMessage({
            message: message,
            privateKey: privateKey,
            noAuxRand: true  // For deterministic signatures
        });
        
        // Generate a unique signing ID for tracking
        const signingId = `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        return {
            success: true,
            signingId: signingId,
            message: message,
            signature: signature,
            signerAddress: address,
            signerPublicKey: publicKeyHex, // Store the public key for verification
            networkType: networkType,
            timestamp: new Date().toISOString(),
            messageLength: message.length,
            signatureLength: signature.length
        };
        
    } catch (error) {
        console.error("Error in signMessage:", error);
        return {
            success: false,
            error: error.message || error.toString() || 'Unknown signing error',
            message: message,
            signerAddress: address,
            networkType: networkType,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Verify a message signature
 * @param {string} message - The original message
 * @param {string} signature - The signature to verify
 * @param {string} address - The address or public key of the signer
 * @param {string} networkType - Network type
 * @returns {Promise<Object>} - Verification result
 */
async function verifyMessage(message, signature, address, networkType = 'mainnet') {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { verifyMessage, PublicKey } = kaspa;
        
        if (!message || !message.trim()) {
            throw new Error('Message cannot be empty');
        }
        
        if (!signature || !signature.trim()) {
            throw new Error('Signature cannot be empty');
        }
        
        if (!address || !address.trim()) {
            throw new Error('Address/public key cannot be empty');
        }
        
        // Create verification parameters
        const verifyParams = {
            message: message,
            signature: signature
        };
        
        // Handle different address formats exactly as in your working example
        if (address.includes(':')) {
            // For Kaspa addresses, use address property
            verifyParams.address = address;
        } else {
            // For hex public keys, create PublicKey object and use publicKey property
            try {
                // Validate that it's a valid hex string
                if (!/^[0-9a-fA-F]+$/.test(address)) {
                    throw new Error('Public key must be a valid hexadecimal string');
                }
                const publicKey = new PublicKey(address);
                verifyParams.publicKey = address;
            } catch (error) {
                console.error("Error with public key:", error);
                const errorMsg = error.message || 'Invalid public key format';
                throw new Error(`Invalid public key format: ${errorMsg}`);
            }
        }
        
        // Use the verifyMessage function from Kaspa SDK exactly as in your example
        
        let isValid = false;
        try {
            isValid = verifyMessage(verifyParams);
        } catch (verifyError) {
            console.error("Verification failed with error:", verifyError);
            
            // If address verification failed, try to get the public key from the address
            if (verifyParams.address) {
                try {
                    const { Address } = kaspa;
                    
                    // Try to create an Address object and extract public key info
                    const addressObj = new Address(verifyParams.address);
                    
                    // For now, re-throw the original error since we don't know the exact method
                    throw verifyError;
                    
                } catch (addressError) {
                    console.error("Failed to extract public key from address:", addressError);
                    throw verifyError; // Throw the original verification error
                }
            } else {
                throw verifyError;
            }
        }
        
        // Generate a unique verification ID for tracking
        const verificationId = `verify_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        return {
            success: true,
            verificationId: verificationId,
            isValid: isValid,
            message: message,
            signature: signature,
            signerAddress: address,
            networkType: networkType,
            timestamp: new Date().toISOString(),
            messageLength: message.length,
            signatureLength: signature.length
        };
        
    } catch (error) {
        console.error("Error in verifyMessage:", error);
        return {
            success: false,
            error: error.message || error.toString() || 'Unknown verification error',
            message: message,
            signature: signature,
            signerAddress: address,
            networkType: networkType,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Export message data for download
 * @param {Object} messageData - Message data to export
 * @param {string} type - Export type ('unsigned-message', 'signed-message', 'verification-result')
 * @returns {string} - JSON string for download
 */
function exportMessageData(messageData, type) {
    const exportData = {
        type: `kaspa-${type}`,
        ...messageData,
        exportTimestamp: new Date().toISOString()
    };
    
    return JSON.stringify(exportData, null, 2);
}

/**
 * Validate imported message data
 * @param {Object} importedData - Imported message data
 * @param {string} expectedType - Expected type
 * @returns {Object} - Validation result
 */
function validateImportedMessageData(importedData, expectedType) {
    try {
        if (!importedData || typeof importedData !== 'object') {
            throw new Error('Invalid file format - not a valid JSON object');
        }
        
        const fullExpectedType = `kaspa-${expectedType}`;
        if (importedData.type !== fullExpectedType) {
            throw new Error(`Invalid file type - expected ${fullExpectedType}, got ${importedData.type || 'unknown'}`);
        }
        
        // Common validations
        if (!importedData.message || typeof importedData.message !== 'string') {
            throw new Error('Invalid or missing message');
        }
        
        // Type-specific validations
        switch (expectedType) {
            case 'unsigned-message':
                if (!importedData.signerAddress) {
                    throw new Error('Missing signer address');
                }
                break;
                
            case 'signed-message':
                if (!importedData.signature) {
                    throw new Error('Missing signature');
                }
                if (!importedData.signerAddress) {
                    throw new Error('Missing signer address');
                }
                break;
                
            case 'verification-result':
                if (!importedData.signature) {
                    throw new Error('Missing signature');
                }
                if (typeof importedData.isValid !== 'boolean') {
                    throw new Error('Missing or invalid verification result');
                }
                break;
        }
        
        return {
            isValid: true,
            data: importedData
        };
        
    } catch (error) {
        return {
            isValid: false,
            error: error.message
        };
    }
}

export {
    createUnsignedMessage,
    signMessage,
    verifyMessage,
    exportMessageData,
    validateImportedMessageData
}; 