/**
 * Wallet Manager
 * Business logic for wallet management UI and operations
 */

import { walletStorage } from './wallet-storage.js';
import { walletEncryption } from './wallet-encryption.js';

class WalletManager {
    constructor() {
        this.isLoggedIn = false;
        this.currentWallet = null;
        this.currentPassword = null;
        this.sessionTimeout = 30 * 60 * 1000; // 30 minutes
        this.sessionTimer = null;
        this.listeners = [];
        this.initialize();
    }

    /**
     * Initialize wallet manager
     */
    async initialize() {
        // Check if there's a current wallet set
        await this.checkCurrentWallet();
        
        // Set up session management
        this.setupSessionManagement();
        
        console.log('Wallet Manager initialized');
    }

    /**
     * Add event listener for wallet changes
     * @param {Function} callback - Callback function
     */
    addWalletChangeListener(callback) {
        this.listeners.push(callback);
    }

    /**
     * Remove event listener
     * @param {Function} callback - Callback function to remove
     */
    removeWalletChangeListener(callback) {
        this.listeners = this.listeners.filter(l => l !== callback);
    }

    /**
     * Notify listeners of wallet changes
     * @param {string} event - Event type
     * @param {Object} data - Event data
     */
    notifyListeners(event, data = null) {
        this.listeners.forEach(callback => {
            try {
                callback(event, data);
            } catch (error) {
                console.error('Error in wallet change listener:', error);
            }
        });
    }

    /**
     * Save a new wallet with encryption
     * @param {Object} walletData - Wallet data
     * @param {string} password - Wallet password
     * @param {string} label - Optional wallet label
     * @returns {Promise<string>} Wallet ID
     */
    async saveWallet(walletData, password, label = null) {
        try {
            // Validate password strength
            const passwordValidation = walletEncryption.validatePasswordStrength(password);
            if (!passwordValidation.isValid) {
                throw new Error(`Weak password: ${passwordValidation.feedback.join(', ')}`);
            }

            // Save wallet
            const walletId = await walletStorage.saveWallet(walletData, password);
            
            // Update label if provided
            if (label) {
                await walletStorage.updateWalletLabel(walletId, label);
            }

            // Notify listeners
            this.notifyListeners('wallet_saved', { walletId, address: walletData.address });
            
            // Auto-login if no current wallet
            if (!this.isLoggedIn) {
                await this.loginWallet(walletId, password);
            }

            return walletId;
        } catch (error) {
            throw new Error(`Failed to save wallet: ${error.message}`);
        }
    }

    /**
     * Login to a wallet
     * @param {string} walletId - Wallet ID
     * @param {string} password - Wallet password
     * @returns {Promise<Object>} Decrypted wallet data
     */
    async loginWallet(walletId, password) {
        try {
            // Decrypt wallet
            const decryptedWallet = await walletStorage.decryptWallet(walletId, password);
            
            // Set as current wallet
            await walletStorage.setCurrentWallet(walletId);
            
            // Update session
            this.currentWallet = decryptedWallet;
            this.currentPassword = password;
            this.isLoggedIn = true;
            
            // Reset session timer
            this.resetSessionTimer();
            
            // Notify listeners
            this.notifyListeners('wallet_login', decryptedWallet);
            
            // Update UI
            this.updateWalletManagerUI();
            this.updateCurrentWalletDisplay();
            
            return decryptedWallet;
        } catch (error) {
            throw new Error(`Login failed: ${error.message}`);
        }
    }

    /**
     * Logout from current wallet
     */
    async logoutWallet() {
        try {
            // Clear session
            this.currentWallet = null;
            this.currentPassword = null;
            this.isLoggedIn = false;
            
            // Clear session timer
            if (this.sessionTimer) {
                clearTimeout(this.sessionTimer);
                this.sessionTimer = null;
            }
            
            // Clear current wallet in storage
            await walletStorage.clearCurrentWallet();
            
            // Notify listeners
            this.notifyListeners('wallet_logout');
            
            // Update UI
            this.updateWalletManagerUI();
            this.updateCurrentWalletDisplay();
            
            console.log('Logged out successfully');
        } catch (error) {
            console.error('Logout error:', error);
        }
    }

    /**
     * Delete a wallet
     * @param {string} walletId - Wallet ID
     * @returns {Promise<boolean>} Success status
     */
    async deleteWallet(walletId) {
        try {
            // If deleting current wallet, logout first
            if (this.currentWallet && this.currentWallet.id === walletId) {
                await this.logoutWallet();
            }
            
            // Delete wallet
            const success = await walletStorage.deleteWallet(walletId);
            
            if (success) {
                // Notify listeners
                this.notifyListeners('wallet_deleted', { walletId });
                
                // Update UI
                this.updateWalletManagerUI();
            }
            
            return success;
        } catch (error) {
            console.error('Delete wallet error:', error);
            return false;
        }
    }

    /**
     * Get current wallet info
     * @returns {Object|null} Current wallet data
     */
    getCurrentWallet() {
        return this.currentWallet;
    }

    /**
     * Check if logged in
     * @returns {boolean} Login status
     */
    isWalletLoggedIn() {
        return this.isLoggedIn && this.currentWallet !== null;
    }

    /**
     * Get available wallets for dropdown
     * @returns {Promise<Array>} Array of wallet options
     */
    async getWalletOptions() {
        try {
            const wallets = await walletStorage.getAllWallets();
            return wallets.map(wallet => ({
                id: wallet.id,
                label: wallet.label,
                address: wallet.address,
                network: wallet.network,
                lastUsed: wallet.lastUsed,
                createdAt: wallet.createdAt
            }));
        } catch (error) {
            console.error('Failed to get wallet options:', error);
            return [];
        }
    }

    /**
     * Check current wallet from storage
     */
    async checkCurrentWallet() {
        try {
            const currentWalletId = await walletStorage.getCurrentWalletId();
            if (currentWalletId) {
                const wallet = await walletStorage.getWallet(currentWalletId);
                if (wallet) {
                    // Wallet exists but user needs to login
                    this.notifyListeners('wallet_needs_login', { walletId: currentWalletId, wallet });
                }
            }
        } catch (error) {
            console.error('Error checking current wallet:', error);
        }
    }

    /**
     * Setup session management
     */
    setupSessionManagement() {
        // Auto-logout on page visibility change
        document.addEventListener('visibilitychange', () => {
            if (document.hidden && this.isLoggedIn) {
                // Optionally auto-logout when tab becomes hidden
                // this.logoutWallet();
            }
        });

        // Auto-logout before page unload
        window.addEventListener('beforeunload', () => {
            if (this.isLoggedIn) {
                // Clear sensitive data from memory
                this.currentPassword = null;
            }
        });
    }

    /**
     * Reset session timer
     */
    resetSessionTimer() {
        if (this.sessionTimer) {
            clearTimeout(this.sessionTimer);
        }
        
        this.sessionTimer = setTimeout(() => {
            if (this.isLoggedIn) {
                console.log('Session expired - auto logout');
                this.logoutWallet();
                this.showWalletManagerStatus('Session expired. Please login again.', 'info');
            }
        }, this.sessionTimeout);
    }

    /**
     * Update wallet manager UI
     */
    async updateWalletManagerUI() {
        const walletSelect = document.getElementById('wallet-select');
        const loginSection = document.getElementById('wallet-login-section');
        const loggedInSection = document.getElementById('wallet-logged-in-section');
        const currentWalletInfo = document.getElementById('current-wallet-info-manager');

        if (!walletSelect) return; // UI not loaded yet

        try {
            // Get wallet options
            const walletOptions = await this.getWalletOptions();
            
            // Update wallet dropdown
            walletSelect.innerHTML = '<option value="">Select a wallet...</option>';
            walletOptions.forEach(wallet => {
                const option = document.createElement('option');
                option.value = wallet.id;
                option.textContent = `${wallet.label} (${wallet.network})`;
                walletSelect.appendChild(option);
            });

            if (this.isLoggedIn && this.currentWallet) {
                // Show logged in state
                loginSection.style.display = 'none';
                loggedInSection.style.display = 'block';
                
                // Update current wallet info
                document.getElementById('current-wallet-label').textContent = this.currentWallet.label || 'Unknown';
                document.getElementById('current-wallet-address').textContent = this.currentWallet.address;
                document.getElementById('current-wallet-network').textContent = this.currentWallet.network;
                
                // Select current wallet in dropdown
                walletSelect.value = this.currentWallet.id;
            } else {
                // Show login state
                loginSection.style.display = 'block';
                loggedInSection.style.display = 'none';
            }
        } catch (error) {
            console.error('Failed to update wallet manager UI:', error);
        }
    }

    /**
     * Update current wallet display in other sections
     */
    updateCurrentWalletDisplay() {
        // Update wallet displays in other sections
        const walletDisplays = [
            'current-wallet-info',
            'current-wallet-info-msg',
            'balance-wallet-info'
        ];

        walletDisplays.forEach(displayId => {
            const display = document.getElementById(displayId);
            if (display) {
                if (this.isLoggedIn && this.currentWallet) {
                    display.style.display = 'block';
                    const addressSpan = display.querySelector('[id$="-address"], [id$="-address-msg"]');
                    const networkSpan = display.querySelector('[id$="-network"], [id$="-network-msg"]');
                    
                    if (addressSpan) addressSpan.textContent = this.currentWallet.address;
                    if (networkSpan) networkSpan.textContent = this.currentWallet.network;
                } else {
                    display.style.display = 'none';
                }
            }
        });

        // Enable/disable buttons based on login status
        this.updateButtonStates();
    }

    /**
     * Update button states based on login status
     */
    updateButtonStates() {
        const buttonsToEnable = [
            'checkBalance',
            'calculateFee',
            'generateUnsignedTxQR',
            'signMessage',
            'generateUnsignedQR'
        ];

        buttonsToEnable.forEach(buttonId => {
            const button = document.getElementById(buttonId);
            if (button) {
                button.disabled = !this.isLoggedIn;
            }
        });
    }

    /**
     * Show status message in wallet manager
     * @param {string} message - Status message
     * @param {string} type - Message type (success, error, info)
     */
    showWalletManagerStatus(message, type = 'info') {
        const statusDiv = document.getElementById('wallet-manager-status');
        if (statusDiv) {
            statusDiv.textContent = message;
            statusDiv.className = `status-container ${type}`;
            statusDiv.style.display = 'block';
            
            // Auto-hide after 5 seconds
            setTimeout(() => {
                statusDiv.style.display = 'none';
            }, 5000);
        }
    }

    /**
     * Export wallet backup
     * @param {string} walletId - Wallet ID
     * @returns {Promise<Object>} Backup data
     */
    async exportWalletBackup(walletId) {
        try {
            return await walletStorage.exportWalletBackup(walletId);
        } catch (error) {
            throw new Error(`Export failed: ${error.message}`);
        }
    }

    /**
     * Import wallet backup
     * @param {Object} backupData - Backup data
     * @returns {Promise<string>} Imported wallet ID
     */
    async importWalletBackup(backupData) {
        try {
            const walletId = await walletStorage.importWalletBackup(backupData);
            
            // Update UI
            await this.updateWalletManagerUI();
            
            // Notify listeners
            this.notifyListeners('wallet_imported', { walletId });
            
            return walletId;
        } catch (error) {
            throw new Error(`Import failed: ${error.message}`);
        }
    }

    /**
     * Change wallet password
     * @param {string} walletId - Wallet ID
     * @param {string} oldPassword - Current password
     * @param {string} newPassword - New password
     * @returns {Promise<boolean>} Success status
     */
    async changeWalletPassword(walletId, oldPassword, newPassword) {
        try {
            // Validate new password
            const passwordValidation = walletEncryption.validatePasswordStrength(newPassword);
            if (!passwordValidation.isValid) {
                throw new Error(`Weak password: ${passwordValidation.feedback.join(', ')}`);
            }

            // Decrypt with old password
            const decryptedWallet = await walletStorage.decryptWallet(walletId, oldPassword);
            
            // Delete old wallet
            await walletStorage.deleteWallet(walletId);
            
            // Save with new password
            const newWalletId = await walletStorage.saveWallet({
                privateKey: decryptedWallet.privateKey,
                address: decryptedWallet.address,
                network: decryptedWallet.network,
                mnemonic: decryptedWallet.mnemonic,
                derivationPath: decryptedWallet.derivationPath
            }, newPassword);
            
            // Update label
            await walletStorage.updateWalletLabel(newWalletId, decryptedWallet.label);
            
            // Update current session if this is the active wallet
            if (this.currentWallet && this.currentWallet.id === walletId) {
                this.currentWallet.id = newWalletId;
                this.currentPassword = newPassword;
                await walletStorage.setCurrentWallet(newWalletId);
            }
            
            // Notify listeners
            this.notifyListeners('wallet_password_changed', { walletId: newWalletId });
            
            return true;
        } catch (error) {
            throw new Error(`Password change failed: ${error.message}`);
        }
    }
}

// Create singleton instance
const walletManager = new WalletManager();

export { walletManager, WalletManager }; 