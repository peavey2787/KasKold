// Kaspa Transaction Submission Module
import { getKaspa, isInitialized } from './init.js';

// Check if a UTXO is still available (not spent)
async function checkUTXOAvailability(rpc, transactionId, index) {
    try {
        const { entries } = await rpc.getUtxosByAddresses([]);
        // This is a simplified check - in a real implementation, 
        // we'd need to check the specific UTXO by transaction ID and index
        return true;
    } catch (error) {
        console.warn('Could not check UTXO availability:', error.message);
        return true; // Assume available if we can't check
    }
}

// Submit signed transaction to real Kaspa network
async function submitTransaction(signedTransactionData, networkEndpoint = null) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { RpcClient, Resolver } = kaspa;
        
        // Determine network type from signed transaction data
        const networkType = signedTransactionData.networkType || 'mainnet';

        // Create RPC client using the SDK's resolver for public nodes
        const rpc = new RpcClient({
            networkId: networkType,
            resolver: new Resolver()
        });

        await rpc.connect();
        
        try {
            // Check network sync status first
            const serverInfo = await rpc.getServerInfo();
            
            if (!serverInfo.isSynced) {
                throw new Error('Network node is not synced. Please wait for synchronization to complete.');
            }
            
            // Handle uploaded vs freshly signed transactions differently
            let transactionId;
            
            if (signedTransactionData.isUploaded && signedTransactionData.serializedTransaction) {
                // Use the SDK's Transaction.deserializeFromObject method - much cleaner!
                const kaspa = getKaspa();
                const { Transaction } = kaspa;
                
                const rawTx = signedTransactionData.serializedTransaction;
                
                // Convert string values back to numbers for deserialization
                // The deserializeFromObject method expects numeric values, not strings
                const numericTx = {
                    ...rawTx,
                    version: typeof rawTx.version === 'string' ? parseInt(rawTx.version) : rawTx.version,
                    lockTime: typeof rawTx.lockTime === 'string' ? parseInt(rawTx.lockTime) : rawTx.lockTime,
                    gas: typeof rawTx.gas === 'string' ? parseInt(rawTx.gas) : rawTx.gas,
                    mass: typeof rawTx.mass === 'string' ? parseInt(rawTx.mass) : rawTx.mass,
                    inputs: rawTx.inputs.map(input => ({
                        ...input,
                        sequence: typeof input.sequence === 'string' ? parseInt(input.sequence) : input.sequence,
                        sigOpCount: typeof input.sigOpCount === 'string' ? parseInt(input.sigOpCount) : input.sigOpCount,
                        index: typeof input.index === 'string' ? parseInt(input.index) : input.index,
                        utxo: input.utxo ? {
                            ...input.utxo,
                            amount: typeof input.utxo.amount === 'string' ? parseInt(input.utxo.amount) : input.utxo.amount,
                            blockDaaScore: typeof input.utxo.blockDaaScore === 'string' ? parseInt(input.utxo.blockDaaScore) : input.utxo.blockDaaScore
                        } : input.utxo
                    })),
                    outputs: rawTx.outputs.map(output => ({
                        ...output,
                        value: typeof output.value === 'string' ? parseInt(output.value) : output.value,
                        amount: typeof output.amount === 'string' ? parseInt(output.amount) : output.amount
                    }))
                };
                
                // Use the SDK's deserializeFromObject method to recreate the WASM Transaction
                const wasmTransaction = Transaction.deserializeFromObject(numericTx);
                
                // Use RPC submitTransaction method with the WASM object
                const response = await rpc.submitTransaction({
                    transaction: wasmTransaction,
                    allowOrphan: false
                });
                
                transactionId = response.transactionId;
                
            } else {
                // Use the SDK's built-in submit method on the signed transaction
                if (typeof signedTransactionData.signedTransaction.submit !== 'function') {
                    throw new Error('Signed transaction object does not have a submit method');
                }
                
                // The signedTransaction is a WASM object with a submit() method
                transactionId = await signedTransactionData.signedTransaction.submit(rpc);
            }
            
            return {
                success: true,
                transactionId: transactionId,
                networkResponse: {
                    accepted: true,
                    message: 'Transaction accepted by network',
                    transactionId: transactionId
                },
                status: 'submitted',
                timestamp: new Date().toISOString()
            };
            
        } catch (submitError) {
            // If it's an orphan error, suggest possible solutions
            if (submitError.message && submitError.message.includes('orphan')) {
                console.log('Orphan transaction detected - UTXO may have been spent or network issues');
            }
            
            throw submitError; // Re-throw to be caught by outer catch
        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

    } catch (error) {
        return {
            success: false,
            error: error.message || error.toString() || 'Unknown submission error',
            status: 'failed'
        };
    }
}

// Mock network submission (replace with real RPC calls)
async function mockNetworkSubmission(signedTransactionData, networkEndpoint) {
    // Simulate network delay
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Mock different network responses
    const responses = [
        {
            accepted: true,
            transactionId: signedTransactionData.transactionId,
            message: 'Transaction accepted by network',
            confirmations: 0,
            estimatedConfirmationTime: '~1 minute'
        },
        {
            accepted: false,
            error: 'Insufficient funds',
            message: 'Transaction rejected: insufficient funds'
        },
        {
            accepted: false,
            error: 'Double spend',
            message: 'Transaction rejected: inputs already spent'
        }
    ];
    
    // Return success response for demo (90% success rate)
    return Math.random() > 0.1 ? responses[0] : responses[1];
}

// Real network submission (template for future implementation)
async function submitToRealNetwork(signedTransactionData, rpcEndpoint) {
    // This would be the real implementation:
    /*
    const rpcCall = {
        method: 'submitTransaction',
        params: {
            transaction: serializeTransaction(signedTransactionData.signedTransaction),
            allowOrphan: false
        },
        id: 1
    };
    
    const response = await fetch(rpcEndpoint, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(rpcCall)
    });
    
    return await response.json();
    */
    
    throw new Error('Real network submission not implemented yet');
}

// Get transaction status from real Kaspa network
async function getTransactionStatus(transactionId, networkType = 'mainnet', networkEndpoint = null) {
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

        await rpc.connect();
        
        try {
            // Get transaction information
            const txResponse = await rpc.getTransaction({
                transactionId: transactionId,
                includeBlockVerboseData: true
            });
            
            if (txResponse && txResponse.transaction) {
                const tx = txResponse.transaction;
                const isConfirmed = tx.blockHash && tx.blockHash !== '';
                
                return {
                    transactionId: transactionId,
                    status: isConfirmed ? 'confirmed' : 'pending',
                    confirmations: isConfirmed ? tx.confirmations || 1 : 0,
                    blockHash: tx.blockHash || null,
                    blockTime: tx.blockTime || null,
                    timestamp: new Date().toISOString()
                };
            } else {
                return {
                    transactionId: transactionId,
                    status: 'not_found',
                    confirmations: 0,
                    blockHash: null,
                    timestamp: new Date().toISOString()
                };
            }
            
        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }
        
    } catch (error) {
        console.error('Transaction status check error:', error);
        return {
            transactionId: transactionId,
            status: 'unknown',
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

// Validate transaction before submission
function validateTransactionForSubmission(signedTransactionData) {
    const errors = [];
    
    // Handle uploaded vs freshly signed transactions
    if (signedTransactionData.isUploaded) {
        if (!signedTransactionData.serializedTransaction) {
            errors.push('No serialized transaction data found for uploaded transaction');
        }
    } else {
        if (!signedTransactionData.signedTransaction) {
            errors.push('No signed transaction found');
        }
        
        // Check if all inputs are signed (only for non-uploaded transactions)
        if (signedTransactionData.signedTransaction) {
            const inputs = signedTransactionData.signedTransaction.inputs;
            for (let i = 0; i < inputs.length; i++) {
                if (!inputs[i].signatureScript || inputs[i].signatureScript.length === 0) {
                    errors.push(`Input ${i} is not signed`);
                }
            }
        }
    }
    
    if (!signedTransactionData.transactionId) {
        errors.push('No transaction ID found');
    }
    
    return {
        isValid: errors.length === 0,
        errors: errors
    };
}

export {
    submitTransaction,
    getTransactionStatus,
    validateTransactionForSubmission,
    mockNetworkSubmission
}; 