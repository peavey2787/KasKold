// Kaspa UTXO Fetching Module
import { getKaspa, isInitialized } from './init.js';

// Fetch all UTXOs for a given address
async function fetchUTXOsForAddress(address, networkType) {
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
            
            // Get UTXOs for the address with timeout
            const utxoPromise = rpc.getUtxosByAddresses([address]);
            const utxoTimeoutPromise = new Promise((_, reject) => {
                setTimeout(() => reject(new Error('UTXO fetch timeout after 15 seconds')), 15000);
            });
            
            const result = await Promise.race([utxoPromise, utxoTimeoutPromise]);
            
            const { entries } = result;
            
            if (entries && entries.length > 0) {
            }

            // Return entries (can be empty array if no UTXOs)
            return {
                success: true,
                utxos: entries || [],
                count: entries ? entries.length : 0,
                address: address,
                networkType: networkType
            };

        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

    } catch (error) {
        return {
            success: false,
            error: error.message || 'Failed to fetch UTXOs',
            utxos: [],
            count: 0,
            address: address,
            networkType: networkType
        };
    }
}

// Fetch UTXOs for multiple addresses
async function fetchUTXOsForAddresses(addresses, networkType) {
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
            // Validate all addresses
            for (const addr of addresses) {
                try {
                    new Address(addr);
                } catch (error) {
                    throw new Error(`Invalid address format for ${addr}: ${error.message}`);
                }
            }
            
            // Get UTXOs for all addresses with timeout
            const utxoPromise = rpc.getUtxosByAddresses(addresses);
            const utxoTimeoutPromise = new Promise((_, reject) => {
                setTimeout(() => reject(new Error('UTXO fetch timeout after 15 seconds')), 15000);
            });
            
            const { entries } = await Promise.race([utxoPromise, utxoTimeoutPromise]);

            // Return entries (can be empty array if no UTXOs)
            return {
                success: true,
                utxos: entries || [],
                count: entries ? entries.length : 0,
                addresses: addresses,
                networkType: networkType
            };

        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

    } catch (error) {
        return {
            success: false,
            error: error.message || 'Failed to fetch UTXOs',
            utxos: [],
            count: 0,
            addresses: addresses,
            networkType: networkType
        };
    }
}

// Fetch transaction details and its UTXOs
async function fetchTransactionUTXOs(transactionId, networkType) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { RpcClient, Resolver } = kaspa;

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
            // Get transaction details with timeout
            const txPromise = rpc.getTransaction(transactionId);
            const txTimeoutPromise = new Promise((_, reject) => {
                setTimeout(() => reject(new Error('Transaction fetch timeout after 15 seconds')), 15000);
            });
            
            const transaction = await Promise.race([txPromise, txTimeoutPromise]);

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

        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

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
    }
}

// Calculate total balance from UTXOs
function calculateBalanceFromUTXOs(utxos) {
    if (!utxos || utxos.length === 0) {
        return {
            totalBalance: 0,
            totalBalanceSompi: 0n,
            utxoCount: 0
        };
    }

    const totalBalanceSompi = utxos.reduce((sum, utxo) => {
        const amount = BigInt(utxo.amount || 0);
        return sum + amount;
    }, 0n);

    const totalBalance = Number(totalBalanceSompi) / 100000000; // Convert to KAS

    return {
        totalBalance: totalBalance,
        totalBalanceSompi: totalBalanceSompi,
        utxoCount: utxos.length
    };
}

// Validate if UTXOs are sufficient for amount + fee
function validateUTXOsSufficient(utxos, requiredAmountSompi, requiredFeeSompi = 0n) {
    const { totalBalanceSompi } = calculateBalanceFromUTXOs(utxos);
    const totalRequired = BigInt(requiredAmountSompi) + BigInt(requiredFeeSompi);
    
    return {
        sufficient: totalBalanceSompi >= totalRequired,
        availableBalance: totalBalanceSompi,
        requiredAmount: totalRequired,
        shortfall: totalBalanceSompi < totalRequired ? totalRequired - totalBalanceSompi : 0n
    };
}

export {
    fetchUTXOsForAddress,
    fetchUTXOsForAddresses,
    fetchTransactionUTXOs,
    calculateBalanceFromUTXOs,
    validateUTXOsSufficient
}; 
