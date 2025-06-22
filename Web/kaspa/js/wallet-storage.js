/**
 * Wallet Storage Manager
 * Handles CRUD operations for encrypted wallet storage using localforage
 */

import { walletEncryption } from './wallet-encryption.js';

class WalletStorage {
    constructor() {
        this.storageKey = 'kaspa_wallets';
        this.currentWalletKey = 'kaspa_current_wallet';
        this.initialized = false;
        this.initializationPromise = this.initializeStorage();
    }

    /**
     * Initialize storage configuration
     */
    async initializeStorage() {
        try {
            console.log('Initializing wallet storage...');
            
            // Ensure localforage is available
            if (typeof localforage === 'undefined') {
                console.log('LocalForage not found, loading from CDN...');
                await this.loadLocalForage();
            }
            
            localforage.config({
                driver: [localforage.INDEXEDDB, localforage.WEBSQL, localforage.LOCALSTORAGE],
                name: 'KaspaWallet',
                version: 1.0,
                storeName: 'wallets'
            });
            
            this.initialized = true;
            console.log('Wallet storage initialized successfully');
        } catch (error) {
            console.error('Failed to initialize wallet storage:', error);
        }
    }

    /**
     * Load localforage from CDN if not available
     */
    async loadLocalForage() {
        return new Promise((resolve, reject) => {
            const script = document.createElement('script');
            script.src = 'https://cdn.jsdelivr.net/npm/localforage@1.10.0/dist/localforage.min.js';
            script.onload = resolve;
            script.onerror = reject;
            document.head.appendChild(script);
        });
    }

    /**
     * Ensure storage is initialized
     */
    async ensureInitialized() {
        if (!this.initialized) {
            await this.initializationPromise;
        }
    }

    /**
     * Generate unique wallet ID
     * @param {string} address - Wallet address
     * @param {string} network - Network type
     * @returns {string} Unique wallet ID
     */
    generateWalletId(address, network) {
        return `${network}_${address.substring(0, 10)}...${address.substring(address.length - 10)}`;
    }

    /**
     * Save encrypted wallet to storage
     * @param {Object} walletData - Wallet data to save
     * @param {string} password - Password for encryption
     * @returns {Promise<string>} Wallet ID
     */
    async saveWallet(walletData, password) {
        try {
            await this.ensureInitialized();
            const { privateKey, address, network, mnemonic, derivationPath } = walletData;
            
            // Validate required fields
            if (!privateKey || !address || !network) {
                throw new Error('Missing required wallet data: privateKey, address, or network');
            }

            const walletId = this.generateWalletId(address, network);
            
            // Encrypt the private key
            const encryptedPrivateKey = await walletEncryption.encryptPrivateKey(privateKey, password);
            const serializedEncryptedKey = walletEncryption.serializeEncryptedData(encryptedPrivateKey);
            
            // Encrypt mnemonic if provided
            let encryptedMnemonic = null;
            if (mnemonic) {
                const encryptedMnemonicData = await walletEncryption.encryptPrivateKey(mnemonic, password);
                encryptedMnemonic = walletEncryption.serializeEncryptedData(encryptedMnemonicData);
            }

            // Create wallet entry
            const walletEntry = {
                id: walletId,
                address: address,
                network: network,
                encryptedPrivateKey: serializedEncryptedKey,
                encryptedMnemonic: encryptedMnemonic,
                derivationPath: derivationPath || null,
                createdAt: Date.now(),
                lastUsed: Date.now(),
                label: `Wallet ${address.substring(0, 8)}...`
            };

            // Get existing wallets
            const existingWallets = await this.getAllWallets();
            
            // Check if wallet already exists
            if (existingWallets.some(w => w.id === walletId)) {
                throw new Error('Wallet already exists in storage');
            }

            // Add new wallet
            existingWallets.push(walletEntry);
            
            // Save updated wallets list
            await localforage.setItem(this.storageKey, existingWallets);
            
            console.log(`Wallet saved with ID: ${walletId}`);
            return walletId;
        } catch (error) {
            throw new Error(`Failed to save wallet: ${error.message}`);
        }
    }

    /**
     * Get all wallets (without decrypted private keys)
     * @returns {Promise<Array>} Array of wallet entries
     */
    async getAllWallets() {
        try {
            await this.ensureInitialized();
            const wallets = await localforage.getItem(this.storageKey);
            console.log('Retrieved wallets from storage:', wallets);
            return wallets || [];
        } catch (error) {
            console.error('Failed to get wallets:', error);
            return [];
        }
    }

    /**
     * Get wallet by ID
     * @param {string} walletId - Wallet ID
     * @returns {Promise<Object|null>} Wallet entry or null
     */
    async getWallet(walletId) {
        try {
            const wallets = await this.getAllWallets();
            return wallets.find(w => w.id === walletId) || null;
        } catch (error) {
            console.error('Failed to get wallet:', error);
            return null;
        }
    }

    /**
     * Decrypt wallet private key
     * @param {string} walletId - Wallet ID
     * @param {string} password - Password for decryption
     * @returns {Promise<Object>} Decrypted wallet data
     */
    async decryptWallet(walletId, password) {
        try {
            const wallet = await this.getWallet(walletId);
            if (!wallet) {
                throw new Error('Wallet not found');
            }

            // Decrypt private key
            const encryptedPrivateKey = walletEncryption.deserializeEncryptedData(wallet.encryptedPrivateKey);
            const privateKey = await walletEncryption.decryptPrivateKey(encryptedPrivateKey, password);

            // Decrypt mnemonic if available
            let mnemonic = null;
            if (wallet.encryptedMnemonic) {
                const encryptedMnemonicData = walletEncryption.deserializeEncryptedData(wallet.encryptedMnemonic);
                mnemonic = await walletEncryption.decryptPrivateKey(encryptedMnemonicData, password);
            }

            // Update last used timestamp
            await this.updateLastUsed(walletId);

            return {
                id: wallet.id,
                address: wallet.address,
                network: wallet.network,
                privateKey: privateKey,
                mnemonic: mnemonic,
                derivationPath: wallet.derivationPath,
                label: wallet.label,
                createdAt: wallet.createdAt,
                lastUsed: wallet.lastUsed
            };
        } catch (error) {
            throw new Error(`Failed to decrypt wallet: ${error.message}`);
        }
    }

    /**
     * Update wallet label
     * @param {string} walletId - Wallet ID
     * @param {string} label - New label
     * @returns {Promise<boolean>} Success status
     */
    async updateWalletLabel(walletId, label) {
        try {
            const wallets = await this.getAllWallets();
            const walletIndex = wallets.findIndex(w => w.id === walletId);
            
            if (walletIndex === -1) {
                throw new Error('Wallet not found');
            }

            wallets[walletIndex].label = label;
            await localforage.setItem(this.storageKey, wallets);
            
            return true;
        } catch (error) {
            console.error('Failed to update wallet label:', error);
            return false;
        }
    }

    /**
     * Update last used timestamp
     * @param {string} walletId - Wallet ID
     * @returns {Promise<boolean>} Success status
     */
    async updateLastUsed(walletId) {
        try {
            const wallets = await this.getAllWallets();
            const walletIndex = wallets.findIndex(w => w.id === walletId);
            
            if (walletIndex === -1) {
                return false;
            }

            wallets[walletIndex].lastUsed = Date.now();
            await localforage.setItem(this.storageKey, wallets);
            
            return true;
        } catch (error) {
            console.error('Failed to update last used:', error);
            return false;
        }
    }

    /**
     * Delete wallet from storage
     * @param {string} walletId - Wallet ID
     * @returns {Promise<boolean>} Success status
     */
    async deleteWallet(walletId) {
        try {
            const wallets = await this.getAllWallets();
            const filteredWallets = wallets.filter(w => w.id !== walletId);
            
            if (filteredWallets.length === wallets.length) {
                throw new Error('Wallet not found');
            }

            await localforage.setItem(this.storageKey, filteredWallets);
            
            // Clear current wallet if it was the deleted one
            const currentWalletId = await this.getCurrentWalletId();
            if (currentWalletId === walletId) {
                await this.clearCurrentWallet();
            }
            
            console.log(`Wallet deleted: ${walletId}`);
            return true;
        } catch (error) {
            console.error('Failed to delete wallet:', error);
            return false;
        }
    }

    /**
     * Set current active wallet
     * @param {string} walletId - Wallet ID
     * @returns {Promise<boolean>} Success status
     */
    async setCurrentWallet(walletId) {
        try {
            const wallet = await this.getWallet(walletId);
            if (!wallet) {
                throw new Error('Wallet not found');
            }

            await localforage.setItem(this.currentWalletKey, walletId);
            await this.updateLastUsed(walletId);
            
            return true;
        } catch (error) {
            console.error('Failed to set current wallet:', error);
            return false;
        }
    }

    /**
     * Get current active wallet ID
     * @returns {Promise<string|null>} Current wallet ID or null
     */
    async getCurrentWalletId() {
        try {
            return await localforage.getItem(this.currentWalletKey);
        } catch (error) {
            console.error('Failed to get current wallet ID:', error);
            return null;
        }
    }

    /**
     * Clear current active wallet
     * @returns {Promise<boolean>} Success status
     */
    async clearCurrentWallet() {
        try {
            await localforage.removeItem(this.currentWalletKey);
            return true;
        } catch (error) {
            console.error('Failed to clear current wallet:', error);
            return false;
        }
    }

    /**
     * Check if wallet exists
     * @param {string} address - Wallet address
     * @param {string} network - Network type
     * @returns {Promise<boolean>} Whether wallet exists
     */
    async walletExists(address, network) {
        try {
            const walletId = this.generateWalletId(address, network);
            const wallet = await this.getWallet(walletId);
            return wallet !== null;
        } catch (error) {
            console.error('Failed to check wallet existence:', error);
            return false;
        }
    }

    /**
     * Get wallets by network
     * @param {string} network - Network type
     * @returns {Promise<Array>} Array of wallets for the network
     */
    async getWalletsByNetwork(network) {
        try {
            const wallets = await this.getAllWallets();
            return wallets.filter(w => w.network === network);
        } catch (error) {
            console.error('Failed to get wallets by network:', error);
            return [];
        }
    }

    /**
     * Clear all wallets (dangerous operation)
     * @returns {Promise<boolean>} Success status
     */
    async clearAllWallets() {
        try {
            await localforage.removeItem(this.storageKey);
            await localforage.removeItem(this.currentWalletKey);
            console.log('All wallets cleared');
            return true;
        } catch (error) {
            console.error('Failed to clear all wallets:', error);
            return false;
        }
    }

    /**
     * Export wallet data (encrypted) for backup
     * @param {string} walletId - Wallet ID
     * @returns {Promise<Object>} Backup data
     */
    async exportWalletBackup(walletId) {
        try {
            const wallet = await this.getWallet(walletId);
            if (!wallet) {
                throw new Error('Wallet not found');
            }

            return {
                type: 'kaspa_wallet_backup',
                version: '1.0',
                wallet: wallet,
                exportedAt: Date.now()
            };
        } catch (error) {
            throw new Error(`Failed to export wallet: ${error.message}`);
        }
    }

    /**
     * Import wallet from backup
     * @param {Object} backupData - Backup data
     * @returns {Promise<string>} Imported wallet ID
     */
    async importWalletBackup(backupData) {
        try {
            if (backupData.type !== 'kaspa_wallet_backup') {
                throw new Error('Invalid backup format');
            }

            const wallet = backupData.wallet;
            const wallets = await this.getAllWallets();
            
            // Check if wallet already exists
            if (wallets.some(w => w.id === wallet.id)) {
                throw new Error('Wallet already exists in storage');
            }

            // Add imported wallet
            wallets.push(wallet);
            await localforage.setItem(this.storageKey, wallets);
            
            console.log(`Wallet imported with ID: ${wallet.id}`);
            return wallet.id;
        } catch (error) {
            throw new Error(`Failed to import wallet: ${error.message}`);
        }
    }
}

// Create singleton instance
const walletStorage = new WalletStorage();

export { walletStorage, WalletStorage }; 