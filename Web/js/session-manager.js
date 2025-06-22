/**
 * Session Manager for Kaspa Wallet
 * Handles session persistence, timeout, and automatic logout
 */

export class SessionManager {
    constructor() {
        this.sessionKey = 'kaspa_wallet_session';
        this.settingsKey = 'kaspa_session_settings';
        this.timeoutId = null;
        this.onSessionExpired = null;
        this.defaultSettings = {
            timeoutMinutes: 0, // 0 = no timeout (current behavior)
            autoSave: true
        };
    }

    /**
     * Get session timeout settings
     * @returns {Object} Session settings
     */
    getSettings() {
        try {
            const saved = localStorage.getItem(this.settingsKey);
            return saved ? JSON.parse(saved) : this.defaultSettings;
        } catch (error) {
            console.error('Failed to load session settings:', error);
            return this.defaultSettings;
        }
    }

    /**
     * Save session timeout settings
     * @param {Object} settings - Session settings
     */
    saveSettings(settings) {
        try {
            localStorage.setItem(this.settingsKey, JSON.stringify(settings));
        } catch (error) {
            console.error('Failed to save session settings:', error);
        }
    }

    /**
     * Save wallet session data
     * @param {Object} walletState - Current wallet state
     */
    saveSession(walletState) {
        const settings = this.getSettings();
        
        if (!settings.autoSave || settings.timeoutMinutes === 0) {
            return;
        }

        try {
            // Create a safe copy of currentWallet without circular references
            const safeCurrentWallet = walletState.currentWallet ? {
                id: walletState.currentWallet.id,
                name: walletState.currentWallet.name,
                address: walletState.currentWallet.address,
                network: walletState.currentWallet.network,
                mnemonic: walletState.currentWallet.mnemonic,
                derivationPath: walletState.currentWallet.derivationPath,
                privateKey: walletState.currentWallet.privateKey
            } : null;

            // Create safe copy of addresses array with only essential serializable data
            const safeAllAddresses = walletState.allAddresses ? walletState.allAddresses.map(addr => ({
                address: addr.address || '',
                type: addr.type || 'receive',
                index: typeof addr.index === 'number' ? addr.index : 0,
                balance: typeof addr.balance === 'number' ? addr.balance : (typeof addr.balance === 'bigint' ? Number(addr.balance) : 0),
                used: Boolean(addr.used)
            })) : [];

            const sessionData = {
                walletState: {
                    isLoggedIn: walletState.isLoggedIn,
                    currentWallet: safeCurrentWallet,
                    address: walletState.address,
                    network: walletState.network,
                    allAddresses: safeAllAddresses,
                    isHDWallet: walletState.isHDWallet,
                },
                timestamp: Date.now(),
                expiresAt: Date.now() + (settings.timeoutMinutes * 60 * 1000)
            };

            const jsonString = JSON.stringify(sessionData);
            localStorage.setItem(this.sessionKey, jsonString);
            
            this.startTimeout(settings.timeoutMinutes);
        } catch (error) {
            console.error('Failed to save session:', error);
            console.error('Error details:', error.message);
        }
    }

    /**
     * Load and validate saved session
     * @returns {Object|null} Restored wallet state or null if expired/invalid
     */
    loadSession() {
        try {
            const saved = localStorage.getItem(this.sessionKey);
            if (!saved) {
                return null;
            }

            const sessionData = JSON.parse(saved);
            const now = Date.now();

            // Check if session has expired
            if (sessionData.expiresAt && now > sessionData.expiresAt) {
                this.clearSession();
                return null;
            }

            // Calculate remaining time and start timeout
            const settings = this.getSettings();
            if (settings.timeoutMinutes > 0) {
                const remainingMs = sessionData.expiresAt - now;
                const remainingMinutes = Math.ceil(remainingMs / (60 * 1000));
                this.startTimeout(remainingMinutes);
            }

            return sessionData.walletState;
        } catch (error) {
            console.error('Failed to load session:', error);
            this.clearSession();
            return null;
        }
    }

    /**
     * Clear saved session data
     */
    clearSession() {
        try {
            localStorage.removeItem(this.sessionKey);
            this.stopTimeout();
        } catch (error) {
            console.error('Failed to clear session:', error);
        }
    }

    /**
     * Start session timeout
     * @param {number} minutes - Timeout in minutes
     */
    startTimeout(minutes) {
        this.stopTimeout(); // Clear any existing timeout

        if (minutes <= 0) return; // No timeout

        const timeoutMs = minutes * 60 * 1000;
        this.timeoutId = setTimeout(() => {
            this.handleSessionExpired();
        }, timeoutMs);
    }

    /**
     * Stop session timeout
     */
    stopTimeout() {
        if (this.timeoutId) {
            clearTimeout(this.timeoutId);
            this.timeoutId = null;
        }
    }

    /**
     * Handle session expiration
     */
    handleSessionExpired() {
        this.clearSession();
        
        if (this.onSessionExpired) {
            this.onSessionExpired();
        }
    }

    /**
     * Set session expiration callback
     * @param {Function} callback - Function to call when session expires
     */
    setSessionExpiredCallback(callback) {
        this.onSessionExpired = callback;
    }

    /**
     * Reset session timeout (call on user activity)
     */
    resetTimeout() {
        const settings = this.getSettings();
        if (settings.timeoutMinutes > 0) {
            this.startTimeout(settings.timeoutMinutes);
        }
    }

    /**
     * Check if session persistence is enabled
     * @returns {boolean} True if sessions should be saved
     */
    isSessionPersistenceEnabled() {
        const settings = this.getSettings();
        return settings.autoSave && settings.timeoutMinutes > 0;
    }

    /**
     * Get remaining session time in minutes
     * @returns {number} Minutes remaining, or -1 if no timeout
     */
    getRemainingTime() {
        try {
            const saved = localStorage.getItem(this.sessionKey);
            if (!saved) return -1;

            const sessionData = JSON.parse(saved);
            if (!sessionData.expiresAt) return -1;

            const remainingMs = sessionData.expiresAt - Date.now();
            return Math.max(0, Math.ceil(remainingMs / (60 * 1000)));
        } catch (error) {
            return -1;
        }
    }
}

export default SessionManager; 