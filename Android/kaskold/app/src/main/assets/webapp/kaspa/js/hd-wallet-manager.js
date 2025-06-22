/**
 * HD Wallet Manager for Kaspa
 * Handles hierarchical deterministic wallet operations including:
 * - Address generation and management
 * - Balance tracking across multiple addresses
 * - UTXO management
 * - Transaction history
 */

import { getKaspa } from './init.js';

export class HDWalletManager {
    constructor(mnemonic, network, derivationPath = "m/44'/111111'/0'") {
        this.mnemonic = mnemonic;
        this.network = network;
        this.derivationPath = derivationPath;
        this.addresses = {
            receive: new Map(), // index -> {address, used, balance, utxos}
            change: new Map()
        };
        this.currentReceiveIndex = 0;
        this.currentChangeIndex = 0;
        this.totalBalance = 0n;
        this.kaspa = null;
        this.xPrv = null;
    }

    /**
     * Initialize the HD wallet manager
     */
    async initialize() {
        this.kaspa = getKaspa();
        const { Mnemonic, XPrv } = this.kaspa;
        
        const mnemonicObj = new Mnemonic(this.mnemonic);
        const seed = mnemonicObj.toSeed();
        this.xPrv = new XPrv(seed);
        
        // Generate initial receive address
        await this.generateNextReceiveAddress();
    }

    /**
     * Generate a new receive address
     * @returns {Object} Address info {address, index, derivationPath}
     */
    async generateNextReceiveAddress() {
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
     * Generate a new change address
     * @returns {Object} Address info {address, index, derivationPath}
     */
    async generateNextChangeAddress() {
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
     * Generate an address at a specific index and type
     * @param {string} type - 'receive' or 'change'
     * @param {number} index - Address index
     * @returns {Object} Address info
     */
    async generateAddress(type, index) {
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
     * Get the current active receive address
     * @returns {string} Current receive address
     */
    getCurrentReceiveAddress() {
        if (this.addresses.receive.size === 0) {
            throw new Error('No receive addresses generated');
        }
        
        // Return the latest generated address
        const latestIndex = this.currentReceiveIndex - 1;
        return this.addresses.receive.get(latestIndex)?.address;
    }

    /**
     * Get the current active change address
     * @returns {string} Current change address
     */
    getCurrentChangeAddress() {
        if (this.addresses.change.size === 0) {
            // Generate first change address if none exists
            this.generateNextChangeAddress();
        }
        
        const latestIndex = this.currentChangeIndex - 1;
        return this.addresses.change.get(latestIndex)?.address;
    }

    /**
     * Mark an address as used (when it receives funds or is used in transaction)
     * @param {string} address - Address to mark as used
     */
    markAddressAsUsed(address) {
        // Find and mark the address as used
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
     * Get all addresses (receive and change)
     * @returns {Array} Array of all addresses
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
     * Get addresses by type
     * @param {string} type - 'receive' or 'change'
     * @returns {Array} Array of addresses of specified type
     */
    getAddressesByType(type) {
        const addressMap = type === 'receive' ? this.addresses.receive : this.addresses.change;
        return Array.from(addressMap.values());
    }

    /**
     * Update balance for a specific address
     * @param {string} address - Address to update
     * @param {bigint} balance - New balance in sompi
     * @param {Array} utxos - UTXOs for this address
     */
    updateAddressBalance(address, balance, utxos = []) {
        let found = false;
        
        // Update receive addresses
        for (const addressInfo of this.addresses.receive.values()) {
            if (addressInfo.address === address) {
                addressInfo.balance = balance;
                addressInfo.utxos = utxos;
                found = true;
                break;
            }
        }
        
        // Update change addresses
        if (!found) {
            for (const addressInfo of this.addresses.change.values()) {
                if (addressInfo.address === address) {
                    addressInfo.balance = balance;
                    addressInfo.utxos = utxos;
                    found = true;
                    break;
                }
            }
        }
        
        // Recalculate total balance
        this.recalculateTotalBalance();
    }

    /**
     * Recalculate total balance across all addresses
     */
    recalculateTotalBalance() {
        let total = 0n;
        
        for (const addressInfo of this.addresses.receive.values()) {
            total += addressInfo.balance;
        }
        
        for (const addressInfo of this.addresses.change.values()) {
            total += addressInfo.balance;
        }
        
        this.totalBalance = total;
    }

    /**
     * Get total wallet balance
     * @returns {bigint} Total balance in sompi
     */
    getTotalBalance() {
        return this.totalBalance;
    }

    /**
     * Get all UTXOs across all addresses
     * @returns {Array} Array of all UTXOs
     */
    getAllUtxos() {
        const utxos = [];
        
        for (const addressInfo of this.addresses.receive.values()) {
            utxos.push(...addressInfo.utxos);
        }
        
        for (const addressInfo of this.addresses.change.values()) {
            utxos.push(...addressInfo.utxos);
        }
        
        return utxos;
    }

    /**
     * Get private key for a specific address
     * @param {string} address - Address to get private key for
     * @returns {string} Private key hex string
     */
    getPrivateKeyForAddress(address) {
        // Find address in receive addresses
        for (const addressInfo of this.addresses.receive.values()) {
            if (addressInfo.address === address) {
                return addressInfo.privateKey;
            }
        }
        
        // Find address in change addresses
        for (const addressInfo of this.addresses.change.values()) {
            if (addressInfo.address === address) {
                return addressInfo.privateKey;
            }
        }
        
        throw new Error(`Private key not found for address: ${address}`);
    }

    /**
     * Get address info by address string
     * @param {string} address - Address to find
     * @returns {Object|null} Address info or null if not found
     */
    getAddressInfo(address) {
        // Check receive addresses
        for (const addressInfo of this.addresses.receive.values()) {
            if (addressInfo.address === address) {
                return addressInfo;
            }
        }
        
        // Check change addresses
        for (const addressInfo of this.addresses.change.values()) {
            if (addressInfo.address === address) {
                return addressInfo;
            }
        }
        
        return null;
    }

    /**
     * Check if we should generate a new receive address
     * (when current address has been used)
     * @returns {boolean} Whether to generate new address
     */
    shouldGenerateNewReceiveAddress() {
        if (this.addresses.receive.size === 0) return true;
        
        const latestIndex = this.currentReceiveIndex - 1;
        const currentAddress = this.addresses.receive.get(latestIndex);
        
        return currentAddress?.used || currentAddress?.balance > 0n;
    }

    /**
     * Check if we should generate a new change address
     * @returns {boolean} Whether to generate new address
     */
    shouldGenerateNewChangeAddress() {
        if (this.addresses.change.size === 0) return true;
        
        const latestIndex = this.currentChangeIndex - 1;
        const currentAddress = this.addresses.change.get(latestIndex);
        
        return currentAddress?.used || currentAddress?.balance > 0n;
    }

    /**
     * Export wallet state for persistence
     * @returns {Object} Serializable wallet state
     */
    exportState() {
        return {
            mnemonic: this.mnemonic,
            network: this.network,
            derivationPath: this.derivationPath,
            addresses: {
                receive: Array.from(this.addresses.receive.entries()),
                change: Array.from(this.addresses.change.entries())
            },
            currentReceiveIndex: this.currentReceiveIndex,
            currentChangeIndex: this.currentChangeIndex,
            totalBalance: this.totalBalance.toString()
        };
    }

    /**
     * Import wallet state from persistence
     * @param {Object} state - Wallet state to import
     */
    async importState(state) {
        this.mnemonic = state.mnemonic;
        this.network = state.network;
        this.derivationPath = state.derivationPath;
        
        // Restore addresses
        this.addresses.receive = new Map(state.addresses.receive);
        this.addresses.change = new Map(state.addresses.change);
        
        this.currentReceiveIndex = state.currentReceiveIndex;
        this.currentChangeIndex = state.currentChangeIndex;
        this.totalBalance = BigInt(state.totalBalance);
        
        // Re-initialize kaspa objects
        await this.initialize();
    }
}

export default HDWalletManager; 