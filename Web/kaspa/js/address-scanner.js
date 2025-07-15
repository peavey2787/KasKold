/**
 * Address Scanner Module
 * Handles scanning for addresses with balances from extended public keys (xpub)
 * Based on working implementation from user's web app
 */

import { getKaspa, isInitialized } from './init.js';

export class AddressScanner {
    constructor() {
        this.rpc = null;
    }

    /**
     * Initialize RPC connection
     * @param {string} networkType - Network type (mainnet, testnet-10, etc.)
     */
    async initializeRpc(networkType) {
        if (!isInitialized()) {
            throw new Error('Kaspa WASM not initialized');
        }

        const kaspa = getKaspa();
        const { Resolver, RpcClient } = kaspa;

        const resolver = new Resolver();
        this.rpc = new RpcClient({
            networkId: networkType,
            resolver: resolver
        });

        try {
            await this.rpc.connect();
            return this.rpc;
        } catch (error) {
            throw new Error(`Network connection failed for ${networkType}: ${error.message}`);
        }
    }

    /**
     * Clean up RPC connection
     */
    async cleanup() {
        if (this.rpc) {
            await this.rpc.disconnect();
            this.rpc = null;
        }
    }

    /**
     * Generate xpub from HD wallet using unified wallet manager
     * This should ONLY be used for UTXO fetching operations, not balance refresh
     * @param {object} hdWallet - HD wallet object
     * @returns {string} Extended public key
     */
    async generateXpubFromHDWallet(hdWallet) {
        try {
            // Use the unified wallet manager for consistent XPUB generation
            const { getHDWallet } = await import('./wallet-manager.js');
            
            const unifiedWallet = getHDWallet(
                hdWallet.mnemonic,
                hdWallet.network || 'mainnet',
                hdWallet.derivationPath
            );
            
            await unifiedWallet.initialize();
            return await unifiedWallet.getXPub();

        } catch (error) {
            throw new Error('Failed to generate extended public key: ' + error.message);
        }
    }

    /**
     * Check balance for a single address using centralized balance manager
     * @param {string} address - Address to check
     * @param {string} networkType - Network type
     * @param {boolean} reuseConnection - Whether to reuse existing RPC connection
     * @returns {object} Balance result
     */
    async checkAddressBalance(address, networkType, reuseConnection = false) {
        try {

            // Use centralized balance manager for consistent balance checking
            const { getBalanceByAddressRPC } = await import('./balance-manager.js');
            const balanceResult = await getBalanceByAddressRPC(address, networkType, reuseConnection);

            if (!balanceResult.success) {
                return {
                    success: false,
                    error: balanceResult.error,
                    balance: 0,
                    balanceSompi: 0n,
                    utxoCount: 0,
                    utxos: [],
                    networkError: true
                };
            }

            if (balanceResult.totalBalanceSompi === 0n) {
                return {
                    success: true,
                    balance: 0,
                    balanceSompi: 0n,
                    utxoCount: 0,
                    utxos: []
                };
            }
            
            // Get UTXOs for additional info if balance found
            const utxos = [];
            try {
                // Only initialize RPC if we don't have a connection or not reusing
                if (!this.rpc || !reuseConnection) {
                    await this.initializeRpc(networkType);
                }

                const utxoResponse = await this.rpc.getUtxosByAddresses([address]);
                if (utxoResponse && utxoResponse.entries) {
                    for (const entry of utxoResponse.entries) {
                        if (entry && entry.utxoEntries) {
                            for (const utxo of entry.utxoEntries) {
                                if (utxo && utxo.amount) {
                                    utxos.push(utxo);
                                }
                            }
                        }
                    }
                }
            } catch (utxoError) {
                console.warn(`Warning: Could not fetch UTXOs for ${address}:`, utxoError.message);
                // Continue without UTXOs - balance is still valid
            }

            return {
                success: true,
                balance: balanceResult.totalBalance,
                balanceSompi: balanceResult.totalBalanceSompi,
                utxoCount: utxos.length,
                utxos: utxos
            };

        } catch (error) {
            console.error(`❌ ADDRESS SCANNER: Error checking balance for ${address}:`, error);
            return {
                success: false,
                error: error.message,
                balance: 0,
                balanceSompi: 0n,
                utxoCount: 0,
                utxos: [],
                networkError: true
            };
        } finally {
            // Only cleanup if not reusing connection
            if (!reuseConnection) {
                await this.cleanup();
            }
        }
    }

    /**
     * Derive address from xpub using the working pattern
     * @param {string} xpub - Extended public key
     * @param {number} change - 0 for receive, 1 for change
     * @param {number} index - Address index
     * @param {string} networkType - Network type
     * @returns {string} Derived address
     */
    async deriveAddressFromXpub(xpub, change, index, networkType) {
        if (!isInitialized()) {
            throw new Error('Kaspa WASM not initialized');
        }

        const kaspa = getKaspa();
        const { XPub } = kaspa;

        const xPubObj = new XPub(xpub);

        // Use the exact derivation pattern from working web app
        const changeXPub = xPubObj.deriveChild(change, false);
        const indexXPub = changeXPub.deriveChild(index, false);
        const publicKey = indexXPub.toPublicKey();
        const addressObj = publicKey.toAddress(networkType);
        const address = addressObj.toString();

        return address;
    }

    /**
     * Derive multiple addresses from xpub using specific indices (like the working web app)
     * @param {string} xpub - Extended public key
     * @param {string} networkType - Network type
     * @param {Array<number>} indices - Array of indices to derive
     * @param {number} change - Change value (0 for receiving, 1 for change addresses)
     * @param {Function} balanceChecker - Function to check balance
     * @returns {Promise<{success: boolean, addresses?: Array, error?: string}>}
     */
    async deriveAddressesWithBalance(xpub, networkType, indices = [1, 2, 5, 10, 20, 50, 100], change = 0, balanceChecker = null) {
        try {

            const kaspa = getKaspa();
            const { XPub } = kaspa;
            const xPubObj = new XPub(xpub);
            const addresses = [];

            // Initialize RPC connection once
            await this.initializeRpc(networkType);

            for (const index of indices) {
                try {
                    // Derive using specified change pattern (exact same as working web app)
                    const changeXPub = xPubObj.deriveChild(change, false);
                    const indexXPub = changeXPub.deriveChild(index, false);
                    const publicKey = indexXPub.toPublicKey();
                    const addressObj = publicKey.toAddress(networkType);
                    const address = addressObj.toString();

                    let balance = null;
                    let balanceSompi = 0n;
                    let utxos = [];

                    if (balanceChecker) {
                        const balanceResult = await balanceChecker(address, networkType);
                        if (balanceResult.success) {
                            balance = balanceResult.balance;
                            balanceSompi = balanceResult.balanceSompi || 0n;
                            utxos = balanceResult.utxos || [];
                        }
                    } else {
                        // Use built-in balance checker
                        const balanceResult = await this.checkAddressBalance(address, networkType, true);
                        if (balanceResult.success) {
                            balance = balanceResult.balance;
                            balanceSompi = balanceResult.balanceSompi || 0n;
                            utxos = balanceResult.utxos || [];
                        }
                    }

                    addresses.push({
                        index: index,
                        address: address,
                        change: change,
                        balance: balance,
                        balanceSompi: balanceSompi,
                        utxos: utxos
                    });

                } catch (error) {
                    // Silently continue on derivation errors
                }
            }

            return {
                success: true,
                addresses: addresses
            };

        } catch (error) {
            return {
                success: false,
                error: error.message
            };
        } finally {
            await this.cleanup();
        }
    }

    /**
     * Scan xpub using the working web app approach with specific indices
     * @param {string} xpub - Extended public key
     * @param {string} networkType - Network type
     * @param {Array<number>} indices - Specific indices to test
     * @param {function} progressCallback - Progress callback
     * @returns {object} Scan result
     */
    async scanXpubWithSpecificIndices(xpub, networkType, indices = [0, 1, 2, 5, 10, 20, 50, 100], progressCallback = null) {
        try {
            const addressesWithBalance = [];
            let totalBalance = 0n;
            let networkErrorCount = 0;

            // Initialize RPC connection once at the start
            await this.initializeRpc(networkType);

            if (progressCallback) {
                progressCallback({
                    current: 0,
                    total: indices.length * 2, // Both receive and change
                    status: 'Starting targeted address scan...'
                });
            }



            // Scan both receive (0) and change (1) addresses for each index
            for (let change = 0; change <= 1; change++) {
                const changeType = change === 0 ? 'receive' : 'change';

                for (let i = 0; i < indices.length; i++) {
                    const index = indices[i];

                    if (progressCallback) {
                        const current = (change * indices.length) + i;
                        const total = indices.length * 2;
                        progressCallback({
                            current: current,
                            total: total,
                            status: `Checking ${changeType} address at index ${index}...`
                        });
                    }

                    try {
                        // Derive address using the exact same method as working web app
                        const address = await this.deriveAddressFromXpub(xpub, change, index, networkType);

                        // Check balance (reuse connection)
                        const balanceResult = await this.checkAddressBalance(address, networkType, true);

                        if (balanceResult.success && balanceResult.balanceSompi > 0n) {
                            addressesWithBalance.push({
                                address: address,
                                balance: balanceResult.balance,
                                balanceSompi: balanceResult.balanceSompi,
                                change: change,
                                index: index,
                                utxos: balanceResult.utxos,
                                utxoCount: balanceResult.utxoCount
                            });

                            totalBalance += balanceResult.balanceSompi;

                        } else {
                            // Check for network errors
                            if (balanceResult.networkError) {
                                networkErrorCount++;
                            }
                        }

                    } catch (error) {
                        networkErrorCount++;
                    }
                }
            }

            if (progressCallback) {
                progressCallback({
                    current: 100,
                    total: 100,
                    status: `Targeted scan complete: ${addressesWithBalance.length} addresses found`
                });
            }



            return {
                success: true,
                addressesWithBalance: addressesWithBalance,
                totalBalance: parseFloat(await import('./currency-utils.js').then(m => m.sompiToKas(totalBalance))),
                totalBalanceSompi: totalBalance,
                addressesFound: addressesWithBalance.length,
                networkErrors: networkErrorCount,
                indicesScanned: indices
            };

        } catch (error) {
            console.error('Error in targeted xpub scan:', error);
            return {
                success: false,
                error: error.message,
                addressesWithBalance: [],
                totalBalance: 0,
                totalBalanceSompi: 0n,
                addressesFound: 0
            };
        } finally {
            // Clean up RPC connection after scanning is complete
            await this.cleanup();
        }
    }

    /**
     * Scan xpub for addresses with balances in a range
     * @param {string} xpub - Extended public key
     * @param {string} networkType - Network type
     * @param {number} maxScan - Maximum addresses to scan
     * @param {function} progressCallback - Progress callback
     * @returns {object} Scan result
     */
    async scanXpubRange(xpub, networkType, maxScan = 100, progressCallback = null) {
        try {
            const addressesWithBalance = [];
            let totalBalance = 0n;
            let consecutiveEmpty = 0;
            let networkErrorCount = 0;
            const GAP_LIMIT = 20;

            // Initialize RPC connection once at the start
            await this.initializeRpc(networkType);

            if (progressCallback) {
                progressCallback({
                    current: 0,
                    total: maxScan,
                    status: 'Starting address scan...'
                });
            }

            // Scan both receive (0) and change (1) addresses
            for (let change = 0; change <= 1; change++) {
                const changeType = change === 0 ? 'receive' : 'change';
                consecutiveEmpty = 0;

                for (let index = 0; index < maxScan && consecutiveEmpty < GAP_LIMIT; index++) {
                    if (progressCallback) {
                        const current = (change * maxScan) + index;
                        const total = maxScan * 2;
                        progressCallback({
                            current: Math.min(current, total),
                            total: total,
                            status: `Checking ${changeType} address ${index}...`
                        });
                    }

                    try {
                        // Derive address
                        const address = await this.deriveAddressFromXpub(xpub, change, index, networkType);

                        // Check balance (reuse connection)
                        const balanceResult = await this.checkAddressBalance(address, networkType, true);

                        if (balanceResult.success && balanceResult.balanceSompi > 0n) {
                            addressesWithBalance.push({
                                address: address,
                                balance: balanceResult.balance,
                                balanceSompi: balanceResult.balanceSompi,
                                change: change,
                                index: index,
                                utxos: balanceResult.utxos,
                                utxoCount: balanceResult.utxoCount
                            });

                            totalBalance += balanceResult.balanceSompi;
                            consecutiveEmpty = 0; // Reset gap counter
                        } else {
                            consecutiveEmpty++;
                            
                            // Check for network errors
                            if (balanceResult.networkError) {
                                networkErrorCount++;
                            }
                        }

                    } catch (error) {
                        consecutiveEmpty++;
                    }
                }
            }

            if (progressCallback) {
                progressCallback({
                    current: 100,
                    total: 100,
                    status: `Scan complete: ${addressesWithBalance.length} addresses found`
                });
            }



            return {
                success: true,
                addressesWithBalance: addressesWithBalance,
                totalBalance: parseFloat(await import('./currency-utils.js').then(m => m.sompiToKas(totalBalance))),
                totalBalanceSompi: totalBalance,
                addressesFound: addressesWithBalance.length,
                networkErrors: networkErrorCount
            };

        } catch (error) {
            return {
                success: false,
                error: error.message,
                addressesWithBalance: [],
                totalBalance: 0,
                totalBalanceSompi: 0n,
                addressesFound: 0
            };
        } finally {
            // Clean up RPC connection after scanning is complete
            await this.cleanup();
        }
    }

    /**
     * Perform comprehensive discovery for HD wallets
     * @param {string} xpub - Extended public key
     * @param {string} networkType - Network type
     * @param {number} gapLimit - Gap limit for scanning
     * @param {function} progressCallback - Progress callback
     * @returns {object} Discovery result
     */
    async performComprehensiveDiscovery(xpub, networkType, gapLimit = 20, progressCallback = null) {
        try {
            const scanResult = await this.scanXpubRange(xpub, networkType, gapLimit * 2, progressCallback);

            if (!scanResult.success) {
                return {
                    success: false,
                    error: scanResult.error
                };
            }

            return {
                success: true,
                result: {
                    addressesFound: scanResult.addressesFound,
                    totalBalance: scanResult.totalBalance,
                    totalBalanceSompi: scanResult.totalBalanceSompi,
                    addressesWithBalance: scanResult.addressesWithBalance,
                    receiveAddresses: scanResult.addressesWithBalance.filter(addr => addr.change === 0),
                    changeAddresses: scanResult.addressesWithBalance.filter(addr => addr.change === 1)
                }
            };

        } catch (error) {
            return {
                success: false,
                error: error.message
            };
        }
    }

    /**
     * Fetch UTXOs for a single address
     * @param {string} address - Address to fetch UTXOs for
     * @param {string} networkType - Network type
     * @returns {object} UTXO result
     */
    async fetchUTXOsForAddress(address, networkType) {
        try {
            await this.initializeRpc(networkType);
            
            // Validate the address format
            const kaspa = getKaspa();
            const { Address } = kaspa;
            try {
                new Address(address);
            } catch (error) {
                throw new Error(`Invalid address format: ${error.message}`);
            }
            
            // Use centralized balance manager to check if address has balance
            const { getBalanceByAddressRPC } = await import('./balance-manager.js');
            const balanceResult = await getBalanceByAddressRPC(address, networkType, true);
            
            if (!balanceResult.success || balanceResult.totalBalanceSompi === 0n) {
                // No balance found, return empty result
                return {
                    success: true,
                    utxos: [],
                    count: 0,
                    address: address,
                    networkType: networkType
                };
            }
            
            // Address has balance, now get UTXOs
            const utxoResponse = await this.rpc.getUtxosByAddresses([address]);
            
            const utxos = [];
            if (utxoResponse && utxoResponse.entries) {
                for (const entry of utxoResponse.entries) {
                    // Handle WASM objects vs traditional structures
                    if (entry && typeof entry === 'object' && 
                        (entry.__wbg_ptr || entry.constructor?.name === 'UtxoEntryReference')) {
                        // This is a WASM UtxoEntryReference object - add it directly as a UTXO
                        utxos.push(entry);
                    } else if (entry && entry.utxoEntries) {
                        // Traditional structure with utxoEntries array
                        for (const utxo of entry.utxoEntries) {
                            if (utxo) {
                                utxos.push(utxo);
                            }
                        }
                    } else if (entry && entry.entry) {
                        // Handle case where entry is wrapped in another entry object
                        const innerEntry = entry.entry;
                        if (innerEntry && typeof innerEntry === 'object' && 
                            (innerEntry.__wbg_ptr || innerEntry.constructor?.name === 'UtxoEntryReference')) {
                            utxos.push(innerEntry);
                        }
                    }
                }
            }

            return {
                success: true,
                utxos: utxos,
                count: utxos.length,
                address: address,
                networkType: networkType
            };

        } catch (error) {
            return {
                success: false,
                error: error.message || 'Failed to fetch UTXOs',
                utxos: [],
                count: 0,
                address: address,
                networkType: networkType
            };
        } finally {
            await this.cleanup();
        }
    }

    /**
     * Fetch UTXOs for multiple addresses
     * @param {array} addresses - Array of addresses
     * @param {string} networkType - Network type
     * @returns {object} UTXO result
     */
    async fetchUTXOsForAddresses(addresses, networkType) {
        
        try {
            await this.initializeRpc(networkType);
            const allUtxos = [];

            // Use the same approach as wallet manager: check balances first, then get UTXOs
            for (const address of addresses) {
                try {
                    
                    // Use centralized balance manager to check if address has balance
                    const { getBalanceByAddressRPC } = await import('./balance-manager.js');
                    const balanceResult = await getBalanceByAddressRPC(address, networkType, true);
                    
                    if (balanceResult.success && balanceResult.totalBalanceSompi > 0n) {
                        
                        // Address has balance, now get UTXOs
                        const utxoResponse = await this.rpc.getUtxosByAddresses([address]);
                        
                        if (utxoResponse && utxoResponse.entries) {
                            
                            for (const entry of utxoResponse.entries) {
                                
                                // Handle Kaspa WASM UtxoEntryReference objects
                                if (entry) {
                                    // For WASM objects, the entry itself might be the UTXO
                                    // or it might have properties we need to access differently
                                    try {
                                        
                                        // The entry itself is the UTXO for WASM objects
                                        if (entry.__wbg_ptr || entry.constructor.name.includes('Utxo')) {
                                            allUtxos.push(entry);
                                        }
                                        
                                        // Also try the traditional approach for backwards compatibility
                                        if (entry.utxoEntries) {
                                            
                                            for (const utxo of entry.utxoEntries) {
                                                if (utxo) {
                                                    allUtxos.push(utxo);
                                                }
                                            }
                                        }
                                    } catch (wasmError) {
                                        // Fallback: treat the entry as a UTXO itself
                                        if (entry) {
                                            allUtxos.push(entry);
                                        }
                                    }
                                } 
                            }
                        }
                    }
                } catch (error) {
                    console.warn(`❌ Error fetching UTXOs for address ${address}:`, error);
                    // Continue with other addresses
                }
            }
            
            return {
                success: true,
                utxos: allUtxos,
                count: allUtxos.length
            };

        } catch (error) {
            console.error('❌ Error in fetchUTXOsForAddresses:', error);
            return {
                success: false,
                error: error.message,
                utxos: [],
                count: 0
            };
        } finally {
            await this.cleanup();
        }
    }
    /**
     * Fetch transaction details and its UTXOs
     * @param {string} transactionId - Transaction ID
     * @param {string} networkType - Network type
     * @returns {object} Transaction UTXO result
     */
    async fetchTransactionUTXOs(transactionId, networkType) {
        try {
            await this.initializeRpc(networkType);
            
            // Get transaction details
            const transaction = await this.rpc.getTransaction(transactionId);

            if (!transaction) {
                throw new Error(`Transaction ${transactionId} not found`);
            }

            // Extract input and output UTXOs from transaction
            const inputUtxos = transaction.inputs || [];
            const outputUtxos = transaction.outputs || [];

            return {
                success: true,
                transaction: transaction,
                inputUtxos: inputUtxos,
                outputUtxos: outputUtxos,
                transactionId: transactionId,
                networkType: networkType
            };

        } catch (error) {
            return {
                success: false,
                error: error.message || 'Failed to fetch transaction UTXOs',
                transaction: null,
                inputUtxos: [],
                outputUtxos: [],
                transactionId: transactionId,
                networkType: networkType
            };
        } finally {
            await this.cleanup();
        }
    }
}

// Re-export centralized balance calculation functions
export { 
    calculateBalanceFromUTXOs, 
    validateUTXOsSufficient 
} from './balance-manager.js';

// Create singleton instance
export const addressScanner = new AddressScanner(); 

// Convenience exports for backward compatibility
export const fetchUTXOsForAddress = (address, networkType) => addressScanner.fetchUTXOsForAddress(address, networkType);
export const fetchUTXOsForAddresses = (addresses, networkType) => addressScanner.fetchUTXOsForAddresses(addresses, networkType);
export const fetchTransactionUTXOs = (transactionId, networkType) => addressScanner.fetchTransactionUTXOs(transactionId, networkType); 