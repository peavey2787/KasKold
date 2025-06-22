// Kaspa Wallet Balance Module
import { getKaspa, isInitialized } from './init.js';
import { fetchUTXOsForAddress } from './utxo-fetcher.js';

// Fetch ALL UTXOs for balance checking (using centralized fetcher)
async function fetchAllUTXOs(address, networkType) {
    const result = await fetchUTXOsForAddress(address, networkType);
    
    if (!result.success) {
        throw new Error(result.error);
    }
    
    console.log('Total UTXOs found:', result.count);
    console.log('First few UTXOs:', result.utxos.slice(0, 3));
    
    return result.utxos;
}

// Check balance for an address
async function checkAddressBalance(address, networkType) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        // Fetch ALL UTXOs for the address
        const entries = await fetchAllUTXOs(address, networkType);
        
        // Calculate total balance from all UTXO entries
        const totalBalance = entries.reduce((sum, entry) => sum + BigInt(entry.amount), BigInt(0));
        const balanceInKas = Number(totalBalance) / 100000000;
        
        return {
            success: true,
            address: address,
            balance: {
                kas: balanceInKas,
                sompi: totalBalance.toString()
            },
            utxoCount: entries.length,
            utxos: entries,
            networkType: networkType
        };

    } catch (error) {
        return {
            success: false,
            error: error.message,
            address: address
        };
    }
}

// Get detailed UTXO information
function getUTXODetails(balanceResult) {
    
    if (!balanceResult.success) {
        return [];
    }


    const details = balanceResult.utxos.map((entry, index) => {
        return {
            index: index,
            transactionId: entry.outpoint.transactionId,
            outputIndex: entry.outpoint.index,
            amount: {
                kas: Number(BigInt(entry.amount)) / 100000000,
                sompi: entry.amount.toString()
            },
            blockDaaScore: entry.blockDaaScore.toString(),
            isCoinbase: entry.isCoinbase
        };
    });
    
    return details;
}

// Fetch transaction history for an address
async function fetchTransactionHistory(address, networkType, limit = 50) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { RpcClient, Resolver, Address } = kaspa;

        // Create RPC client using the SDK's resolver for public nodes
        const rpc = new RpcClient({
            networkId: networkType,
            resolver: new Resolver()
        });

        // Connect with timeout
        const connectPromise = rpc.connect();
        const connectTimeoutPromise = new Promise((_, reject) => {
            setTimeout(() => reject(new Error('RPC connection timeout after 15 seconds')), 15000);
        });
        
        await Promise.race([connectPromise, connectTimeoutPromise]);

        try {
            // Validate the address format
            try {
                new Address(address);
            } catch (error) {
                throw new Error(`Invalid address format: ${error.message}`);
            }
            
            // Get transaction history for the address
            const historyPromise = rpc.getTransactionsByAddresses([address], limit);
            const historyTimeoutPromise = new Promise((_, reject) => {
                setTimeout(() => reject(new Error('Transaction history fetch timeout after 15 seconds')), 15000);
            });
            
            const transactions = await Promise.race([historyPromise, historyTimeoutPromise]);
            

            return {
                success: true,
                transactions: transactions || [],
                count: transactions ? transactions.length : 0,
                address: address,
                networkType: networkType
            };

        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

    } catch (error) {
        console.error('Error fetching transaction history:', error);
        return {
            success: false,
            error: error.message || 'Failed to fetch transaction history',
            transactions: [],
            count: 0,
            address: address,
            networkType: networkType
        };
    }
}

// Format transaction history for display
function formatTransactionHistory(historyResult) {
    if (!historyResult.success) {
        return `<p style="color: #dc3545;">Error: ${historyResult.error}</p>`;
    }

    if (!historyResult.transactions || historyResult.transactions.length === 0) {
        return '<p style="text-align: center; color: #666; font-style: italic;">No transactions found</p>';
    }

    return historyResult.transactions.map((tx, index) => {
        const txId = tx.transactionId || tx.id || 'Unknown';
        const timestamp = tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleString() : 'Pending';
        const confirmations = tx.confirmations || 0;
        const isConfirmed = confirmations > 0;
        
        // Calculate net amount (simplified - would need more complex logic for accurate amounts)
        let netAmount = 0;
        let direction = 'Unknown';
        
        if (tx.inputs && tx.outputs) {
            // This is a simplified calculation - real implementation would need to check
            // which inputs/outputs belong to our address
            const totalOutputs = tx.outputs.reduce((sum, output) => sum + (output.amount || 0), 0);
            netAmount = totalOutputs / 100000000; // Convert to KAS
            direction = 'Received'; // Simplified assumption
        }

        return `
            <div style="margin: 0.5em 0; padding: 0.5em; background: #f8f9fa; border: 1px solid #e9ecef; border-radius: 3px;">
                <p><strong>Transaction #${index + 1}</strong></p>
                <p><strong>ID:</strong> ${txId.substring(0, 16)}...</p>
                <p><strong>Direction:</strong> ${direction}</p>
                <p><strong>Amount:</strong> ${netAmount.toFixed(8)} KAS</p>
                <p><strong>Status:</strong> ${isConfirmed ? `Confirmed (${confirmations})` : 'Pending'}</p>
                <p><strong>Time:</strong> ${timestamp}</p>
            </div>
        `;
    }).join('');
}

// Format balance for display
function formatBalance(balanceResult) {
    if (!balanceResult.success) {
        return `Error: ${balanceResult.error}`;
    }

    return `${balanceResult.balance.kas.toFixed(8)} KAS (${balanceResult.utxoCount} UTXO${balanceResult.utxoCount !== 1 ? 's' : ''})`;
}

export {
    fetchAllUTXOs,
    checkAddressBalance,
    getUTXODetails,
    fetchTransactionHistory,
    formatTransactionHistory,
    formatBalance
}; 
