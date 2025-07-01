// Kaspa Transaction Creation Module
import { getKaspa, isInitialized } from './init.js';
import { calculateTransactionFee } from './fee-calculator.js';
import { fetchUTXOsForAddress, fetchUTXOsForAddresses } from './address-scanner.js';
import { MAX_UTXOS_PER_TRANSACTION, UTXO_CONSOLIDATION_THRESHOLD } from './constants.js';

// Smart UTXO selection to avoid storage mass limits
function selectOptimalUTXOs(utxos, requiredAmount, maxUtxos = MAX_UTXOS_PER_TRANSACTION) {
    if (!utxos || utxos.length === 0) {
        return { selected: [], totalValue: 0n, sufficient: false };
    }

    // Convert UTXOs to a standardized format and sort by value (largest first)
    const standardizedUtxos = utxos.map(utxo => ({
        original: utxo,
        value: BigInt(utxo.amount || utxo.value || 0)
    })).sort((a, b) => {
        // Sort by value descending (largest first)
        if (a.value > b.value) return -1;
        if (a.value < b.value) return 1;
        return 0;
    });

    let selectedUtxos = [];
    let totalValue = 0n;
    const requiredAmountBigInt = BigInt(requiredAmount);

    // Strategy 1: Try to find a single UTXO that covers the amount (best for privacy and fees)
    for (const utxo of standardizedUtxos) {
        if (utxo.value >= requiredAmountBigInt) {
            return {
                selected: [utxo.original],
                totalValue: utxo.value,
                sufficient: true,
                strategy: 'single-utxo'
            };
        }
    }

    // Strategy 2: Select largest UTXOs first until we have enough (greedy approach)
    for (const utxo of standardizedUtxos) {
        if (selectedUtxos.length >= maxUtxos) {
            break;
        }
        
        selectedUtxos.push(utxo.original);
        totalValue += utxo.value;
        
        if (totalValue >= requiredAmountBigInt) {
            return {
                selected: selectedUtxos,
                totalValue: totalValue,
                sufficient: true,
                strategy: 'greedy-selection'
            };
        }
    }

    // If we reach here, we don't have enough funds even with all UTXOs
    return {
        selected: selectedUtxos,
        totalValue: totalValue,
        sufficient: false,
        strategy: 'insufficient-funds'
    };
}

// Legacy wrapper for backward compatibility
async function fetchUTXOs(address, networkType) {
    const result = await fetchUTXOsForAddress(address, networkType);
    
    if (!result.success) {
        throw new Error(result.error);
    }
    
    return result.utxos;
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
    
    // Handle WASM Address objects or objects with toString method
    if (typeof address === 'object') {
        if (address.toString && typeof address.toString === 'function') {
            try {
                const addressStr = address.toString();
                // Check if the toString result includes a prefix
                if (typeof addressStr === 'string' && addressStr.includes(':')) {
                    return addressStr;
                }
                // If no prefix, add one based on network type
                const networkPrefixes = {
                    'mainnet': 'kaspa:',
                    'testnet-10': 'kaspatest:',
                    'testnet-11': 'kaspatest:',
                    'devnet': 'kaspadev:',
                    'simnet': 'kaspasim:'
                };
                const prefix = networkPrefixes[networkType] || 'kaspa:';
                return prefix + addressStr;
            } catch (e) {
                console.warn('Failed to convert address object to string:', e);
                return null;
            }
        }
        
        // Handle address objects with payload property
        if (address.payload) {
            const networkPrefixes = {
                'mainnet': 'kaspa:',
                'testnet-10': 'kaspatest:',
                'testnet-11': 'kaspatest:',
                'devnet': 'kaspadev:',
                'simnet': 'kaspasim:'
            };
            const prefix = networkPrefixes[networkType] || 'kaspa:';
            return prefix + address.payload;
        }
    }
    
    // Fallback string conversion
    try {
        return String(address);
    } catch (e) {
        console.warn('Failed to convert address to string:', e);
        return null;
    }
}

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
        console.log('🔍 Extracting data from WASM scriptPublicKey:', {
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
            console.warn('Failed to access WASM scriptPublicKey properties directly:', e);
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
            console.warn('Failed to convert WASM scriptPublicKey to string:', e);
        }
        
        console.warn('❌ Could not extract data from WASM scriptPublicKey, returning fallback');
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
    
    console.warn('❌ Unknown scriptPublicKey format:', typeof scriptPublicKey, scriptPublicKey);
    return {
        version: 0,
        script: '0020000000000000000000000000000000000000000000000000000000000000000000' // 32-byte zero script as fallback
    };
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
        
        // Safe conversion using currency utilities
        const { kasNumberToSompi } = await import('./currency-utils.js');
        const amountInSompi = kasNumberToSompi(amountFloat);
        const manualFeeInSompi = kasNumberToSompi(manualFeeFloat);
        
        // Fetch real UTXOs for the address
        const allEntries = await fetchUTXOs(fromAddress, networkType);
        
        if (!allEntries || allEntries.length === 0) {
            throw new Error('No UTXOs found for this address. The address either has no balance, does not exist on the network, or all UTXOs are already spent. Please check the address and ensure it has sufficient funds.');
        }
        
        const totalRequired = amountInSompi + manualFeeInSompi;
        
        // Use smart UTXO selection to avoid storage mass limits
        const utxoSelection = selectOptimalUTXOs(allEntries, totalRequired);
        
        if (!utxoSelection.sufficient) {
            const { sompiToKas } = await import('./currency-utils.js');
            const availableKas = sompiToKas(utxoSelection.totalValue);
            const requiredKas = sompiToKas(totalRequired);
            throw new Error(`Insufficient funds. Available: ${availableKas} KAS, Required: ${requiredKas} KAS (amount + fee)`);
        }
        
        const entries = utxoSelection.selected;
        
        // Add helpful info about UTXO selection
        if (allEntries.length > UTXO_CONSOLIDATION_THRESHOLD) {
            console.log(`Transaction using ${entries.length} of ${allEntries.length} UTXOs (${utxoSelection.strategy}). Consider consolidating UTXOs to reduce future transaction costs.`);
        }
        
        // Use createTransactions with manual fee as priorityFee
        let transactions, summary;
        
        try {
            ({ transactions, summary } = await createTransactions({
                entries,
                outputs: [{
                    address: toAddress,
                    amount: amountInSompi
                }],
                priorityFee: manualFeeInSompi, // Use manual fee as priority fee
                changeAddress: fromAddress,
                networkId: networkType
            }));
        } catch (wasmError) {
            // Handle storage mass exceeded error specifically
            if (wasmError.message && wasmError.message.includes('Storage mass exceeds maximum')) {
                // Try with fewer UTXOs
                const reducedMaxUtxos = Math.max(10, Math.floor(entries.length / 2));
                console.log(`Storage mass exceeded with ${entries.length} UTXOs, retrying with max ${reducedMaxUtxos} UTXOs`);
                
                const reducedSelection = selectOptimalUTXOs(allEntries, totalRequired, reducedMaxUtxos);
                
                if (!reducedSelection.sufficient) {
                    throw new Error(`Transaction too large: This transaction would require ${entries.length} UTXOs, but the maximum allowed is ${reducedMaxUtxos}. Consider consolidating your UTXOs first by sending smaller amounts to yourself.`);
                }
                
                // Retry with fewer UTXOs
                ({ transactions, summary } = await createTransactions({
                    entries: reducedSelection.selected,
                    outputs: [{
                        address: toAddress,
                        amount: amountInSompi
                    }],
                    priorityFee: manualFeeInSompi,
                    changeAddress: fromAddress,
                    networkId: networkType
                }));
            } else {
                throw wasmError;
            }
        }
        
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
            feeMode: 'manual',
            // CRITICAL: Preserve original UTXO entries for offline QR generation
            utxoEntries: entries.map((entry, idx) => {
                console.log(`🔍 Processing UTXO entry ${idx} for QR storage:`, {
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
                
                console.log(`✅ Constructed outpoint for UTXO ${idx}:`, outpoint);
                
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
        console.error('Transaction creation error details:', {
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
        
        let errorMessage = 'Unknown error occurred';
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

// Create transaction using Kaspa WASM SDK createTransactions
async function createTransaction(fromAddress, toAddress, amount, networkType, options = {}) {
    try {
    const initialized = isInitialized();
    
    if (!initialized) {
        console.error('Kaspa WASM not initialized');
        throw new Error('Kaspa WASM not initialized');
    }

    let kaspa;
    try {
        kaspa = getKaspa();
    } catch (error) {
        console.error('Failed to get Kaspa instance:', error);
        throw error;
    }

    try {
        const { createTransactions, kaspaToSompi } = kaspa;
        const { sompiToKas } = await import('./currency-utils.js');
        
        // Convert KAS to sompi manually (kaspaToSompi was causing index out of bounds)
        const amountFloat = parseFloat(amount);
        
        if (isNaN(amountFloat) || amountFloat <= 0) {
            console.error('Invalid amount:', amountFloat);
            throw new Error('Invalid amount: must be a positive number');
        }
        
        // Safe conversion using currency utilities
        const { kasNumberToSompi } = await import('./currency-utils.js');
        const amountInSompi = kasNumberToSompi(amountFloat);
        
        // Check if we have multiple addresses (HD wallet) or single address
        let entries;
        if (options.hdWallet) {
            // For HD wallets, first try to get addresses with balance
            let allAddresses = options.hdWallet.getAllAddresses()
                .filter(addr => addr.balance > 0n)
                .map(addr => addr.address);
                        
            // If no addresses have balance information, check all generated addresses dynamically
            if (allAddresses.length === 0) {
                const allGeneratedAddresses = options.hdWallet.getAllAddresses().map(addr => addr.address);
                
                if (allGeneratedAddresses.length === 0) {
                    throw new Error('No addresses generated in HD wallet. Please generate at least one address first.');
                }
                
                // Check balance for each address to find those with UTXOs
                const { getSingleWallet } = await import('./wallet-manager.js');
                const addressesWithBalance = [];
                
                for (const address of allGeneratedAddresses) {
                    try {
                        const singleWallet = getSingleWallet(address, networkType);
                        await singleWallet.initialize();
                        const balanceResult = await singleWallet.checkSingleAddressBalance(address);
                        if (balanceResult.success && balanceResult.balance.kas > 0) {
                            addressesWithBalance.push(address);
                            // Update the HD wallet with this balance information
                            const { kasToSompi } = await import('./currency-utils.js');
                            const balanceInSompi = kasToSompi(balanceResult.balance.kas.toString());
                            options.hdWallet.updateAddressBalance(address, balanceInSompi, balanceResult.utxos || []);
                        }
                    } catch (balanceError) {
                        console.warn(`Failed to check balance for address ${address}:`, balanceError);
                    }
                }
                
                allAddresses = addressesWithBalance;
            }
            
            if (allAddresses.length === 0) {
                throw new Error('No addresses with balance found in HD wallet. Please ensure the wallet has received funds and try again.');
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
        
        // Extract options and calculate priority fee BEFORE UTXO selection
        const { feeOption = 'normal', changeAddress } = options;
        const finalChangeAddress = changeAddress || fromAddress;
        
        // Calculate priority fee based on option
        const priorityFeeRates = {
            'slow': 0n,
            'normal': 1000n, 
            'fast': 5000n
        };
        const priorityFee = priorityFeeRates[feeOption] || priorityFeeRates['normal'];
        
        // Apply smart UTXO selection to avoid storage mass limits
        const totalRequired = amountInSompi + priorityFee; // Rough estimate, actual fee will be calculated by WASM
        const utxoSelection = selectOptimalUTXOs(entries, totalRequired);
        
        if (!utxoSelection.sufficient) {
            // Try with all UTXOs if selection wasn't sufficient (maybe fee estimation was off)
            console.log('Initial UTXO selection insufficient, trying with all UTXOs');
        } else {
            entries = utxoSelection.selected;
            
            // Add helpful info about UTXO selection
            if (entries.length > UTXO_CONSOLIDATION_THRESHOLD) {
                console.log(`Transaction using ${entries.length} UTXOs (${utxoSelection.strategy}). Consider consolidating UTXOs to reduce future transaction costs.`);
            }
        }
                
        let transactions, summary;
        
        try {
            ({ transactions, summary } = await createTransactions({
                entries,
                outputs: [{
                    address: toAddress,
                    amount: amountInSompi
                }],
                priorityFee: priorityFee,
                changeAddress: finalChangeAddress, // Use provided change address or fallback to fromAddress
                networkId: networkType
            }));
        } catch (wasmError) {
            // Handle storage mass exceeded error specifically
            if (wasmError.message && wasmError.message.includes('Storage mass exceeds maximum')) {
                // Try with fewer UTXOs
                const reducedMaxUtxos = Math.max(10, Math.floor(entries.length / 2));
                console.log(`Storage mass exceeded with ${entries.length} UTXOs, retrying with max ${reducedMaxUtxos} UTXOs`);
                
                const reducedSelection = selectOptimalUTXOs(entries, amountInSompi, reducedMaxUtxos);
                
                if (!reducedSelection.sufficient) {
                    throw new Error(`Transaction too large: This transaction would require ${entries.length} UTXOs, but the maximum allowed is ${reducedMaxUtxos}. Consider consolidating your UTXOs first by sending smaller amounts to yourself.`);
                }
                
                // Retry with fewer UTXOs
                ({ transactions, summary } = await createTransactions({
                    entries: reducedSelection.selected,
                    outputs: [{
                        address: toAddress,
                        amount: amountInSompi
                    }],
                    priorityFee: priorityFee,
                    changeAddress: finalChangeAddress,
                    networkId: networkType
                }));
            } else {
                throw wasmError;
            }
        }
        
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
            changeAddress: finalChangeAddress,
            fee: {
                priorityFee: priorityFee.toString(),
                feeInSompi: summary.fees,
                feeInKas: sompiToKas(summary.fees),
                changeAmount: summary.changeAmount ? sompiToKas(summary.changeAmount) : '0.00000000',
                manual: false // Flag to indicate automatic fee
            },
            summary: summary,
            networkType: networkType,
            status: 'created',
            feeMode: 'automatic',
            // CRITICAL: Preserve original UTXO entries for offline QR generation
            utxoEntries: entries.map((entry, idx) => {
                console.log(`🔍 Processing UTXO entry ${idx} for QR storage:`, {
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
                
                console.log(`✅ Constructed outpoint for UTXO ${idx}:`, outpoint);
                
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
        console.error('Manual fee transaction creation error details:', {
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
        
        let errorMessage = 'Unknown error occurred';
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
    } catch (outerError) {
        console.error('OUTER CATCH - Error at very start of createTransaction:', outerError);
        console.error('OUTER CATCH - Error details:', {
            message: outerError.message,
            stack: outerError.stack,
            name: outerError.name,
            error: outerError,
            errorType: typeof outerError,
            errorConstructor: outerError.constructor?.name,
            errorKeys: Object.keys(outerError || {}),
            errorString: String(outerError),
            errorJSON: JSON.stringify(outerError, Object.getOwnPropertyNames(outerError))
        });
        
        return {
            success: false,
            error: outerError.message || outerError.toString() || 'Unknown error at function start'
        };
    }
}

// Import centralized export function
import { exportUnsignedTransaction } from './serialization-utils.js';

export {
    createTransaction,
    createTransactionWithManualFee,
    exportUnsignedTransaction as exportTransaction,
    fetchUTXOs
}; 