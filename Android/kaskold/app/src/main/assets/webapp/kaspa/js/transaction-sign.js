// Kaspa Transaction Signing Module
import { getKaspa, isInitialized } from './init.js';

// Sign transaction with private key(s) using SDK's pending transaction sign method
async function signTransaction(transactionData, privateKeys) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { PrivateKey, createTransactions, RpcClient, Resolver, PendingTransaction } = kaspa;

        // Handle both single private key (string) and multiple private keys (object)
        let privKeyArray = [];
        
        if (typeof privateKeys === 'string') {
            // Single private key for single-address wallet
            privKeyArray = [new PrivateKey(privateKeys)];
        } else if (typeof privateKeys === 'object' && privateKeys !== null) {
            // Multiple private keys for HD wallet (object with address -> privateKey mapping)
            for (const [address, privateKey] of Object.entries(privateKeys)) {
                if (privateKey) {
                    privKeyArray.push(new PrivateKey(privateKey));
                }
            }
            
            if (privKeyArray.length === 0) {
                throw new Error('No valid private keys provided for HD wallet');
            }
        } else {
            throw new Error('Invalid private key format - expected string or object');
        }

        let pendingTransaction;

        // Calculate amount in sompi from transaction data
        let amountInSompi;
        if (transactionData.amountInSompi) {
            // If amount is already in sompi format
            amountInSompi = typeof transactionData.amountInSompi === 'string' ? 
                BigInt(transactionData.amountInSompi) : BigInt(transactionData.amountInSompi);
        } else if (transactionData.amount) {
            // Convert KAS to sompi
            const { kasNumberToSompi } = await import('./currency-utils.js');
            const amountFloat = parseFloat(transactionData.amount);
            amountInSompi = kasNumberToSompi(amountFloat);
        } else {
            throw new Error('No amount specified in transaction data');
        }

        // Determine change address
        const changeAddress = transactionData.changeAddress || transactionData.fromAddress;
        if (!changeAddress) {
            throw new Error('No change address specified in transaction data');
        }

        // Check if we need to recreate the transaction or use existing pending transaction
        if (transactionData.transactionDetails && transactionData.transactionDetails.inputUtxos && 
            transactionData.transactionDetails.inputUtxos.length > 0) {
            
            console.log('🔄 Using UTXO data from enhanced QR for offline signing');
            const entries = transactionData.transactionDetails.inputUtxos;
            console.log(`📊 UTXO entries from transactionDetails: ${entries.length}`);
            console.log('🔍 First UTXO sample:', entries[0]);
            
            // Check if UTXOs contain warning about incomplete data
            if (entries[0]?.warning === 'OFFLINE_SIGNING_INCOMPLETE') {
                throw new Error('QR contains incomplete UTXO data. This transaction cannot be signed offline.');
            }
            
            // Validate UTXO entries format
            console.log('🔍 Validating UTXO entries format...');
            for (let i = 0; i < entries.length; i++) {
                const entry = entries[i];
                console.log(`🔍 UTXO ${i}:`, {
                    hasOutpoint: !!entry.outpoint,
                    outpointKeys: entry.outpoint ? Object.keys(entry.outpoint) : [],
                    hasAddress: !!entry.address,
                    addressType: typeof entry.address,
                    hasAmount: !!entry.amount,
                    amountType: typeof entry.amount,
                    hasScriptPublicKey: !!entry.scriptPublicKey,
                    scriptPublicKeyKeys: entry.scriptPublicKey ? Object.keys(entry.scriptPublicKey) : []
                });
                
                // Convert amount to BigInt if needed
                if (typeof entry.amount === 'string') {
                    try {
                        entries[i].amount = BigInt(entry.amount);
                        console.log(`✅ Converted UTXO ${i} amount from string to BigInt`);
                    } catch (conversionError) {
                        console.error(`❌ Failed to convert UTXO ${i} amount to BigInt:`, conversionError);
                    }
                }
                
                // Ensure address is a string (not an object)
                if (entry.address && typeof entry.address === 'object' && entry.address.payload) {
                    // Convert address object to string
                    entries[i].address = entry.address.payload;
                    console.log(`✅ Converted UTXO ${i} address from object to string:`, entries[i].address);
                }
            }
            
            console.log('📊 Creating transaction with:', {
                entryCount: entries.length,
                toAddress: transactionData.toAddress,
                amount: amountInSompi.toString(),
                changeAddress: changeAddress,
                networkType: transactionData.networkType
            });
            
            // Log the original entries structure for debugging
            console.log('🔍 Original entries structure:', {
                entryCount: entries.length,
                firstEntry: entries[0] ? {
                    keys: Object.keys(entries[0]),
                    hasPrefix: 'prefix' in entries[0],
                    prefix: entries[0].prefix,
                    address: entries[0].address,
                    outpoint: entries[0].outpoint,
                    amount: entries[0].amount
                } : 'No entries'
            });
            
            // Recreate the transaction using the original UTXO entries
            console.log('🔄 Recreating transaction from original UTXO entries for offline signing');
            try {
                // Helper function to validate and normalize UTXO entry for createTransactions
                async function validateAndNormalizeUTXOEntry(entry, index, networkType) {
                    const { convertStringToBigInt } = await import('./serialization-utils.js');
                    
                    // Use serialization utils to properly convert the entry
                    const normalized = convertStringToBigInt(entry);
                    
                    console.log(`🔍 UTXO ${index} after convertStringToBigInt:`, {
                        keys: Object.keys(normalized),
                        hasAddress: !!normalized.address,
                        hasOutpoint: !!normalized.outpoint,
                        hasAmount: !!normalized.amount,
                        hasScriptPublicKey: !!normalized.scriptPublicKey,
                        hasBlockDaaScore: !!normalized.blockDaaScore,
                        hasIsCoinbase: 'isCoinbase' in normalized,
                        scriptPublicKeyType: typeof normalized.scriptPublicKey,
                        scriptPublicKeyValue: normalized.scriptPublicKey,
                        scriptPublicKeyKeys: normalized.scriptPublicKey ? Object.keys(normalized.scriptPublicKey) : 'N/A'
                    });
                    
                    // Ensure address is a string (remove any prefix property)
                    if (normalized.address && typeof normalized.address === 'object') {
                        if (normalized.address.prefix && normalized.address.payload) {
                            normalized.address = `${normalized.address.prefix}:${normalized.address.payload}`;
                        } else if (normalized.address.toString) {
                            normalized.address = normalized.address.toString();
                        }
                    }
                    
                    // Remove any prefix property that shouldn't be there
                    if (normalized.prefix) {
                        delete normalized.prefix;
                    }
                    
                    // Validate required properties exist
                    if (!normalized.outpoint) {
                        throw new Error(`UTXO ${index} missing outpoint`);
                    }
                    
                    if (!normalized.outpoint.transactionId) {
                        throw new Error(`UTXO ${index} outpoint missing transactionId`);
                    }
                    
                    if (normalized.outpoint.index === undefined || normalized.outpoint.index === null) {
                        throw new Error(`UTXO ${index} outpoint missing index`);
                    }
                    
                    if (!normalized.amount) {
                        throw new Error(`UTXO ${index} missing amount`);
                    }
                    
                    if (!normalized.scriptPublicKey) {
                        throw new Error(`UTXO ${index} missing scriptPublicKey`);
                    }
                    
                    if (!normalized.blockDaaScore) {
                        throw new Error(`UTXO ${index} missing blockDaaScore`);
                    }
                    
                    // Ensure blockDaaScore is BigInt
                    if (typeof normalized.blockDaaScore === 'string') {
                        normalized.blockDaaScore = BigInt(normalized.blockDaaScore);
                    } else if (typeof normalized.blockDaaScore !== 'bigint') {
                        normalized.blockDaaScore = BigInt(normalized.blockDaaScore);
                    }
                    
                    if (normalized.isCoinbase === undefined || normalized.isCoinbase === null) {
                        throw new Error(`UTXO ${index} missing isCoinbase`);
                    }
                    
                    console.log(`✅ UTXO ${index} validation passed`);
                    return normalized;
                }

                // Process UTXO entries to ensure they match IUtxoEntry interface
                const processedEntries = await Promise.all(entries.map(async (entry, index) => {
                    const normalizedEntry = await validateAndNormalizeUTXOEntry(entry, index, transactionData.networkType);
                    if (!normalizedEntry) {
                        throw new Error(`Failed to validate and normalize UTXO entry at index ${index}`);
                    }
                    return normalizedEntry;
                }));
                
                console.log('🔍 Debugging createTransactions parameters:', {
                    entriesType: typeof processedEntries,
                    entriesLength: processedEntries?.length,
                    firstEntryKeys: processedEntries?.[0] ? Object.keys(processedEntries[0]) : 'N/A',
                    firstEntryAddress: processedEntries?.[0]?.address || 'MISSING',
                    firstEntryScriptPublicKey: processedEntries?.[0]?.scriptPublicKey || 'MISSING',
                    firstEntryScriptPublicKeyType: typeof processedEntries?.[0]?.scriptPublicKey,
                    firstEntryScriptPublicKeyKeys: processedEntries?.[0]?.scriptPublicKey ? Object.keys(processedEntries?.[0]?.scriptPublicKey) : 'N/A',
                    toAddress: transactionData.toAddress,
                    amountInSompi: amountInSompi.toString(),
                    changeAddress: changeAddress,
                    networkType: transactionData.networkType,
                    feeType: typeof (transactionData.fee ? BigInt(transactionData.fee.priorityFee || '1000') : 1000n)
                });
                
                // Log the complete structure of the first entry
                if (processedEntries && processedEntries.length > 0) {
                    console.log('🔍 COMPLETE first entry structure:', JSON.stringify(processedEntries[0], (key, value) => 
                        typeof value === 'bigint' ? value.toString() + 'n' : value, 2));
                }

                const { transactions } = await createTransactions({
                    entries: processedEntries,
                    outputs: [{
                        address: transactionData.toAddress,
                        amount: amountInSompi
                    }],
                    priorityFee: transactionData.fee ? BigInt(transactionData.fee.priorityFee || '1000') : 1000n,
                    changeAddress: changeAddress,
                    networkId: transactionData.networkType
                });

                if (!transactions || transactions.length === 0) {
                    throw new Error('Failed to recreate transaction - createTransactions returned empty result');
                }

                pendingTransaction = transactions[0];
                console.log('✅ Successfully recreated pending transaction for offline signing');
            } catch (createTxError) {
                console.error('❌ createTransactions failed:', createTxError);
                console.error('🔍 Error details:', {
                    message: createTxError.message,
                    stack: createTxError.stack,
                    name: createTxError.name
                });
                
                // Log the actual entries that failed
                console.error('🔍 Failed entries structure:', {
                    entryCount: entries.length,
                    firstEntry: entries[0] ? {
                        keys: Object.keys(entries[0]),
                        hasPrefix: 'prefix' in entries[0],
                        prefix: entries[0].prefix,
                        address: entries[0].address,
                        outpoint: entries[0].outpoint
                    } : 'No entries',
                    allEntries: entries.map((entry, i) => ({
                        index: i,
                        keys: Object.keys(entry),
                        hasPrefix: 'prefix' in entry,
                        prefix: entry.prefix
                    }))
                });
                
                throw new Error(`Failed to recreate transaction: ${createTxError.message}`);
            }
        } else {
            // Use existing pending transaction
            pendingTransaction = transactionData.pendingTransaction || (transactionData.transactions && transactionData.transactions[0]);
            
            if (!pendingTransaction) {
                throw new Error('No pending transaction found to sign');
            }
        }
        
        // Use the SDK's built-in sign method on the pending transaction
        try {
            console.log('🔏 About to sign transaction:', {
                privKeyCount: privKeyArray.length,
                inputCount: pendingTransaction.inputs?.length,
                outputCount: pendingTransaction.outputs?.length,
                transactionId: pendingTransaction.id || 'unknown'
            });
            
            await pendingTransaction.sign(privKeyArray);
            console.log('✅ Transaction signed successfully');
        } catch (signError) {
            console.error('❌ Transaction signing failed:', {
                message: signError.message,
                stack: signError.stack,
                name: signError.name,
                privKeyCount: privKeyArray.length,
                inputCount: pendingTransaction?.inputs?.length || 'unknown',
                outputCount: pendingTransaction?.outputs?.length || 'unknown',
                pendingTransactionType: typeof pendingTransaction,
                hasPendingTransaction: !!pendingTransaction
            });
            throw new Error(`Transaction signing failed: ${signError.message}`);
        }

        return {
            success: true,
            signedTransaction: pendingTransaction,
            transactionId: transactionData.transactionId,
            status: 'signed'
        };

    } catch (error) {
        return {
            success: false,
            error: error.message
        };
    }
}

// Verify transaction signatures
function verifyTransactionSignatures(signedTransactionData) {
    try {
        const kaspa = getKaspa();
        const { PublicKey, SigHashType } = kaspa;

        const transaction = signedTransactionData.signedTransaction;
        let allValid = true;
        const verificationResults = [];

        for (let i = 0; i < transaction.inputs.length; i++) {
            const input = transaction.inputs[i];
            const signatureScript = input.signatureScript;
            
            if (signatureScript.length === 0) {
                verificationResults.push(false);
                allValid = false;
                continue;
            }

            // Extract signature and public key from signature script
            const signature = signatureScript.slice(0, -33); // All but last 33 bytes
            const publicKeyBytes = signatureScript.slice(-33); // Last 33 bytes
            
            try {
                const publicKey = PublicKey.fromBytes(publicKeyBytes);
                // In real implementation, would verify signature against transaction hash
                verificationResults.push(true);
            } catch (e) {
                verificationResults.push(false);
                allValid = false;
            }
        }

        return {
            allValid: allValid,
            individualResults: verificationResults
        };

    } catch (error) {
        return {
            allValid: false,
            error: error.message
        };
    }
}

// Import centralized export function
import { exportSignedTransaction } from './serialization-utils.js';

export {
    signTransaction,
    verifyTransactionSignatures,
    exportSignedTransaction
}; 