// Kaspa Transaction Submission Module
import { getKaspa, isInitialized } from './init.js';
import { prepareForWasmDeserialization, cleanTransactionDataForSubmission } from './serialization-utils.js';

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
            
            if (signedTransactionData.isUploaded) {
                
                // For uploaded signed transactions, we need to recreate the WASM object from serialized data
                const kaspa = getKaspa();
                const { Transaction } = kaspa;
                
                let wasmTransaction;
                
                // Try to find serialized transaction data
                if (signedTransactionData.serializedTransaction) {
                    
                    try {
                        // Handle different serialization formats
                        if (signedTransactionData.serializedTransaction.type === 'bytes' && signedTransactionData.serializedTransaction.serializedBytes) {
                            // Handle byte array serialization
                            const bytes = new Uint8Array(signedTransactionData.serializedTransaction.serializedBytes);
                            wasmTransaction = Transaction.deserialize(bytes);
                        } else if (signedTransactionData.serializedTransaction.fallbackType === 'minimal_transaction') {
                            // Handle fallback structure - this means the original QR was created with a fallback
                            throw new Error('This signed transaction QR was created with incomplete serialization data and cannot be submitted from upload. Please submit the transaction directly from the signing session.');
                        } else if (typeof signedTransactionData.serializedTransaction === 'object') {
                            // Handle object serialization
                            
                            // Check if the serialized transaction has the required data
                            const serializedTx = signedTransactionData.serializedTransaction;
                            
                            // Validate that we have essential transaction data
                            if (!serializedTx.inputs || !Array.isArray(serializedTx.inputs) || serializedTx.inputs.length === 0) {
                                throw new Error('Serialized transaction has no inputs - cannot submit empty transaction');
                            }
                            
                            if (!serializedTx.outputs || !Array.isArray(serializedTx.outputs) || serializedTx.outputs.length === 0) {
                                throw new Error('Serialized transaction has no outputs - cannot submit empty transaction');
                            }
                            
                            const preparedTx = prepareForWasmDeserialization(serializedTx);                            
                            
                            wasmTransaction = Transaction.deserializeFromObject(preparedTx);
                        } else {
                            throw new Error('Unknown serialization format');
                        }
                        
                    } catch (deserializeError) {
                        console.error('⚠️ Failed to deserialize transaction:', deserializeError);                        
                        throw new Error(`Failed to reconstruct transaction from QR data: ${deserializeError.message}. The signed transaction data may be incomplete or in an unsupported format.`);
                    }
                } else {
                    console.error('🔍 No serializedTransaction found. Available keys:', Object.keys(signedTransactionData));
                    // If no serialized transaction, we can't submit an uploaded QR
                    throw new Error('Uploaded signed transaction QR does not contain submittable transaction data. The QR code may be incomplete or corrupted.');
                }
                
                // Submit using RPC client directly
                const response = await rpc.submitTransaction({
                    transaction: wasmTransaction,
                    allowOrphan: false
                });
                
                transactionId = response.transactionId;
                
            } else {
                // For non-uploaded transactions, use the original WASM object
                if (!signedTransactionData.signedTransaction || typeof signedTransactionData.signedTransaction.submit !== 'function') {
                    // Check if we have a fallback structure but the original WASM object is available
                    if (signedTransactionData.serializedTransaction?.fallbackType === 'minimal_transaction' && 
                        signedTransactionData.serializedTransaction?.wasmObjectAvailable && 
                        signedTransactionData.signedTransaction) {                        
                        // Try to submit using the original WASM object
                        transactionId = await signedTransactionData.signedTransaction.submit(rpc);
                    } else {
                    throw new Error('Signed transaction object does not have a submit method');
                }
                } else {
                // The signedTransaction is a WASM object with a submit() method
                transactionId = await signedTransactionData.signedTransaction.submit(rpc);
                }
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
                console.error('Orphan transaction detected - UTXO may have been spent or network issues');
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
    validateTransactionForSubmission
}; 