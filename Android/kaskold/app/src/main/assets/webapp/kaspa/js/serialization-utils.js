// Kaspa Transaction Serialization Utilities
// Centralized functions for converting transaction data to/from various formats

/**
 * Convert BigInt values to strings for JSON serialization
 * @param {*} obj - Object to convert
 * @returns {*} - Object with BigInt values converted to strings
 */
export function convertBigIntToString(obj) {
    if (obj === null || obj === undefined) {
        return obj;
    }
    
    if (typeof obj === 'bigint') {
        return obj.toString();
    }
    
    if (Array.isArray(obj)) {
        return obj.map(convertBigIntToString);
    }
    
    if (typeof obj === 'object') {
        const converted = {};
        for (const [key, value] of Object.entries(obj)) {
            converted[key] = convertBigIntToString(value);
        }
        return converted;
    }
    
    return obj;
}

/**
 * Convert string values back to BigInt where appropriate
 * @param {*} obj - Object to convert
 * @param {string[]} bigIntFields - Fields that should be converted to BigInt
 * @returns {*} - Object with string values converted back to BigInt
 */
export function convertStringToBigInt(obj, bigIntFields = ['amount', 'fee', 'value', 'satoshis', 'lockTime', 'gas']) {
    if (obj === null || obj === undefined) {
        return obj;
    }
    
    if (Array.isArray(obj)) {
        return obj.map(item => convertStringToBigInt(item, bigIntFields));
    }
    
    if (typeof obj === 'object') {
        const converted = {};
        for (const [key, value] of Object.entries(obj)) {
            if (bigIntFields.includes(key) && typeof value === 'string' && /^\d+$/.test(value)) {
                converted[key] = BigInt(value);
            } else {
                converted[key] = convertStringToBigInt(value, bigIntFields);
            }
        }
        return converted;
    }
    
    return obj;
}

/**
 * Safe serialization of WASM objects for QR codes and JSON storage
 * @param {Object} wasmObject - WASM object (Transaction, PendingTransaction, etc.)
 * @returns {Object} - Serialized object safe for JSON
 */
export function serializeWasmObject(wasmObject) {
    if (!wasmObject) {
        return null;
    }
    
    console.log('🔍 WASM Object Analysis:', {
        type: typeof wasmObject,
        constructor: wasmObject.constructor?.name || 'unknown',
        hasWbgPtr: '__wbg_ptr' in wasmObject,
        wbgPtr: wasmObject.__wbg_ptr,
        isWasmObject: wasmObject.__wbg_ptr !== undefined,
        availableMethods: Object.getOwnPropertyNames(Object.getPrototypeOf(wasmObject)).filter(name => typeof wasmObject[name] === 'function')
    });
    
    try {
        // Try serializeToObject first (most reliable)
        if (typeof wasmObject.serializeToObject === 'function') {
            console.log('✅ Using serializeToObject() method');
            const result = wasmObject.serializeToObject();
            console.log('📦 serializeToObject result:', result ? Object.keys(result) : null);
            return result;
        }
        
        // Try serializeToSafeJSON as fallback
        if (typeof wasmObject.serializeToSafeJSON === 'function') {
            console.log('✅ Using serializeToSafeJSON() as fallback');
            const jsonStr = wasmObject.serializeToSafeJSON();
            const result = JSON.parse(jsonStr);
            console.log('📦 serializeToSafeJSON result:', result ? Object.keys(result) : null);
            return result;
        }
        
        // Try toJSON as another fallback
        if (typeof wasmObject.toJSON === 'function') {
            console.log('✅ Using toJSON() as fallback');
            const result = wasmObject.toJSON();
            console.log('📦 toJSON result:', result ? Object.keys(result) : null);
            return result;
        }
        
        // Manual extraction as last resort
        console.warn('⚠️ No serialization method found, extracting manually');
        console.log('🔍 Available methods:', Object.getOwnPropertyNames(Object.getPrototypeOf(wasmObject)));
        
        // Try to get the actual WASM object data by calling toString or other methods
        let fallbackData = {};
        
        // Try to extract data via direct property access
        try {
            const wasmKeys = Object.keys(wasmObject);
            console.log('🔍 Direct WASM object keys:', wasmKeys);
            
            for (const key of wasmKeys) {
                if (key !== '__wbg_ptr' && typeof wasmObject[key] !== 'function') {
                    fallbackData[key] = wasmObject[key];
                }
            }
        } catch (e) {
            console.warn('Could not extract direct properties:', e);
        }
        
        // If we still have no data, return a minimal structure to prevent errors
        if (Object.keys(fallbackData).length === 0) {
            console.warn('⚠️ No data extracted from WASM object, returning minimal structure');
            fallbackData = {
                id: null,
                version: 1,
                inputs: [],
                outputs: [],
                lockTime: 0n,
                mass: 0,
                gas: 0n,
                subnetworkId: '0000000000000000000000000000000000000000',
                payload: ''
            };
        }
        
        const result = { ...extractWasmObjectData(wasmObject), ...fallbackData };
        console.log('📦 Manual extraction result:', result ? Object.keys(result) : null);
        return result;
        
    } catch (error) {
        console.error('❌ Serialization failed:', error);
        console.error('❌ WASM object details:', {
            type: typeof wasmObject,
            keys: Object.keys(wasmObject),
            proto: Object.getPrototypeOf(wasmObject)
        });
        throw new Error(`Failed to serialize WASM object: ${error.message}`);
    }
}

/**
 * Manual extraction of WASM object properties
 * @param {Object} wasmObject - WASM object to extract data from
 * @returns {Object} - Extracted data
 */
function extractWasmObjectData(wasmObject) {
    const extracted = {};
    
    // Common properties to extract (as getters or direct properties)
    const commonProps = [
        'id', 'transaction', 'mass', 'feeAmount', 'aggregateInputAmount',
        'aggregateOutputAmount', 'changeAmount', 'paymentAmount', 'type',
        'version', 'inputs', 'outputs', 'lockTime', 'subnetworkId', 'gas', 'payload'
    ];
    
    // Try direct property access first
    for (const prop of commonProps) {
        try {
            if (wasmObject[prop] !== undefined && typeof wasmObject[prop] !== 'function') {
                extracted[prop] = wasmObject[prop];
            }
        } catch (e) {
            // Skip properties that can't be accessed
        }
    }
    
    // Try calling getter methods (WASM objects often have getter methods)
    const getterMethods = commonProps.map(prop => `get${prop.charAt(0).toUpperCase() + prop.slice(1)}`);
    const allGetters = [...getterMethods, 'getId', 'getTransaction', 'getMass', 'getFeeAmount', 
                       'getAggregateInputAmount', 'getAggregateOutputAmount', 'getChangeAmount',
                       'getPaymentAmount', 'getType', 'getVersion', 'getInputs', 'getOutputs',
                       'getLockTime', 'getSubnetworkId', 'getGas', 'getPayload'];
    
    for (const getter of allGetters) {
        try {
            if (typeof wasmObject[getter] === 'function') {
                const result = wasmObject[getter]();
                if (result !== undefined) {
                    // Convert getter name back to property name
                    const propName = getter.startsWith('get') ? 
                        getter.slice(3).charAt(0).toLowerCase() + getter.slice(4) : getter;
                    extracted[propName] = result;
                    console.log(`✅ Extracted via ${getter}():`, propName, typeof result);
                }
            }
        } catch (e) {
            // Skip methods that fail
        }
    }
    
    // Try to extract nested transaction data if available
    if (wasmObject.transaction && typeof wasmObject.transaction === 'object') {
        try {
            const nestedTransaction = wasmObject.transaction;
            for (const prop of commonProps) {
                try {
                    if (nestedTransaction[prop] !== undefined && typeof nestedTransaction[prop] !== 'function') {
                        extracted[prop] = nestedTransaction[prop];
                    }
                } catch (e) {
                    // Skip properties that can't be accessed
                }
            }
        } catch (e) {
            console.warn('Could not extract nested transaction data:', e);
        }
    }
    
    // Try specific WASM object methods that might be available
    const wasmSpecificMethods = ['serialize', 'toJson', 'toObject', 'asObject', 'getData'];
    for (const method of wasmSpecificMethods) {
        try {
            if (typeof wasmObject[method] === 'function') {
                const result = wasmObject[method]();
                if (result && typeof result === 'object') {
                    console.log(`✅ Got data from ${method}():`, Object.keys(result));
                    Object.assign(extracted, result);
                }
            }
        } catch (e) {
            // Skip methods that fail
        }
    }
    
    // If we still have very little data, try to get all enumerable properties
    if (Object.keys(extracted).length < 3) {
        try {
            // Get all enumerable properties
            for (const key in wasmObject) {
                try {
                    const value = wasmObject[key];
                    if (value !== undefined && typeof value !== 'function' && key !== '__wbg_ptr') {
                        extracted[key] = value;
                    }
                } catch (e) {
                    // Skip properties that can't be accessed
                }
            }
            
            // Also try Object.getOwnPropertyNames for non-enumerable properties
            const propNames = Object.getOwnPropertyNames(wasmObject);
            for (const propName of propNames) {
                try {
                    if (!extracted.hasOwnProperty(propName) && propName !== '__wbg_ptr') {
                        const value = wasmObject[propName];
                        if (value !== undefined && typeof value !== 'function') {
                            extracted[propName] = value;
                        }
                    }
                } catch (e) {
                    // Skip properties that can't be accessed
                }
            }
        } catch (e) {
            console.warn('Could not enumerate WASM object properties:', e);
        }
    }
    
    console.log('📋 Manually extracted properties:', Object.keys(extracted));
    return extracted;
}

/**
 * Prepare transaction data for WASM SDK deserialization
 * @param {Object} rawTx - Raw transaction data (possibly from QR)
 * @returns {Object} - Transaction data formatted for WASM SDK
 */
export function prepareForWasmDeserialization(rawTx) {
    if (!rawTx) {
        throw new Error('No transaction data provided');
    }
    
    // Validate required fields
    const requiredFields = ['version', 'inputs', 'outputs', 'lockTime'];
    const missingFields = requiredFields.filter(field => rawTx[field] === undefined);
    
    if (missingFields.length > 0) {
        throw new Error(`Missing required fields for deserialization: ${missingFields.join(', ')}`);
    }
    
    // Convert string values back to appropriate types for WASM SDK
    const prepared = {
        ...rawTx,
        // Ensure numeric fields are properly typed
        version: typeof rawTx.version === 'string' ? parseInt(rawTx.version) : rawTx.version,
        lockTime: typeof rawTx.lockTime === 'string' ? BigInt(rawTx.lockTime) : rawTx.lockTime,
        gas: rawTx.gas ? (typeof rawTx.gas === 'string' ? BigInt(rawTx.gas) : rawTx.gas) : BigInt(0),
        mass: rawTx.mass ? (typeof rawTx.mass === 'string' ? parseInt(rawTx.mass) : rawTx.mass) : 0,
        
        // Process inputs array
        inputs: Array.isArray(rawTx.inputs) ? rawTx.inputs.map(input => ({
            ...input,
            sequence: typeof input.sequence === 'string' ? parseInt(input.sequence) : (input.sequence || 0),
            sigOpCount: typeof input.sigOpCount === 'string' ? parseInt(input.sigOpCount) : (input.sigOpCount || 1),
            index: typeof input.index === 'string' ? parseInt(input.index) : input.index,
            // Process UTXO data
            utxo: input.utxo ? {
                ...input.utxo,
                amount: typeof input.utxo.amount === 'string' ? BigInt(input.utxo.amount) : input.utxo.amount,
                blockDaaScore: typeof input.utxo.blockDaaScore === 'string' ? parseInt(input.utxo.blockDaaScore) : input.utxo.blockDaaScore
            } : input.utxo
        })) : [],
        
        // Process outputs array
        outputs: Array.isArray(rawTx.outputs) ? rawTx.outputs.map(output => ({
            ...output,
            value: typeof output.value === 'string' ? BigInt(output.value) : output.value,
            amount: typeof output.amount === 'string' ? BigInt(output.amount) : (output.amount || output.value)
        })) : [],
        
        // Ensure required fields have defaults
        subnetworkId: rawTx.subnetworkId || '0000000000000000000000000000000000000000',
        payload: rawTx.payload || ''
    };
    
    // Validate the prepared data
    if (!Array.isArray(prepared.inputs) || !Array.isArray(prepared.outputs)) {
        throw new Error('Invalid inputs or outputs array after preparation');
    }
    
    console.log('✅ Prepared transaction data for WASM deserialization:', {
        version: prepared.version,
        inputCount: prepared.inputs.length,
        outputCount: prepared.outputs.length,
        lockTime: prepared.lockTime.toString(),
        gas: prepared.gas.toString()
    });
    
    return prepared;
}

/**
 * Create QR-safe transaction data structure
 * @param {Object} transactionData - Original transaction data
 * @param {string} type - Transaction type ('unsigned', 'signed', 'submitted')
 * @returns {Object} - QR-safe data structure
 */
export function createQRTransactionData(transactionData, type = 'unsigned') {
    const baseData = {
        type: `kaspa-${type}-transaction-qr`,
        version: '2.0',
        transactionId: transactionData.transactionId,
        fromAddress: transactionData.fromAddress,
        toAddress: transactionData.toAddress,
        amount: transactionData.amount,
        amountInSompi: transactionData.amountInSompi,
        fee: transactionData.fee,
        feeMode: transactionData.feeMode,
        networkType: transactionData.networkType,
        timestamp: transactionData.timestamp || new Date().toISOString(),
        status: type,
        changeAddress: transactionData.changeAddress
    };
    
    // Add type-specific data
    switch (type) {
        case 'unsigned':
            return {
                ...baseData,
                serializedPendingTransaction: transactionData.serializedPendingTransaction || null,
                transactionDetails: transactionData.transactionDetails || null,
                originalTransactionData: transactionData.originalTransactionData || null
            };
            
        case 'signed':
            return {
                ...baseData,
                serializedTransaction: transactionData.serializedTransaction || null,
                serializedPendingTransaction: transactionData.serializedPendingTransaction || null,
                signedAt: transactionData.signedAt || new Date().toISOString(),
                // Include full transaction details for offline submission
                inputs: transactionData.inputs || [],
                outputs: transactionData.outputs || [],
                version: transactionData.version || 1,
                lockTime: transactionData.lockTime || 0,
                gas: transactionData.gas || 0,
                mass: transactionData.mass || 0,
                subnetworkId: transactionData.subnetworkId || '0000000000000000000000000000000000000000',
                payload: transactionData.payload || '',
                // Include UTXO entries for complete offline capability
                utxoEntries: transactionData.utxoEntries || []
            };
            
        case 'submitted':
            return {
                ...baseData,
                networkResponse: transactionData.networkResponse || null,
                submittedAt: transactionData.submittedAt || new Date().toISOString()
            };
            
        default:
            return baseData;
    }
}

/**
 * Validate transaction data structure for specific type
 * @param {Object} data - Transaction data to validate
 * @param {string} expectedType - Expected transaction type
 * @returns {Object} - Validation result with isValid and error properties
 */
export function validateTransactionData(data, expectedType) {
    if (!data) {
        return { isValid: false, error: 'No transaction data provided' };
    }
    
    // Check required base fields
    const requiredFields = ['transactionId', 'fromAddress', 'toAddress', 'amount', 'networkType'];
    const missingFields = requiredFields.filter(field => !data[field]);
    
    if (missingFields.length > 0) {
        return { isValid: false, error: `Missing required fields: ${missingFields.join(', ')}` };
    }
    
    // Type-specific validation
    switch (expectedType) {
        case 'unsigned':
            if (!data.serializedPendingTransaction && !data.transactionDetails) {
                return { isValid: false, error: 'Unsigned transaction missing serialized data' };
            }
            break;
            
        case 'signed':
            if (!data.serializedTransaction) {
                return { isValid: false, error: 'Signed transaction missing serialized transaction data' };
            }
            break;
            
        case 'submitted':
            if (!data.networkResponse && !data.submittedAt) {
                return { isValid: false, error: 'Submitted transaction missing network response data' };
            }
            break;
    }
    
    return { isValid: true };
}

/**
 * Convert transaction data from QR format to internal format
 * @param {Object} qrData - Data from QR code
 * @returns {Object} - Internal transaction data format
 */
export function convertFromQRFormat(qrData) {
    if (!qrData) {
        throw new Error('No QR data provided');
    }
    
    // Detect transaction type
    let type = 'unknown';
    if (qrData.type === 'kaspa-unsigned-transaction-qr') {
        type = 'unsigned';
    } else if (qrData.type === 'kaspa-signed-transaction-qr') {
        type = 'signed';
    } else if (qrData.type === 'kaspa-submitted-transaction-qr') {
        type = 'submitted';
    }
    
    if (type === 'unknown') {
        throw new Error(`Unrecognized QR transaction format: ${qrData.type}`);
    }
    
    // Convert string values back to appropriate types
    const converted = convertStringToBigInt(qrData);
    
    // Mark as uploaded and add metadata
    return {
        ...converted,
        isUploaded: true,
        uploadedAt: new Date().toISOString(),
        detectedType: type
    };
}

/**
 * Comprehensive transaction data cleaning for submission
 * @param {Object} transactionData - Transaction data to clean
 * @returns {Object} - Cleaned transaction data
 */
export function cleanTransactionDataForSubmission(transactionData) {
    if (!transactionData) {
        throw new Error('No transaction data provided for cleaning');
    }
    
    console.log('🧹 Cleaning transaction data for submission');
    console.log('📊 Input data keys:', Object.keys(transactionData));
    
    // Create a clean copy
    const cleaned = { ...transactionData };
    
    // Remove potentially problematic fields
    delete cleaned.__wbg_ptr;
    delete cleaned.constructor;
    
    // Try to find proper transaction data for submission
    let submissionTransaction = null;
    
    // For uploaded QR transactions, we need to reconstruct the transaction from the QR data
    if (cleaned.isUploaded && cleaned.type === 'kaspa-signed-transaction-qr') {
        console.log('📦 Processing uploaded signed transaction QR for submission');
        
        // Try to find the actual transaction data from various sources
        if (cleaned.serializedTransaction && typeof cleaned.serializedTransaction === 'object') {
            console.log('📦 Using serializedTransaction from QR data');
            submissionTransaction = cleaned.serializedTransaction;
        } else if (cleaned.serializedPendingTransaction && typeof cleaned.serializedPendingTransaction === 'object') {
            console.log('📦 Using serializedPendingTransaction from QR data');
            submissionTransaction = cleaned.serializedPendingTransaction;
        } else if (cleaned.inputs && cleaned.outputs && Array.isArray(cleaned.inputs) && Array.isArray(cleaned.outputs)) {
            // Use the full transaction data from QR
            console.log('📦 Using full transaction data from QR');
            submissionTransaction = {
                version: cleaned.version || 1,
                inputs: cleaned.inputs,
                outputs: cleaned.outputs,
                lockTime: cleaned.lockTime || 0n,
                gas: cleaned.gas || 0n,
                mass: cleaned.mass || 0,
                subnetworkId: cleaned.subnetworkId || '0000000000000000000000000000000000000000',
                payload: cleaned.payload || ''
            };
        } else {
            // Try to reconstruct from the basic transaction data
            console.log('📦 Reconstructing transaction from QR basic data');
            submissionTransaction = {
                version: 1,
                inputs: [],
                outputs: [{
                    address: cleaned.toAddress,
                    amount: cleaned.amountInSompi || cleaned.amount
                }],
                lockTime: 0n,
                gas: 0n,
                mass: 0,
                subnetworkId: '0000000000000000000000000000000000000000',
                payload: ''
            };
            
            // Add change output if we have change address info
            if (cleaned.changeAddress && cleaned.fee && cleaned.fee.changeAmount) {
                const changeAmountInSompi = cleaned.fee.changeAmount;
                if (changeAmountInSompi && changeAmountInSompi !== '0') {
                    submissionTransaction.outputs.push({
                        address: cleaned.changeAddress,
                        amount: changeAmountInSompi
                    });
                }
            }
        }
    } else {
        // Priority order for non-uploaded transactions: serializedTransaction > signedTransaction > transactions array > original transaction data
        if (cleaned.serializedTransaction) {
            console.log('📦 Found serializedTransaction for submission');
            submissionTransaction = cleaned.serializedTransaction;
        } else if (cleaned.signedTransaction && typeof cleaned.signedTransaction === 'object') {
            console.log('📦 Using signedTransaction for submission');
            submissionTransaction = cleaned.signedTransaction;
        } else if (cleaned.transactions && Array.isArray(cleaned.transactions) && cleaned.transactions.length > 0) {
            console.log('📦 Using first transaction from transactions array');
            submissionTransaction = cleaned.transactions[0];
        } else if (cleaned.originalTransactionData && cleaned.originalTransactionData.signedTransaction) {
            console.log('📦 Using signedTransaction from originalTransactionData');
            submissionTransaction = cleaned.originalTransactionData.signedTransaction;
        }
    }
    
    // Only prepare for WASM if we have proper transaction structure
    if (submissionTransaction && typeof submissionTransaction === 'object') {
        // Check if it has the required fields before trying to prepare
        const hasRequiredFields = ['version', 'inputs', 'outputs', 'lockTime'].every(field => 
            submissionTransaction.hasOwnProperty(field) || 
            submissionTransaction.hasOwnProperty(field.toLowerCase())
        );
        
        if (hasRequiredFields) {
            try {
                console.log('✅ Preparing transaction for WASM submission');
                cleaned.serializedTransaction = prepareForWasmDeserialization(submissionTransaction);
            } catch (error) {
                console.warn('⚠️ Failed to prepare transaction for WASM:', error);
                // Keep the original transaction data
                cleaned.serializedTransaction = submissionTransaction;
            }
        } else {
            console.log('⚠️ Transaction missing required fields, keeping as-is for submission');
            cleaned.serializedTransaction = submissionTransaction;
        }
    } else {
        console.log('⚠️ No suitable transaction data found for submission preparation');
        
        // Last resort: try to create a minimal transaction structure from available data
        if (cleaned.isUploaded) {
            console.log('🔧 Creating minimal transaction structure for uploaded QR');
            cleaned.serializedTransaction = {
                version: 1,
                inputs: [],
                outputs: [{
                    address: cleaned.toAddress,
                    amount: cleaned.amountInSompi || cleaned.amount
                }],
                lockTime: 0n,
                gas: 0n,
                mass: 0,
                subnetworkId: '0000000000000000000000000000000000000000',
                payload: ''
            };
        }
    }
    
    console.log('🧹 Cleaned transaction data keys:', Object.keys(cleaned));
    return cleaned;
}

/**
 * Export unsigned transaction as JSON for offline signing
 * @param {Object} transactionData - Unsigned transaction data
 * @returns {string} - JSON string ready for file download or QR generation
 */
export function exportUnsignedTransaction(transactionData) {
    if (!transactionData) {
        throw new Error('No transaction data provided for export');
    }
    
    const exportData = {
        type: 'kaspa-unsigned-transaction',
        version: '2.0',
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
        changeAddress: transactionData.changeAddress,
        // Include serialized pending transaction data needed for signing
        serializedPendingTransaction: transactionData.serializedPendingTransaction || null,
        // Include UTXO entries for offline signing
        utxoEntries: transactionData.utxoEntries || [],
        // Include transaction summary
        summary: transactionData.summary || null,
        // Include transaction details for comprehensive offline support
        transactionDetails: transactionData.transactionDetails || null
    };
    
    // Convert BigInt values to strings for JSON serialization
    const serializedData = convertBigIntToString(exportData);
    return JSON.stringify(serializedData, null, 2);
}

/**
 * Export signed transaction as JSON for offline submission
 * @param {Object} signedTransactionData - Signed transaction data
 * @returns {string} - JSON string ready for file download or QR generation
 */
export function exportSignedTransaction(signedTransactionData) {
    if (!signedTransactionData) {
        throw new Error('No signed transaction data provided for export');
    }
    
    // Serialize the signed transaction using centralized utilities
    let serializedTransaction = null;
    if (signedTransactionData.signedTransaction) {
        try {
            serializedTransaction = serializeWasmObject(signedTransactionData.signedTransaction);
        } catch (serializeError) {
            console.warn('Could not serialize signed transaction for export:', serializeError);
            // Use existing serialized data if available
            serializedTransaction = signedTransactionData.serializedTransaction || null;
        }
    }
    
    const exportData = {
        type: 'kaspa-signed-transaction',
        version: '2.0',
        transactionId: signedTransactionData.transactionId,
        fromAddress: signedTransactionData.fromAddress,
        toAddress: signedTransactionData.toAddress,
        amount: signedTransactionData.amount,
        amountInSompi: signedTransactionData.amountInSompi,
        fee: signedTransactionData.fee,
        feeMode: signedTransactionData.feeMode || 'automatic',
        networkType: signedTransactionData.networkType,
        timestamp: signedTransactionData.timestamp || new Date().toISOString(),
        status: 'signed',
        signedAt: signedTransactionData.signedAt || new Date().toISOString(),
        changeAddress: signedTransactionData.changeAddress,
        // Include serialized transaction data for submission
        serializedTransaction: serializedTransaction,
        // Include other relevant data
        serializedPendingTransaction: signedTransactionData.serializedPendingTransaction || null,
        summary: signedTransactionData.summary || null,
        utxoEntries: signedTransactionData.utxoEntries || []
    };
    
    // Convert BigInt values to strings for JSON serialization
    const serializedData = convertBigIntToString(exportData);
    return JSON.stringify(serializedData, null, 2);
}

/**
 * Export submitted transaction as JSON for record keeping
 * @param {Object} submittedTransactionData - Submitted transaction data
 * @returns {string} - JSON string ready for file download
 */
export function exportSubmittedTransaction(submittedTransactionData) {
    if (!submittedTransactionData) {
        throw new Error('No submitted transaction data provided for export');
    }
    
    const exportData = {
        type: 'kaspa-submitted-transaction',
        version: '2.0',
        transactionId: submittedTransactionData.transactionId,
        fromAddress: submittedTransactionData.fromAddress,
        toAddress: submittedTransactionData.toAddress,
        amount: submittedTransactionData.amount,
        amountInSompi: submittedTransactionData.amountInSompi,
        fee: submittedTransactionData.fee,
        feeMode: submittedTransactionData.feeMode || 'automatic',
        networkType: submittedTransactionData.networkType,
        timestamp: submittedTransactionData.timestamp || new Date().toISOString(),
        status: 'submitted',
        submittedAt: submittedTransactionData.submittedAt || new Date().toISOString(),
        changeAddress: submittedTransactionData.changeAddress,
        // Include network response data
        networkResponse: submittedTransactionData.networkResponse || null,
        // Include transaction history
        signedAt: submittedTransactionData.signedAt || null,
        createdAt: submittedTransactionData.createdAt || null
    };
    
    // Convert BigInt values to strings for JSON serialization
    const serializedData = convertBigIntToString(exportData);
    return JSON.stringify(serializedData, null, 2);
} 