/**
 * Unified Wallet Manager for Kaspa
 * Consolidates HD and single address wallet functionality
 * Implements DRY principles for balance checking, UTXO management, and address derivation
 */

import { getKaspa, isInitialized } from './init.js';
import { sompiToKas, kasToSompi } from './currency-utils.js';
import { DEFAULT_ACCOUNT_PATH, buildDerivationPath } from './constants.js';

export class UnifiedWalletManager {
    constructor(mnemonic, network, derivationPath = DEFAULT_ACCOUNT_PATH, isHDWallet = true) {
        this.mnemonic = mnemonic;
        this.network = network;
        this.derivationPath = derivationPath;
        this.isHDWallet = isHDWallet;
        this.addresses = {
            receive: new Map(),
            change: new Map()
        };
        this.currentReceiveIndex = 0;
        this.currentChangeIndex = 0;
        this.totalBalance = 0n;
        this.kaspa = null;
        this.xPrv = null;
        this.rpc = null;
    }

    /**
     * Initialize the wallet manager
     */
    async initialize() {
        if (!isInitialized()) {
            throw new Error('Kaspa WASM not initialized');
        }
        
        this.kaspa = getKaspa();
        
        if (this.isHDWallet) {
            const { Mnemonic, XPrv } = this.kaspa;
            const mnemonicObj = new Mnemonic(this.mnemonic);
            const seed = mnemonicObj.toSeed();
            this.xPrv = new XPrv(seed);
            
            // Generate initial receive address
            await this.generateNextReceiveAddress();
        }
    }

    /**
     * Initialize RPC connection (reusable)
     */
    async initializeRpc() {
        if (this.rpc) return this.rpc;

        const { Resolver, RpcClient } = this.kaspa;
        const resolver = new Resolver();
        this.rpc = new RpcClient({
            networkId: this.network,
            resolver: resolver
        });

        await this.rpc.connect();
        return this.rpc;
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
     * Check balance for HD wallet using proper derivation and RPC calls
     * Based on the working method from your external app
     */
    async checkHDWalletBalance() {
        if (!this.isHDWallet) {
            throw new Error('This method is only for HD wallets');
        }

        try {
            await this.initializeRpc();
            
            // Import currency conversion function
            const { sompiToKas } = await import('./currency-utils.js');
            
            const { Mnemonic, XPrv, createAddress } = this.kaspa;
            
            // Create wallet from mnemonic using the working method
            const mnemonic = new Mnemonic(this.mnemonic);
            const seed = mnemonic.toSeed();
            const xPrv = new XPrv(seed);
            
            let totalBalance = 0n;
            const addressesWithBalance = [];
            const gapLimit = 20;
            
            // Scan both receive (0) and change (1) addresses
            for (let change = 0; change <= 1; change++) {
                let consecutiveEmpty = 0;
                let index = 0;
                
                while (consecutiveEmpty < gapLimit && index < 100) {
                    // Use the proper derivation path format from constants
                    const fullPath = buildDerivationPath(this.derivationPath.split('/')[3], change, index);
                    const addressXPrv = xPrv.derivePath(fullPath);
                    const privateKey = addressXPrv.toPrivateKey();
                    const address = createAddress(privateKey.toPublicKey(), this.network);
                    const addressString = address.toString();
                                        
                    // Use centralized balance manager for consistent balance checking
                    const { getBalanceByAddressRPC } = await import('./balance-manager.js');
                    const balanceResult = await getBalanceByAddressRPC(addressString, this.network, true);
                    
                    let addressBalance = 0n;
                    const utxos = [];
                    
                    if (balanceResult.success && balanceResult.totalBalanceSompi > 0n) {
                        addressBalance = balanceResult.totalBalanceSompi;
                        
                        // Also get UTXOs for additional info
                        const utxoResponse = await this.rpc.getUtxosByAddresses([addressString]);
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
                    }
                    
                    if (addressBalance > 0n) {
                        totalBalance += addressBalance;
                        consecutiveEmpty = 0;
                                                 
                        addressesWithBalance.push({
                            address: addressString,
                            balance: sompiToKas(addressBalance),
                            balanceSompi: addressBalance,
                            change: change,
                            index: index,
                            utxos: utxos,
                            utxoCount: utxos.length,
                            derivationPath: fullPath
                        });
                        
                        // Update internal address tracking
                        const addressMap = change === 0 ? this.addresses.receive : this.addresses.change;
                        addressMap.set(index, {
                            address: addressString,
                            index: index,
                            type: change === 0 ? 'receive' : 'change',
                            derivationPath: fullPath,
                            privateKey: privateKey.toString(),
                            used: true,
                            balance: addressBalance,
                            utxos: utxos
                        });
                    } else {
                        consecutiveEmpty++;
                    }
                    
                    index++;                    
                }
            }
            
            this.totalBalance = totalBalance;
            
            const totalBalanceKas = sompiToKas(totalBalance);
              
            return {
                success: true,
                totalBalance: totalBalanceKas,
                totalBalanceSompi: totalBalance,
                addressesFound: addressesWithBalance.length,
                addressesWithBalance: addressesWithBalance
            };
            
        } catch (error) {
            console.error('HD wallet balance check failed:', error);
            return {
                success: false,
                error: error.message,
                totalBalance: 0,
                totalBalanceSompi: 0n,
                addressesFound: 0,
                addressesWithBalance: []
            };
        } finally {
            await this.cleanup();
        }
    }

    /**
     * Check balance for single address wallet
     */
    async checkSingleAddressBalance(address) {
        try {
            await this.initializeRpc();
            
            // Use centralized balance manager for consistent balance checking
            const { getBalanceByAddressRPC } = await import('./balance-manager.js');
            const balanceResult = await getBalanceByAddressRPC(address, this.network, true);
            
            if (!balanceResult.success || balanceResult.totalBalanceSompi === 0n) {
                // No balance found or error
                return {
                    success: true,
                    address: address,
                    balance: { kas: 0, sompi: 0n },
                    utxoCount: 0,
                    utxos: [],
                    networkType: this.network
                };
            }
            
            // Address has balance, now get UTXOs
            const utxoResponse = await this.rpc.getUtxosByAddresses([address]);
            
            const utxos = [];
            
            if (utxoResponse && utxoResponse.entries) {
                for (const entry of utxoResponse.entries) {
                    // Handle WASM objects vs traditional structures (same as UTXO fetcher)
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
                address: address,
                balance: {
                    kas: balanceResult.totalBalance,
                    sompi: balanceResult.totalBalanceSompi
                },
                utxoCount: utxos.length,
                utxos: utxos,
                networkType: this.network
            };
            
        } catch (error) {
            return {
                success: false,
                error: error.message,
                address: address
            };
        } finally {
            await this.cleanup();
        }
    }

    /**
     * Generate xpub for UTXO fetching operations (external tools)
     * This should ONLY be used for UTXO fetching, not balance refresh
     */
    async getXPub() {
        if (!this.isHDWallet) {
            throw new Error('XPUB generation only available for HD wallets');
        }

        try {
            const accountXPrv = this.xPrv.derivePath(this.derivationPath);
            const xpub = accountXPrv.toXPub();
            
            // Handle different xpub formats
            if (typeof xpub === 'object' && xpub !== null) {
                if (xpub.xpub) {
                    return xpub.xpub;
                } else if (xpub.toString) {
                    return xpub.toString();
                }
            }
            
            return xpub.toString();
        } catch (error) {
            throw new Error('Failed to generate XPUB: ' + error.message);
        }
    }

    /**
     * Fetch UTXOs for multiple addresses (for transaction creation)
     */
    async fetchUTXOsForAddresses(addresses) {
        try {
            await this.initializeRpc();
            
            const utxoResponse = await this.rpc.getUtxosByAddresses(addresses);
            const allUtxos = [];
            
            if (utxoResponse && utxoResponse.entries) {
                for (const entry of utxoResponse.entries) {
                    // Handle WASM objects vs traditional structures (same as UTXO fetcher)
                    if (entry && typeof entry === 'object' && 
                        (entry.__wbg_ptr || entry.constructor?.name === 'UtxoEntryReference')) {
                        // This is a WASM UtxoEntryReference object - add it directly as a UTXO
                        allUtxos.push(entry);
                    } else if (entry && entry.utxoEntries) {
                        // Traditional structure with utxoEntries array
                        for (const utxo of entry.utxoEntries) {
                            if (utxo) {
                                allUtxos.push(utxo);
                            }
                        }
                    } else if (entry && entry.entry) {
                        // Handle case where entry is wrapped in another entry object
                        const innerEntry = entry.entry;
                        if (innerEntry && typeof innerEntry === 'object' && 
                            (innerEntry.__wbg_ptr || innerEntry.constructor?.name === 'UtxoEntryReference')) {
                            allUtxos.push(innerEntry);
                        }
                    }
                }
            }
            
            return {
                success: true,
                utxos: allUtxos,
                count: allUtxos.length
            };
            
        } catch (error) {
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
     * Generate next receive address (HD wallets only)
     */
    async generateNextReceiveAddress() {
        if (!this.isHDWallet) {
            throw new Error('Address generation only available for HD wallets');
        }

        const addressInfo = await this.generateAddress('receive', this.currentReceiveIndex);
        this.addresses.receive.set(this.currentReceiveIndex, {
            ...addressInfo,
            used: false,
            balance: 0n,
            utxos: []
        });
        
        this.currentReceiveIndex++;
        return addressInfo;
    }

    /**
     * Generate next change address (HD wallets only)
     */
    async generateNextChangeAddress() {
        if (!this.isHDWallet) {
            throw new Error('Address generation only available for HD wallets');
        }

        const addressInfo = await this.generateAddress('change', this.currentChangeIndex);
        this.addresses.change.set(this.currentChangeIndex, {
            ...addressInfo,
            used: false,
            balance: 0n,
            utxos: []
        });
        
        this.currentChangeIndex++;
        return addressInfo;
    }

    /**
     * Generate address at specific index
     */
    async generateAddress(type, index) {
        if (!this.isHDWallet) {
            throw new Error('Address generation only available for HD wallets');
        }

        const typeIndex = type === 'receive' ? 0 : 1;
        const fullPath = `${this.derivationPath}/${typeIndex}/${index}`;
        
        const derivedXPrv = this.xPrv.derivePath(fullPath);
        const privateKey = derivedXPrv.toPrivateKey();
        const address = privateKey.toPublicKey().toAddress(this.network).toString();
        
        return {
            address,
            index,
            type,
            derivationPath: fullPath,
            privateKey: privateKey.toString()
        };
    }

    /**
     * Get current receive address (ensures it has no UTXOs)
     */
    async getCurrentReceiveAddress() {
        if (!this.isHDWallet) {
            throw new Error('HD address methods only available for HD wallets');
        }

        // Always check if we need a new address before returning current one
        if (await this.shouldGenerateNewReceiveAddress()) {
            await this.generateNextReceiveAddress();
        }

        if (this.addresses.receive.size === 0) {
            throw new Error('No receive addresses generated');
        }

        const latestIndex = this.currentReceiveIndex - 1;
        const currentAddress = this.addresses.receive.get(latestIndex);

        // Double-check that this address has no UTXOs
        if (currentAddress && await this.addressHasUTXOs(currentAddress.address)) {
            console.warn('🚨 SECURITY: Current receive address has UTXOs, generating new one');
            await this.generateNextReceiveAddress();
            const newLatestIndex = this.currentReceiveIndex - 1;
            return this.addresses.receive.get(newLatestIndex)?.address;
        }

        return currentAddress?.address;
    }

    /**
     * Get current change address (generate one if none exists)
     */
    async getCurrentChangeAddress() {
        if (!this.isHDWallet) {
            throw new Error('HD address methods only available for HD wallets');
        }

        if (this.addresses.change.size === 0) {
            // Generate first change address if none exists
            const changeAddressInfo = await this.generateNextChangeAddress();
            return changeAddressInfo.address;
        }
        
        const latestIndex = this.currentChangeIndex - 1;
        return this.addresses.change.get(latestIndex)?.address;
    }

    /**
     * Get all addresses with balances
     */
    getAllAddresses() {
        const addresses = [];
        
        for (const addressInfo of this.addresses.receive.values()) {
            addresses.push(addressInfo);
        }
        
        for (const addressInfo of this.addresses.change.values()) {
            addresses.push(addressInfo);
        }
        
        return addresses;
    }

    /**
     * Get total balance
     */
    getTotalBalance() {
        console.log('🏦 HD WALLET: getTotalBalance() called, returning:', this.totalBalance);
        console.log('🏦 HD WALLET: Balance type:', typeof this.totalBalance);
        return this.totalBalance;
    }

    /**
     * Reset all cached balances to 0 (for fresh discovery)
     */
    resetAllBalances() {
        console.log('🧹 HD WALLET: Resetting all cached balances to 0');

        // Reset all receive address balances
        for (const [index, addressInfo] of this.addresses.receive.entries()) {
            addressInfo.balance = 0n;
            addressInfo.utxos = [];
            console.log(`  Reset receive ${index}: ${addressInfo.address} - Balance: 0`);
        }

        // Reset all change address balances
        for (const [index, addressInfo] of this.addresses.change.entries()) {
            addressInfo.balance = 0n;
            addressInfo.utxos = [];
            console.log(`  Reset change ${index}: ${addressInfo.address} - Balance: 0`);
        }

        // Reset total balance
        this.totalBalance = 0n;
        console.log('🧹 HD WALLET: All balances reset to 0');
    }

    /**
     * Mark address as used
     */
    markAddressAsUsed(address) {
        for (const [index, addressInfo] of this.addresses.receive) {
            if (addressInfo.address === address) {
                addressInfo.used = true;
                break;
            }
        }
        
        for (const [index, addressInfo] of this.addresses.change) {
            if (addressInfo.address === address) {
                addressInfo.used = true;
                break;
            }
        }
    }

    /**
     * Update address balance
     */
    updateAddressBalance(address, balance, utxos = []) {
        // Ensure balance is BigInt
        const balanceBigInt = typeof balance === 'bigint' ? balance : BigInt(balance.toString());
        
        for (const addressInfo of this.addresses.receive.values()) {
            if (addressInfo.address === address) {
                addressInfo.balance = balanceBigInt;
                addressInfo.utxos = utxos;
                break;
            }
        }
        
        for (const addressInfo of this.addresses.change.values()) {
            if (addressInfo.address === address) {
                addressInfo.balance = balanceBigInt;
                addressInfo.utxos = utxos;
                break;
            }
        }
        
        this.recalculateTotalBalance();
    }

    /**
     * Recalculate total balance from all addresses
     */
    recalculateTotalBalance() {
        console.log('🧮 HD WALLET: Recalculating total balance...');
        let total = 0n;
        let addressCount = 0;

        console.log('🧮 HD WALLET: Checking receive addresses...');
        for (const [index, addressInfo] of this.addresses.receive.entries()) {
            const balance = addressInfo.balance;
            console.log(`  Receive ${index}: ${addressInfo.address} - Balance: ${balance}`);
            if (balance) {
                // Ensure balance is BigInt
                const balanceBigInt = typeof balance === 'bigint' ? balance : BigInt(balance.toString());
                total += balanceBigInt;
                addressCount++;
            }
        }

        console.log('🧮 HD WALLET: Checking change addresses...');
        for (const [index, addressInfo] of this.addresses.change.entries()) {
            const balance = addressInfo.balance;
            console.log(`  Change ${index}: ${addressInfo.address} - Balance: ${balance}`);
            if (balance) {
                // Ensure balance is BigInt
                const balanceBigInt = typeof balance === 'bigint' ? balance : BigInt(balance.toString());
                total += balanceBigInt;
                addressCount++;
            }
        }

        this.totalBalance = total;
        console.log(`🧮 HD WALLET: Total balance calculated: ${total} sompi from ${addressCount} addresses`);
    }

    /**
     * Check if we should generate a new receive address for privacy and security
     */
    async shouldGenerateNewReceiveAddress() {
        if (!this.isHDWallet) {
            return false;
        }

        if (this.addresses.receive.size === 0) {
            return true; // No addresses generated yet
        }

        const latestIndex = this.currentReceiveIndex - 1;
        const currentAddress = this.addresses.receive.get(latestIndex);

        if (!currentAddress) {
            return true;
        }

        // Check if address is marked as used
        if (currentAddress.used) {
            return true;
        }

        // Check if address has any balance
        if (currentAddress.balance && currentAddress.balance > 0n) {
            return true;
        }

        // Most important: Check if address has any UTXOs (even if balance is 0)
        if (await this.addressHasUTXOs(currentAddress.address)) {
            console.log('🚨 SECURITY: Current receive address has UTXOs, need new address');
            return true;
        }

        return false;
    }

    /**
     * Check if an address has any UTXOs (critical for security)
     */
    async addressHasUTXOs(address) {
        try {
            await this.initializeRpc();
            const utxoResponse = await this.rpc.getUtxosByAddresses([address]);

            if (utxoResponse && utxoResponse.entries && utxoResponse.entries.length > 0) {
                for (const entry of utxoResponse.entries) {
                    if (entry && entry.utxoEntries && entry.utxoEntries.length > 0) {
                        console.log(`🚨 SECURITY: Address ${address} has ${entry.utxoEntries.length} UTXOs`);
                        return true;
                    }
                }
            }

            return false;
        } catch (error) {
            console.error('Error checking UTXOs for address:', error);
            // If we can't check, assume it has UTXOs for safety
            return true;
        }
    }

    /**
     * Import wallet state from saved data
     */
    async importState(savedState) {
        if (!this.isHDWallet) {
            throw new Error('State import only available for HD wallets');
        }

        if (savedState.addresses) {
            // Import receive addresses (but reset balances to 0 for fresh discovery)
            if (savedState.addresses.receive) {
                for (const [index, addressInfo] of Object.entries(savedState.addresses.receive)) {
                    this.addresses.receive.set(parseInt(index), {
                        ...addressInfo,
                        balance: 0n, // Reset balance to 0 - will be fetched fresh
                        utxos: [] // Clear cached UTXOs - will be fetched fresh
                    });
                }
            }

            // Import change addresses (but reset balances to 0 for fresh discovery)
            if (savedState.addresses.change) {
                for (const [index, addressInfo] of Object.entries(savedState.addresses.change)) {
                    this.addresses.change.set(parseInt(index), {
                        ...addressInfo,
                        balance: 0n, // Reset balance to 0 - will be fetched fresh
                        utxos: [] // Clear cached UTXOs - will be fetched fresh
                    });
                }
            }
        }
        
        // Set current indices
        if (savedState.currentReceiveIndex !== undefined) {
            this.currentReceiveIndex = savedState.currentReceiveIndex;
        }
        
        if (savedState.currentChangeIndex !== undefined) {
            this.currentChangeIndex = savedState.currentChangeIndex;
        }
        
        // Reset total balance to 0 instead of recalculating from potentially stale cached data
        this.totalBalance = 0n;
        console.log('🧮 HD WALLET: Reset total balance to 0 after state import (cached balances cleared)');
    }
}

// Create singleton instances for different wallet types
let hdWalletInstance = null;
let singleWalletInstance = null;

/**
 * Get or create HD wallet instance
 */
export function getHDWallet(mnemonic, network, derivationPath) {
    if (!hdWalletInstance ||
        hdWalletInstance.mnemonic !== mnemonic ||
        hdWalletInstance.network !== network ||
        hdWalletInstance.derivationPath !== derivationPath) {
        hdWalletInstance = new UnifiedWalletManager(mnemonic, network, derivationPath, true);
    }

    // Always reset cached balances to ensure fresh discovery
    hdWalletInstance.resetAllBalances();

    return hdWalletInstance;
}

/**
 * Get or create single address wallet instance
 */
export function getSingleWallet(address, network) {
    if (!singleWalletInstance || 
        singleWalletInstance.network !== network) {
        singleWalletInstance = new UnifiedWalletManager(null, network, null, false);
        singleWalletInstance.primaryAddress = address;
    }
    return singleWalletInstance;
}

/**
 * Clear wallet instances (for logout)
 */
export function clearWalletInstances() {
    hdWalletInstance = null;
    singleWalletInstance = null;
} 