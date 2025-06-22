/**
 * Wallet Encryption/Decryption Utilities
 * Handles secure encryption and decryption of wallet private keys
 */

class WalletEncryption {
    constructor() {
        this.algorithm = 'AES-GCM';
        this.keyLength = 256;
        this.ivLength = 12; // 96 bits for GCM
        this.saltLength = 16; // 128 bits
        this.iterations = 100000; // PBKDF2 iterations
    }

    /**
     * Generate a random salt
     * @returns {Uint8Array} Random salt
     */
    generateSalt() {
        return crypto.getRandomValues(new Uint8Array(this.saltLength));
    }

    /**
     * Generate a random IV
     * @returns {Uint8Array} Random IV
     */
    generateIV() {
        return crypto.getRandomValues(new Uint8Array(this.ivLength));
    }

    /**
     * Derive key from password using PBKDF2
     * @param {string} password - User password
     * @param {Uint8Array} salt - Salt for key derivation
     * @returns {Promise<CryptoKey>} Derived key
     */
    async deriveKey(password, salt) {
        const encoder = new TextEncoder();
        const passwordBuffer = encoder.encode(password);
        
        const importedKey = await crypto.subtle.importKey(
            'raw',
            passwordBuffer,
            'PBKDF2',
            false,
            ['deriveKey']
        );

        return await crypto.subtle.deriveKey(
            {
                name: 'PBKDF2',
                salt: salt,
                iterations: this.iterations,
                hash: 'SHA-256'
            },
            importedKey,
            {
                name: this.algorithm,
                length: this.keyLength
            },
            false,
            ['encrypt', 'decrypt']
        );
    }

    /**
     * Encrypt private key with password
     * @param {string} privateKey - Private key to encrypt
     * @param {string} password - User password
     * @returns {Promise<Object>} Encrypted data with salt and IV
     */
    async encryptPrivateKey(privateKey, password) {
        try {
            const encoder = new TextEncoder();
            const data = encoder.encode(privateKey);
            
            const salt = this.generateSalt();
            const iv = this.generateIV();
            
            const key = await this.deriveKey(password, salt);
            
            const encrypted = await crypto.subtle.encrypt(
                {
                    name: this.algorithm,
                    iv: iv
                },
                key,
                data
            );

            return {
                encrypted: new Uint8Array(encrypted),
                salt: salt,
                iv: iv,
                algorithm: this.algorithm,
                iterations: this.iterations
            };
        } catch (error) {
            throw new Error(`Encryption failed: ${error.message}`);
        }
    }

    /**
     * Decrypt private key with password
     * @param {Object} encryptedData - Encrypted data object
     * @param {string} password - User password
     * @returns {Promise<string>} Decrypted private key
     */
    async decryptPrivateKey(encryptedData, password) {
        try {
            const { encrypted, salt, iv, algorithm, iterations } = encryptedData;
            
            // Verify algorithm and iterations match
            if (algorithm !== this.algorithm || iterations !== this.iterations) {
                throw new Error('Incompatible encryption parameters');
            }
            
            const key = await this.deriveKey(password, salt);
            
            const decrypted = await crypto.subtle.decrypt(
                {
                    name: this.algorithm,
                    iv: iv
                },
                key,
                encrypted
            );

            const decoder = new TextDecoder();
            return decoder.decode(decrypted);
        } catch (error) {
            if (error.name === 'OperationError') {
                throw new Error('Invalid password or corrupted data');
            }
            throw new Error(`Decryption failed: ${error.message}`);
        }
    }

    /**
     * Convert encrypted data to JSON-serializable format
     * @param {Object} encryptedData - Encrypted data object
     * @returns {Object} Serializable encrypted data
     */
    serializeEncryptedData(encryptedData) {
        return {
            encrypted: Array.from(encryptedData.encrypted),
            salt: Array.from(encryptedData.salt),
            iv: Array.from(encryptedData.iv),
            algorithm: encryptedData.algorithm,
            iterations: encryptedData.iterations,
            timestamp: Date.now()
        };
    }

    /**
     * Convert serialized data back to encrypted data format
     * @param {Object} serializedData - Serialized encrypted data
     * @returns {Object} Encrypted data object
     */
    deserializeEncryptedData(serializedData) {
        return {
            encrypted: new Uint8Array(serializedData.encrypted),
            salt: new Uint8Array(serializedData.salt),
            iv: new Uint8Array(serializedData.iv),
            algorithm: serializedData.algorithm,
            iterations: serializedData.iterations,
            timestamp: serializedData.timestamp
        };
    }

    /**
     * Generate a secure random password
     * @param {number} length - Password length (default: 32)
     * @returns {string} Random password
     */
    generateSecurePassword(length = 32) {
        const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*';
        const array = new Uint8Array(length);
        crypto.getRandomValues(array);
        
        return Array.from(array, byte => charset[byte % charset.length]).join('');
    }

    /**
     * Validate password strength
     * @param {string} password - Password to validate
     * @returns {Object} Validation result with score and feedback
     */
    validatePasswordStrength(password) {
        const result = {
            score: 0,
            feedback: [],
            isValid: false
        };

        if (password.length < 8) {
            result.feedback.push('Password must be at least 8 characters long');
        } else {
            result.score += 1;
        }

        if (password.length >= 12) {
            result.score += 1;
        }

        if (/[a-z]/.test(password)) {
            result.score += 1;
        } else {
            result.feedback.push('Password should contain lowercase letters');
        }

        if (/[A-Z]/.test(password)) {
            result.score += 1;
        } else {
            result.feedback.push('Password should contain uppercase letters');
        }

        if (/\d/.test(password)) {
            result.score += 1;
        } else {
            result.feedback.push('Password should contain numbers');
        }

        if (/[^a-zA-Z\d]/.test(password)) {
            result.score += 1;
        } else {
            result.feedback.push('Password should contain special characters');
        }

        result.isValid = result.score >= 4 && password.length >= 8;

        if (result.isValid) {
            result.feedback = ['Password strength is good'];
        }

        return result;
    }
}

// Create singleton instance
const walletEncryption = new WalletEncryption();

export { walletEncryption, WalletEncryption }; 