// Kaspa Transaction Creation Module
import { getKaspa, isInitialized } from './init.js';
import { calculateTransactionFee } from './fee-calculator.js';
import { fetchUTXOsForAddress, fetchUTXOsForAddresses } from './utxo-fetcher.js';

// Legacy wrapper for backward compatibility
async function fetchUTXOs(address, networkType) {
    const result = await fetchUTXOsForAddress(address, networkType);
    
    if (!result.success) {
        throw new Error(result.error);
    }
    
    return result.utxos;
}

// Create transaction using Kaspa WASM SDK createTransactions with manual fee
async function createTransactionWithManualFee(fromAddress, toAddress, amount, manualFeeKas, networkType) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { createTransactions } = kaspa;

        // Convert KAS to sompi manually
        const amountFloat = parseFloat(amount);
        const manualFeeFloat = parseFloat(manualFeeKas);
        
        if (isNaN(amountFloat) || amountFloat <= 0) {
            throw new Error('Invalid amount: must be a positive number');
        }
        
        if (isNaN(manualFeeFloat) || manualFeeFloat < 0) {
            throw new Error('Invalid fee: must be a non-negative number');
        }
        
        // Manual conversion: 1 KAS = 100,000,000 sompi
        const amountInSompi = BigInt(Math.floor(amountFloat * 100000000));
        const manualFeeInSompi = BigInt(Math.floor(manualFeeFloat * 100000000));
        
        // Fetch real UTXOs for the address
        const entries = await fetchUTXOs(fromAddress, networkType);
        
        if (!entries || entries.length === 0) {
            throw new Error('No UTXOs found for this address. The address either has no balance, does not exist on the network, or all UTXOs are already spent. Please check the address and ensure it has sufficient funds.');
        }
        
        // Calculate total available balance
        const totalBalance = entries.reduce((sum, entry) => sum + BigInt(entry.amount), 0n);
        const totalRequired = amountInSompi + manualFeeInSompi;
        
        if (totalBalance < totalRequired) {
            const balanceKas = Number(totalBalance) / 100000000;
            const requiredKas = Number(totalRequired) / 100000000;
            throw new Error(`Insufficient funds. Available: ${balanceKas.toFixed(8)} KAS, Required: ${requiredKas.toFixed(8)} KAS (amount + fee)`);
        }
        
        // Use createTransactions with manual fee as priorityFee
        const { transactions, summary } = await createTransactions({
            entries,
            outputs: [{
                address: toAddress,
                amount: amountInSompi
            }],
            priorityFee: manualFeeInSompi, // Use manual fee as priority fee
            changeAddress: fromAddress,
            networkId: networkType
        });
        
        if (!transactions || transactions.length === 0) {
            throw new Error('Failed to create transactions - insufficient funds or invalid parameters');
        }
        
        // Get the first pending transaction
        const pendingTransaction = transactions[0];
        
        // Generate a simple transaction ID
        const transactionId = 'pending_manual_' + Date.now() + '_' + Math.random().toString(36).substring(7);
        
        return {
            success: true,
            pendingTransaction: pendingTransaction,
            transactions: transactions,
            transactionId: transactionId,
            fromAddress: fromAddress,
            toAddress: toAddress,
            amount: amount,
            amountInSompi: amountInSompi.toString(),
            fee: {
                priorityFee: manualFeeInSompi.toString(),
                feeInSompi: manualFeeInSompi.toString(), // Use manual fee
                feeInKas: manualFeeFloat,
                manual: true // Flag to indicate manual fee
            },
            summary: summary,
            networkType: networkType,
            status: 'created',
            feeMode: 'manual'
        };

    } catch (error) {
        return {
            success: false,
            error: error.message || 'Unknown error occurred'
        };
    }
}

// Create transaction using Kaspa WASM SDK createTransactions
async function createTransaction(fromAddress, toAddress, amount, networkType, options = {}) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { createTransactions, kaspaToSompi } = kaspa;

        // Convert KAS to sompi manually (kaspaToSompi was causing index out of bounds)
        const amountFloat = parseFloat(amount);
        
        if (isNaN(amountFloat) || amountFloat <= 0) {
            throw new Error('Invalid amount: must be a positive number');
        }
        
        // Manual conversion: 1 KAS = 100,000,000 sompi
        const amountInSompi = BigInt(Math.floor(amountFloat * 100000000));
        
        // Check if we have multiple addresses (HD wallet) or single address
        let entries;
        if (options.hdWallet) {
            // For HD wallets, fetch UTXOs from all addresses with balance
            const allAddresses = options.hdWallet.getAllAddresses()
                .filter(addr => addr.balance > 0n)
                .map(addr => addr.address);
            
            if (allAddresses.length === 0) {
                throw new Error('No addresses with balance found in HD wallet');
            }
            
            const utxoResult = await fetchUTXOsForAddresses(allAddresses, networkType);
            if (!utxoResult.success) {
                throw new Error(utxoResult.error);
            }
            entries = utxoResult.utxos;
        } else {
            // For single address wallets
            entries = await fetchUTXOs(fromAddress, networkType);
        }
        
        if (!entries || entries.length === 0) {
            throw new Error('No UTXOs found for this address. The address either has no balance, does not exist on the network, or all UTXOs are already spent. Please check the address and ensure it has sufficient funds.');
        }
        

        
        // Extract options
        const { feeOption = 'normal', changeAddress = fromAddress } = options;
        
        // Calculate priority fee based on option
        const priorityFeeRates = {
            'slow': 0n,
            'normal': 1000n, 
            'fast': 5000n
        };
        const priorityFee = priorityFeeRates[feeOption] || priorityFeeRates['normal'];
        
        // Use createTransactions with object parameter structure
        const { transactions, summary } = await createTransactions({
            entries,
            outputs: [{
                address: toAddress,
                amount: amountInSompi
            }],
            priorityFee: priorityFee,
            changeAddress: changeAddress, // Use provided change address or fallback to fromAddress
            networkId: networkType
        });
        
        if (!transactions || transactions.length === 0) {
            throw new Error('Failed to create transactions - insufficient funds or invalid parameters');
        }
        
        // Get the first pending transaction (most cases will have only one)
        const pendingTransaction = transactions[0];
        
        // Generate a simple transaction ID since pending transactions might not have id() method
        const transactionId = 'pending_' + Date.now() + '_' + Math.random().toString(36).substring(7);
        
        return {
            success: true,
            pendingTransaction: pendingTransaction,
            transactions: transactions,
            transactionId: transactionId,
            fromAddress: fromAddress,
            toAddress: toAddress,
            amount: amount,
            amountInSompi: amountInSompi.toString(),
            changeAddress: changeAddress,
            fee: {
                priorityFee: priorityFee.toString(),
                feeInSompi: summary.fees.toString(),
                feeInKas: Number(summary.fees) / 100000000,
                manual: false // Flag to indicate automatic fee
            },
            summary: summary,
            networkType: networkType,
            status: 'created',
            feeMode: 'automatic'
        };

    } catch (error) {
        return {
            success: false,
            error: error.message || 'Unknown error occurred'
        };
    }
}

// Export transaction as JSON for offline signing
function exportTransaction(transactionData) {
    const exportData = {
        type: 'kaspa-unsigned-transaction',
        transactionId: transactionData.transactionId,
        fromAddress: transactionData.fromAddress,
        toAddress: transactionData.toAddress,
        amount: transactionData.amount,
        amountInSompi: transactionData.amountInSompi,
        fee: transactionData.fee,
        feeMode: transactionData.feeMode || 'automatic',
        networkType: transactionData.networkType,
        timestamp: new Date().toISOString(),
        status: 'unsigned',
        // Include the pending transaction data needed for signing
        pendingTransaction: transactionData.pendingTransaction ? {
            // Serialize the pending transaction to a format that can be reconstructed
            serialized: transactionData.pendingTransaction.serializeToObject ? 
                transactionData.pendingTransaction.serializeToObject() : null
        } : null,
        summary: transactionData.summary
    };
    
    // Convert BigInt values to strings for JSON serialization
    return JSON.stringify(exportData, (key, value) =>
        typeof value === 'bigint' ? value.toString() : value, 2);
}

export {
    createTransaction,
    createTransactionWithManualFee,
    exportTransaction,
    fetchUTXOs
}; 