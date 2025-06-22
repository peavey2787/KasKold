/**
 * HD Balance Manager for Kaspa
 * Handles balance checking and UTXO management across multiple addresses
 */

import { checkAddressBalance } from './wallet-balance.js';

export class HDBalanceManager {
    constructor(hdWallet, network) {
        this.hdWallet = hdWallet;
        this.network = network;
        this.lastUpdateTime = null;
    }

    /**
     * Check balance for all addresses in the HD wallet
     * @returns {Object} Balance summary
     */
    async checkAllBalances() {
        try {
            const allAddresses = this.hdWallet.getAllAddresses();
            const balancePromises = allAddresses.map(addressInfo => 
                this.checkSingleAddressBalance(addressInfo.address)
            );

            const balanceResults = await Promise.all(balancePromises);
            
            let totalBalance = 0n;
            let totalKAS = 0;
            let hasErrors = false;
            const addressBalances = {};

            for (let i = 0; i < allAddresses.length; i++) {
                const addressInfo = allAddresses[i];
                const balanceResult = balanceResults[i];

                if (balanceResult.success) {
                    const balanceInSompi = BigInt(Math.round(balanceResult.balance.kas * 100000000));
                    totalBalance += balanceInSompi;
                    totalKAS += balanceResult.balance.kas;

                    // Update HD wallet with balance and UTXOs
                    this.hdWallet.updateAddressBalance(
                        addressInfo.address, 
                        balanceInSompi, 
                        balanceResult.utxos || []
                    );

                    // Mark address as used if it has balance
                    if (balanceInSompi > 0n) {
                        this.hdWallet.markAddressAsUsed(addressInfo.address);
                    }

                    addressBalances[addressInfo.address] = {
                        kas: balanceResult.balance.kas,
                        sompi: balanceInSompi,
                        utxos: balanceResult.utxos || []
                    };
                } else {
                    hasErrors = true;
                    console.warn(`Failed to check balance for ${addressInfo.address}:`, balanceResult.error);
                    
                    addressBalances[addressInfo.address] = {
                        kas: 0,
                        sompi: 0n,
                        utxos: [],
                        error: balanceResult.error
                    };
                }
            }

            this.lastUpdateTime = new Date();

            return {
                success: !hasErrors || totalBalance > 0n,
                totalBalance: {
                    kas: totalKAS,
                    sompi: totalBalance
                },
                addressBalances: addressBalances,
                addressCount: allAddresses.length,
                lastUpdate: this.lastUpdateTime,
                hasErrors: hasErrors
            };

        } catch (error) {
            console.error('HD balance check failed:', error);
            return {
                success: false,
                error: error.message,
                totalBalance: { kas: 0, sompi: 0n },
                addressBalances: {},
                addressCount: 0,
                lastUpdate: null,
                hasErrors: true
            };
        }
    }

    /**
     * Check balance for a single address
     * @param {string} address - Address to check
     * @returns {Object} Balance result
     */
    async checkSingleAddressBalance(address) {
        try {
            return await checkAddressBalance(address, this.network);
        } catch (error) {
            return {
                success: false,
                error: error.message,
                balance: { kas: 0, sompi: 0n },
                utxos: []
            };
        }
    }

    /**
     * Get all UTXOs across all addresses
     * @returns {Array} Array of all UTXOs
     */
    getAllUTXOs() {
        return this.hdWallet.getAllUtxos();
    }

    /**
     * Get UTXOs for specific addresses
     * @param {Array} addresses - Addresses to get UTXOs for
     * @returns {Array} Array of UTXOs for specified addresses
     */
    getUTXOsForAddresses(addresses) {
        const allUTXOs = this.getAllUTXOs();
        return allUTXOs.filter(utxo => addresses.includes(utxo.address));
    }

    /**
     * Check if wallet needs address generation based on balance activity
     * @returns {Object} Recommendation for address generation
     */
    checkAddressGenerationNeeds() {
        const receiveAddresses = this.hdWallet.getAddressesByType('receive');
        const changeAddresses = this.hdWallet.getAddressesByType('change');

        const recommendations = {
            generateReceiveAddress: false,
            generateChangeAddress: false,
            reasons: []
        };

        // Check if current receive address has been used
        if (receiveAddresses.length > 0) {
            const currentReceiveAddress = receiveAddresses[receiveAddresses.length - 1];
            if (currentReceiveAddress.used || currentReceiveAddress.balance > 0n) {
                recommendations.generateReceiveAddress = true;
                recommendations.reasons.push('Current receive address has been used');
            }
        } else {
            recommendations.generateReceiveAddress = true;
            recommendations.reasons.push('No receive addresses generated');
        }

        // Check if current change address has been used
        if (changeAddresses.length > 0) {
            const currentChangeAddress = changeAddresses[changeAddresses.length - 1];
            if (currentChangeAddress.used || currentChangeAddress.balance > 0n) {
                recommendations.generateChangeAddress = true;
                recommendations.reasons.push('Current change address has been used');
            }
        }

        return recommendations;
    }

    /**
     * Get balance summary for display
     * @returns {Object} Balance summary
     */
    getBalanceSummary() {
        const totalBalance = this.hdWallet.getTotalBalance();
        const allAddresses = this.hdWallet.getAllAddresses();
        
        const addressesWithBalance = allAddresses.filter(addr => addr.balance > 0n);
        const usedAddresses = allAddresses.filter(addr => addr.used);

        return {
            totalBalance: {
                kas: Number(totalBalance) / 100000000,
                sompi: totalBalance
            },
            addressStats: {
                total: allAddresses.length,
                withBalance: addressesWithBalance.length,
                used: usedAddresses.length,
                receive: allAddresses.filter(addr => addr.type === 'receive').length,
                change: allAddresses.filter(addr => addr.type === 'change').length
            },
            lastUpdate: this.lastUpdateTime
        };
    }

    /**
     * Export balance state for persistence
     * @returns {Object} Serializable balance state
     */
    exportBalanceState() {
        return {
            network: this.network,
            lastUpdateTime: this.lastUpdateTime?.toISOString(),
            balanceSummary: this.getBalanceSummary()
        };
    }
}

export default HDBalanceManager; 