// Kaspa Offline Transaction Creation Module
// This module creates transactions using cached UTXO data to avoid network calls
import { getKaspa, isInitialized } from './init.js';

// Helper function to extract scriptPublicKey data from WASM objects
function extractScriptPublicKeyData(scriptPublicKey) {
    if (!scriptPublicKey) {
        return null;
    }
    
    // If it's already a plain object with version and script, return as-is
    if (typeof scriptPublicKey === 'object' && !scriptPublicKey.__wbg_ptr) {
        if (scriptPublicKey.version !== undefined && scriptPublicKey.script !== undefined) {
            return {
                version: typeof scriptPublicKey.version === 'string' ? parseInt(scriptPublicKey.version, 10) : scriptPublicKey.version,
                script: scriptPublicKey.script
            };
        }
    }
    
    // If it's a WASM object, try to extract data
    if (typeof scriptPublicKey === 'object' && scriptPublicKey.__wbg_ptr) {
        console.log('🔍 Extracting data from WASM scriptPublicKey (offline):', {
            hasVersionProp: 'version' in scriptPublicKey,
            hasScriptProp: 'script' in scriptPublicKey,
            availableProps: Object.getOwnPropertyNames(scriptPublicKey)
        });
        
        // Try to access version and script properties directly
        try {
            let version = 0;
            let script = null;
            
            if (typeof scriptPublicKey.version !== 'undefined') {
                version = scriptPublicKey.version;
            }
            
            if (typeof scriptPublicKey.script !== 'undefined') {
                script = scriptPublicKey.script;
            }
            
            // If we got the script, return the extracted data
            if (script) {
                return {
                    version: typeof version === 'string' ? parseInt(version, 10) : version,
                    script: script
                };
            }
        } catch (e) {
            console.warn('Failed to access WASM scriptPublicKey properties directly (offline):', e);
        }
        
        // Fallback: try to serialize the WASM object using toString or other methods
        try {
            if (typeof scriptPublicKey.toString === 'function') {
                const scriptStr = scriptPublicKey.toString();
                if (scriptStr && scriptStr !== '[object Object]') {
                    return {
                        version: 0,
                        script: scriptStr
                    };
                }
            }
        } catch (e) {
            console.warn('Failed to convert WASM scriptPublicKey to string (offline):', e);
        }
        
        console.warn('❌ Could not extract data from WASM scriptPublicKey (offline), returning fallback');
        return {
            version: 0,
            script: '0020000000000000000000000000000000000000000000000000000000000000000000' // 32-byte zero script as fallback
        };
    }
    
    // If it's a hex string, convert to proper format
    if (typeof scriptPublicKey === 'string') {
        return {
            version: 0,
            script: scriptPublicKey
        };
    }
    
    console.warn('❌ Unknown scriptPublicKey format (offline):', typeof scriptPublicKey, scriptPublicKey);
    return {
        version: 0,
        script: '0020000000000000000000000000000000000000000000000000000000000000000000' // 32-byte zero script as fallback
    };
}

// Helper function to normalize address for UTXO entries
function normalizeAddressForUTXO(address, networkType) {
    if (!address) {
        return null;
    }
    
    // If address is already a string, ensure it has the correct prefix
    if (typeof address === 'string') {
        // Check if it already has a prefix
        if (address.includes(':')) {
            return address; // Already has prefix, return as is
        }
        
        // Add prefix based on network type
        const networkPrefixes = {
            'mainnet': 'kaspa:',
            'testnet-10': 'kaspatest:',
            'testnet-11': 'kaspatest:',
            'devnet': 'kaspadev:',
            'simnet': 'kaspasim:'
        };
        const prefix = networkPrefixes[networkType] || 'kaspa:';
        return prefix + address;
    }
    
    // If address is a WASM Address object, convert to string
    if (typeof address === 'object' && address.toString) {
        try {
            return address.toString();
        } catch (e) {
            console.warn('Failed to convert WASM address to string:', e);
            return null;
        }
    }
    
    // If address is an object with a payload property (sometimes happens with WASM objects)
    if (typeof address === 'object' && address.payload) {
        return address.payload;
    }
    
    // If address is an object with a prefix property, extract the actual address
    if (typeof address === 'object' && address.prefix) {
        return address.prefix;
    }
    
    // Fallback: try to convert to string
    try {
        return String(address);
    } catch (e) {
        console.warn('Failed to convert address to string:', e);
        return null;
    }
}

// Create transaction using cached UTXOs (offline mode)
async function createOfflineTransaction(fromAddress, toAddress, amount, networkType, cachedUTXOs, options = {}) {
   
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    if (!cachedUTXOs || !cachedUTXOs.utxos || cachedUTXOs.utxos.length === 0) {
        throw new Error('No cached UTXOs available. Please fetch UTXOs first.');
    }

    try {
        const kaspa = getKaspa();
        const { createTransactions } = kaspa;

        // Convert KAS to sompi manually
        const amountFloat = parseFloat(amount);
        
        if (isNaN(amountFloat) || amountFloat <= 0) {
            throw new Error('Invalid amount: must be a positive number');
        }
        
        // Safe conversion using currency utilities
        const { kasNumberToSompi } = await import('./currency-utils.js');
        const amountInSompi = kasNumberToSompi(amountFloat);
        
        // Use cached UTXOs
        const entries = cachedUTXOs.utxos;
        
        // Validate that we have enough UTXOs
        if (!entries || entries.length === 0) {
            throw new Error('No UTXOs available in cached data. Please fetch fresh UTXOs.');
        }
        
        // For multi-address UTXOs, we need to ensure we have the address information
        // The UTXOs should include address information for proper private key mapping during signing
        entries.forEach((utxo, index) => {
            if (!utxo.address) {
                console.warn(`UTXO at index ${index} missing address information`);
            }
        });
        
        // Calculate total available balance from cached UTXOs
        const totalBalance = entries.reduce((sum, entry) => {
            // Handle both 'amount' and 'value' properties for WASM objects
            const entryAmount = entry.amount || entry.value || 0;
            if (!entryAmount && entryAmount !== 0) {
                console.warn('UTXO entry missing amount/value property:', entry);
                return sum;
            }
            return sum + BigInt(entryAmount);
        }, 0n);
        
        // Get unique addresses that have UTXOs (for informational purposes)
        const sourceAddresses = [...new Set(entries.map(utxo => utxo.address).filter(addr => addr))];
        
        // Extract options
        const { feeOption = 'normal', changeAddress = fromAddress, manualFeeKas } = options;
        
        let priorityFee;
        let isManualFee = false;
        
        if (manualFeeKas !== undefined && manualFeeKas !== null) {
            // Manual fee mode
            const manualFeeFloat = parseFloat(manualFeeKas);
            if (isNaN(manualFeeFloat) || manualFeeFloat < 0) {
                throw new Error('Invalid manual fee: must be a non-negative number');
            }
            priorityFee = kasNumberToSompi(manualFeeFloat);
            isManualFee = true;
        } else {
            // Automatic fee calculation based on option
            const priorityFeeRates = {
                'slow': 0n,
                'normal': 1000n, 
                'fast': 5000n
            };
            priorityFee = priorityFeeRates[feeOption] || priorityFeeRates['normal'];
        }
        
        // Check if we have enough balance including fee
        const totalRequired = amountInSompi + priorityFee;
        
        if (totalBalance < totalRequired) {
            const { sompiToKas } = await import('./currency-utils.js');
            const balanceKas = sompiToKas(totalBalance);
            const requiredKas = sompiToKas(totalRequired);
            throw new Error(`Insufficient funds. Available: ${balanceKas} KAS, Required: ${requiredKas} KAS (amount + fee)`);
        }
                
        const { transactions, summary } = await createTransactions({
            entries,
            outputs: [{
                address: toAddress,
                amount: amountInSompi
            }],
            priorityFee: priorityFee,
            changeAddress: changeAddress,
            networkId: networkType
        });
                
        if (!transactions || transactions.length === 0) {
            throw new Error('Failed to create transactions - insufficient funds or invalid parameters');
        }
        
        // Get the first pending transaction
        const pendingTransaction = transactions[0];
        
        // Generate a simple transaction ID
        const transactionId = (isManualFee ? 'offline_manual_' : 'offline_auto_') + Date.now() + '_' + Math.random().toString(36).substring(7);
        
        // Extract actual input addresses from the created transaction
        const inputAddresses = [];
        if (pendingTransaction && pendingTransaction.inputs) {
            for (const input of pendingTransaction.inputs) {
                if (input.address) {
                    inputAddresses.push(input.address);
                }
            }
        }

        return {
            success: true,
            pendingTransaction: pendingTransaction,
            transactions: transactions,
            transactionId: transactionId,
            fromAddress: fromAddress, // Original request address
            toAddress: toAddress,
            amount: amount,
            amountInSompi: amountInSompi.toString(),
            fee: {
                priorityFee: priorityFee.toString(),
                feeInSompi: priorityFee.toString(),
                feeInKas: sompiToKas(priorityFee),
                manual: isManualFee
            },
            summary: summary,
            networkType: networkType,
            status: 'created_offline',
            feeMode: isManualFee ? 'manual' : feeOption,
            utxoSource: 'cached',
            utxoTimestamp: cachedUTXOs.timestamp,
            utxoCount: cachedUTXOs.count || entries.length,
            // Multi-address information
            sourceAddresses: sourceAddresses, // All addresses that had UTXOs available
            inputAddresses: [...new Set(inputAddresses)], // Actual addresses used in transaction inputs
            isMultiAddress: sourceAddresses.length > 1 || inputAddresses.length > 1,
            // CRITICAL: Preserve original UTXO entries for offline QR generation
            utxoEntries: entries.map((entry, idx) => {
                console.log(`🔍 Processing UTXO entry ${idx} for QR storage (offline):`, {
                    keys: Object.keys(entry),
                    hasOutpoint: !!entry.outpoint,
                    outpointKeys: entry.outpoint ? Object.keys(entry.outpoint) : [],
                    outpointValues: entry.outpoint,
                    hasTransactionId: !!entry.outpoint?.transactionId,
                    hasIndex: entry.outpoint?.index !== undefined,
                    directTransactionId: entry.transactionId,
                    directIndex: entry.index
                });
                
                const normalizedAddress = normalizeAddressForUTXO(entry.address, networkType);
                if (!normalizedAddress) {
                    console.warn('Failed to normalize address for UTXO entry:', entry);
                }
                
                // Extract outpoint data more robustly
                let outpoint;
                if (entry.outpoint) {
                    outpoint = {
                        transactionId: entry.outpoint.transactionId || entry.outpoint.txId || entry.outpoint.id,
                        index: entry.outpoint.index !== undefined ? entry.outpoint.index : 
                               (entry.index !== undefined ? entry.index : 0)
                    };
                } else {
                    // Fallback: try to construct outpoint from direct properties
                    outpoint = {
                        transactionId: entry.transactionId || entry.txId || entry.id || `fallback_${Date.now()}_${idx}`,
                        index: entry.index !== undefined ? entry.index : 0
                    };
                }
                
                console.log(`✅ Constructed outpoint for UTXO ${idx} (offline):`, outpoint);
                
                return {
                    outpoint: outpoint,
                    address: normalizedAddress,
                    amount: entry.amount || entry.value,
                    scriptPublicKey: extractScriptPublicKeyData(entry.scriptPublicKey),
                    blockDaaScore: entry.blockDaaScore,
                    // Additional metadata that might be useful for offline signing
                    isCoinbase: entry.isCoinbase || false
                };
            })
        };

    } catch (error) {
        console.error('Offline transaction creation error details:', {
            message: error.message,
            stack: error.stack,
            name: error.name,
            error: error,
            errorType: typeof error,
            errorConstructor: error.constructor?.name,
            errorKeys: Object.keys(error || {}),
            errorString: String(error),
            errorJSON: JSON.stringify(error, Object.getOwnPropertyNames(error))
        });
        
        let errorMessage = 'Unknown error occurred during offline transaction creation';
        if (error && typeof error === 'object') {
            if (error.message) {
                errorMessage = error.message;
            } else if (error.toString && error.toString() !== '[object Object]') {
                errorMessage = error.toString();
            } else if (error.error) {
                errorMessage = error.error;
            } else {
                errorMessage = `Error object: ${JSON.stringify(error, Object.getOwnPropertyNames(error))}`;
            }
        } else if (error) {
            errorMessage = String(error);
        }
        
        return {
            success: false,
            error: errorMessage
        };
    }
}

// Create transaction with manual fee using cached UTXOs
async function createOfflineTransactionWithManualFee(fromAddress, toAddress, amount, manualFeeKas, networkType, cachedUTXOs) {
    return await createOfflineTransaction(fromAddress, toAddress, amount, networkType, cachedUTXOs, {
        manualFeeKas: manualFeeKas,
        changeAddress: fromAddress
    });
}

// Validate cached UTXO data
function validateCachedUTXOs(cachedUTXOs, networkType) {
    if (!cachedUTXOs) {
        return {
            valid: false,
            error: 'No cached UTXO data available'
        };
    }
    
    if (!cachedUTXOs.utxos || !Array.isArray(cachedUTXOs.utxos)) {
        return {
            valid: false,
            error: 'Invalid UTXO data structure'
        };
    }
    
    if (cachedUTXOs.networkType && cachedUTXOs.networkType !== networkType) {
        return {
            valid: false,
            error: `Network type mismatch. Expected ${networkType}, got ${cachedUTXOs.networkType}`
        };
    }
    
    if (cachedUTXOs.utxos.length === 0) {
        return {
            valid: false,
            error: 'No UTXOs available in cached data'
        };
    }
    
    // Check if UTXOs are too old (optional warning)
    const maxAge = 30 * 60 * 1000; // 30 minutes
    const now = Date.now();
    const age = now - (cachedUTXOs.timestamp || 0);
    
    if (age > maxAge) {
        return {
            valid: true,
            warning: `UTXO data is ${Math.floor(age / 60000)} minutes old. Consider refreshing for accuracy.`
        };
    }
    
    return {
        valid: true
    };
}

// Calculate balance from cached UTXOs using centralized balance manager
async function calculateCachedBalance(cachedUTXOs) {
    if (!cachedUTXOs || !cachedUTXOs.utxos || cachedUTXOs.utxos.length === 0) {
        return {
            totalBalance: 0,
            totalBalanceSompi: 0n,
            utxoCount: 0
        };
    }

    const { calculateBalanceFromUTXOs } = await import('./balance-manager.js');
    return calculateBalanceFromUTXOs(cachedUTXOs.utxos);
}

// Export transaction data for external use
function exportOfflineTransaction(transactionData) {
    if (!transactionData || !transactionData.success) {
        throw new Error('Invalid transaction data');
    }

    const exportData = {
        type: 'kaspa-offline-transaction',
        version: '1.0',
        transactionId: transactionData.transactionId,
        fromAddress: transactionData.fromAddress,
        toAddress: transactionData.toAddress,
        amount: transactionData.amount,
        amountInSompi: transactionData.amountInSompi,
        fee: transactionData.fee,
        networkType: transactionData.networkType,
        status: transactionData.status,
        feeMode: transactionData.feeMode,
        utxoSource: transactionData.utxoSource,
        utxoTimestamp: transactionData.utxoTimestamp,
        utxoCount: transactionData.utxoCount,
        // Multi-address support
        sourceAddresses: transactionData.sourceAddresses || [],
        inputAddresses: transactionData.inputAddresses || [],
        isMultiAddress: transactionData.isMultiAddress || false,
        createdAt: Date.now(),
        pendingTransaction: transactionData.pendingTransaction ? {
            // Export only essential transaction data
            id: transactionData.transactionId,
            inputs: transactionData.pendingTransaction.inputs,
            outputs: transactionData.pendingTransaction.outputs,
            version: transactionData.pendingTransaction.version,
            lockTime: transactionData.pendingTransaction.lockTime
        } : null
    };

    return {
        success: true,
        exportData: exportData,
        jsonString: JSON.stringify(exportData, null, 2)
    };
}

export {
    createOfflineTransaction,
    createOfflineTransactionWithManualFee,
    validateCachedUTXOs,
    calculateCachedBalance,
    exportOfflineTransaction
}; 