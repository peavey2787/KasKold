/**
 * Centralized Balance Manager
 * Provides two main methods for balance calculation:
 * 1. Calculate balance from UTXOs (offline/cached)
 * 2. Calculate balance using Kaspa WASM SDK RPC (online)
 */

import { getKaspa, isInitialized } from './init.js';
import { sompiToKas, kasToSompi } from './currency-utils.js';

export class BalanceManager {
    constructor() {
        this.rpc = null;
    }

    /**
     * Initialize RPC connection for online balance checking
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
     * Method 1: Calculate balance from UTXOs (offline/cached)
     * This is the canonical method for UTXO-based balance calculation
     * @param {Array} utxos - Array of UTXO objects with 'amount' field
     * @returns {Object} Balance calculation result
     */
    calculateBalanceFromUTXOs(utxos) {
        if (!utxos || utxos.length === 0) {
            return {
                success: true,
                totalBalance: 0,
                totalBalanceKas: '0.00000000',
                totalBalanceSompi: 0n,
                utxoCount: 0
            };
        }

        try {
            // Sum all UTXO amounts in sompi
            const totalBalanceSompi = utxos.reduce((sum, utxo) => {
                const amount = BigInt(utxo.amount || utxo.value || 0);
                return sum + amount;
            }, 0n);

            // Convert to KAS string using precise conversion
            const totalBalanceKas = sompiToKas(totalBalanceSompi);

            return {
                success: true,
                totalBalance: parseFloat(totalBalanceKas),
                totalBalanceKas: totalBalanceKas,
                totalBalanceSompi: totalBalanceSompi,
                utxoCount: utxos.length
            };
        } catch (error) {
            console.error('Error calculating balance from UTXOs:', error);
            return {
                success: false,
                error: error.message,
                totalBalance: 0,
                totalBalanceKas: '0.00000000',
                totalBalanceSompi: 0n,
                utxoCount: 0
            };
        }
    }

    /**
     * Method 2: Get balance using Kaspa WASM SDK RPC (online)
     * Uses the official getBalanceByAddress method from Kaspa WASM SDK
     * @param {string} address - Address to check balance for
     * @param {string} networkType - Network type
     * @param {boolean} reuseConnection - Whether to reuse existing RPC connection
     * @returns {Object} Balance result from RPC
     */
    async getBalanceByAddressRPC(address, networkType, reuseConnection = false) {
        try {
            // Only initialize RPC if we don't have a connection or not reusing
            if (!this.rpc || !reuseConnection) {
                await this.initializeRpc(networkType);
            }

            // Use the official getBalanceByAddress method
            const balanceResponse = await this.rpc.getBalanceByAddress({ address });

            if (!balanceResponse || balanceResponse.balance === undefined) {
                return {
                    success: true,
                    address: address,
                    totalBalance: 0,
                    totalBalanceKas: '0.00000000',
                    totalBalanceSompi: 0n,
                    networkType: networkType
                };
            }

            const balanceSompi = BigInt(balanceResponse.balance);
            const balanceKas = sompiToKas(balanceSompi);

            return {
                success: true,
                address: address,
                totalBalance: parseFloat(balanceKas),
                totalBalanceKas: balanceKas,
                totalBalanceSompi: balanceSompi,
                networkType: networkType
            };

        } catch (error) {
            return {
                success: false,
                error: error.message,
                address: address,
                totalBalance: 0,
                totalBalanceKas: '0.00000000',
                totalBalanceSompi: 0n,
                networkType: networkType
            };
        } finally {
            // Only cleanup if not reusing connection
            if (!reuseConnection) {
                await this.cleanup();
            }
        }
    }

    /**
     * Get balance for multiple addresses using RPC
     * @param {Array<string>} addresses - Array of addresses to check
     * @param {string} networkType - Network type
     * @returns {Object} Balance result for all addresses
     */
    async getBalanceForAddresses(addresses, networkType) {
        try {
            await this.initializeRpc(networkType);

            const balanceResponse = await this.rpc.getBalancesByAddresses(addresses);
            let totalBalanceSompi = 0n;
            const addressBalances = [];

            if (balanceResponse && balanceResponse.entries) {
                for (let i = 0; i < balanceResponse.entries.length; i++) {
                    const entry = balanceResponse.entries[i];
                    const address = addresses[i];
                    
                    const balanceSompi = entry && entry.balance ? BigInt(entry.balance) : 0n;
                    const balanceKas = sompiToKas(balanceSompi);
                    
                    totalBalanceSompi += balanceSompi;
                    
                    addressBalances.push({
                        address: address,
                        balance: parseFloat(balanceKas),
                        balanceKas: balanceKas,
                        balanceSompi: balanceSompi
                    });
                }
            }

            const totalBalanceKas = sompiToKas(totalBalanceSompi);

            return {
                success: true,
                addressBalances: addressBalances,
                totalBalance: parseFloat(totalBalanceKas),
                totalBalanceKas: totalBalanceKas,
                totalBalanceSompi: totalBalanceSompi,
                networkType: networkType
            };

        } catch (error) {
            return {
                success: false,
                error: error.message,
                addressBalances: [],
                totalBalance: 0,
                totalBalanceKas: '0.00000000',
                totalBalanceSompi: 0n,
                networkType: networkType
            };
        } finally {
            await this.cleanup();
        }
    }

    /**
     * Validate if UTXOs are sufficient for a transaction
     * @param {Array} utxos - Array of UTXO objects
     * @param {BigInt|string|number} requiredAmountSompi - Required amount in sompi
     * @param {BigInt|string|number} requiredFeeSompi - Required fee in sompi (default: 0)
     * @returns {Object} Validation result
     */
    validateUTXOsSufficient(utxos, requiredAmountSompi, requiredFeeSompi = 0n) {
        const balanceResult = this.calculateBalanceFromUTXOs(utxos);
        
        if (!balanceResult.success) {
            return {
                sufficient: false,
                availableBalance: 0n,
                requiredAmount: BigInt(requiredAmountSompi) + BigInt(requiredFeeSompi),
                shortfall: BigInt(requiredAmountSompi) + BigInt(requiredFeeSompi),
                error: balanceResult.error
            };
        }

        const totalRequired = BigInt(requiredAmountSompi) + BigInt(requiredFeeSompi);
        
        return {
            sufficient: balanceResult.totalBalanceSompi >= totalRequired,
            availableBalance: balanceResult.totalBalanceSompi,
            requiredAmount: totalRequired,
            shortfall: balanceResult.totalBalanceSompi < totalRequired ? 
                      totalRequired - balanceResult.totalBalanceSompi : 0n
        };
    }

    /**
     * Convert balance result to consistent format
     * @param {Object} balanceResult - Balance result from any method
     * @returns {Object} Standardized balance result
     */
    standardizeBalanceResult(balanceResult) {
        if (!balanceResult || !balanceResult.success) {
            return {
                success: false,
                error: balanceResult?.error || 'Unknown balance calculation error',
                balance: 0,
                balanceKas: '0.00000000',
                balanceSompi: 0n
            };
        }

        // Ensure consistent property names
        const sompi = balanceResult.totalBalanceSompi || balanceResult.balanceSompi || 0n;
        const kasString = balanceResult.totalBalanceKas || balanceResult.balanceKas || sompiToKas(sompi);
        const kasNumber = balanceResult.totalBalance || balanceResult.balance || parseFloat(kasString);

        return {
            success: true,
            balance: kasNumber,
            balanceKas: kasString,
            balanceSompi: sompi,
            utxoCount: balanceResult.utxoCount || 0
        };
    }
}

// Create singleton instance
export const balanceManager = new BalanceManager();

// Convenience exports for the two main methods
export const calculateBalanceFromUTXOs = (utxos) => balanceManager.calculateBalanceFromUTXOs(utxos);
export const getBalanceByAddressRPC = (address, networkType, reuseConnection = false) => 
    balanceManager.getBalanceByAddressRPC(address, networkType, reuseConnection);
export const getBalanceForAddresses = (addresses, networkType) => 
    balanceManager.getBalanceForAddresses(addresses, networkType);
export const validateUTXOsSufficient = (utxos, requiredAmountSompi, requiredFeeSompi = 0n) => 
    balanceManager.validateUTXOsSufficient(utxos, requiredAmountSompi, requiredFeeSompi); 