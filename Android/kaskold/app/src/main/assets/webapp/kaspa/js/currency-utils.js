// Kaspa Currency Conversion Utilities
// Safe conversion between KAS and sompi using Kaspa WASM SDK
// This module provides deterministic, precise currency conversions
// without floating point arithmetic errors

import { getKaspa, isInitialized } from './init.js';

/**
 * Convert sompi (smallest unit) to KAS string representation
 * Uses the Kaspa WASM SDK's sompiToKaspaString function for precision
 * @param {bigint|number|string} sompi - Amount in sompi
 * @returns {string} Amount in KAS as a string (e.g., "1.23456789")
 */
export function sompiToKas(sompi) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { sompiToKaspaString } = kaspa;
        
        // Convert to bigint if it's a string or number
        const sompiBigInt = typeof sompi === 'bigint' ? sompi : BigInt(sompi);
        
        // Use the WASM SDK function for precise conversion
        return sompiToKaspaString(sompiBigInt);
    } catch (error) {
        console.error('Error converting sompi to KAS:', error);
        throw new Error(`Failed to convert sompi to KAS: ${error.message}`);
    }
}

/**
 * Convert KAS string to sompi (smallest unit)
 * Uses the Kaspa WASM SDK's kaspaToSompi function for precision
 * @param {string} kas - Amount in KAS as string (e.g., "1.23456789")
 * @returns {bigint} Amount in sompi
 */
export function kasToSompi(kas) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { kaspaToSompi } = kaspa;
        
        // Validate input
        if (typeof kas !== 'string') {
            throw new Error('KAS amount must be a string to avoid floating point precision issues');
        }
        
        // Use the WASM SDK function for precise conversion
        const result = kaspaToSompi(kas);
        
        if (result === undefined) {
            throw new Error('Invalid KAS amount format');
        }
        
        return result;
    } catch (error) {
        console.error('Error converting KAS to sompi:', error);
        throw new Error(`Failed to convert KAS to sompi: ${error.message}`);
    }
}

/**
 * Convert sompi to KAS number (for display purposes only)
 * WARNING: This function should only be used for display, not for calculations
 * @param {bigint|number|string} sompi - Amount in sompi
 * @returns {number} Amount in KAS as number (for display)
 */
export function sompiToKasNumber(sompi) {
    try {
        // First convert to string using precise WASM function
        const kasString = sompiToKas(sompi);
        // Then convert to number for display (this is safe for display purposes)
        return parseFloat(kasString);
    } catch (error) {
        console.error('Error converting sompi to KAS number:', error);
        throw new Error(`Failed to convert sompi to KAS number: ${error.message}`);
    }
}

/**
 * Convert KAS number to sompi (for user input validation)
 * WARNING: This function should only be used for user input validation
 * @param {number} kas - Amount in KAS as number
 * @returns {bigint} Amount in sompi
 */
export function kasNumberToSompi(kas) {
    try {
        // Convert number to string to avoid floating point issues
        const kasString = kas.toString();
        return kasToSompi(kasString);
    } catch (error) {
        console.error('Error converting KAS number to sompi:', error);
        throw new Error(`Failed to convert KAS number to sompi: ${error.message}`);
    }
}

/**
 * Format balance for display with proper precision
 * @param {bigint|number|string} sompi - Amount in sompi
 * @param {number} decimals - Number of decimal places (default: 8)
 * @returns {string} Formatted KAS amount
 */
export function formatKasBalance(sompi, decimals = 8) {
    try {
        const kasString = sompiToKas(sompi);
        const kasNumber = parseFloat(kasString);
        return kasNumber.toFixed(decimals);
    } catch (error) {
        console.error('Error formatting KAS balance:', error);
        return '0.00000000';
    }
}

/**
 * Validate KAS amount string
 * @param {string} kas - Amount in KAS as string
 * @returns {boolean} True if valid KAS amount
 */
export function isValidKasAmount(kas) {
    try {
        if (typeof kas !== 'string') {
            return false;
        }
        
        // Check if it's a valid number
        const number = parseFloat(kas);
        if (isNaN(number) || number < 0) {
            return false;
        }
        
        // Check decimal places (Kaspa has 8 decimal places)
        const parts = kas.split('.');
        if (parts.length > 2) {
            return false; // Multiple decimal points
        }
        
        if (parts.length === 2 && parts[1].length > 8) {
            return false; // Too many decimal places
        }
        
        // Try to convert to sompi to validate
        const sompi = kasToSompi(kas);
        return sompi >= 0n;
    } catch (error) {
        return false;
    }
}

// Re-export centralized balance calculation function
export { calculateBalanceFromUTXOs as calculateTotalBalanceFromUTXOs } from './balance-manager.js';

/**
 * Convert balance result object to use precise conversions
 * @param {Object} balanceResult - Balance result from API
 * @returns {Object} Updated balance result with precise conversions
 */
export function convertBalanceResultToPrecise(balanceResult) {
    if (!balanceResult || !balanceResult.success) {
        return balanceResult;
    }

    try {
        const sompi = BigInt(balanceResult.balance.sompi || 0);
        const kasString = sompiToKas(sompi);

        return {
            ...balanceResult,
            balance: {
                kas: kasString,
                sompi: sompi.toString()
            }
        };
    } catch (error) {
        console.error('Error converting balance result to precise:', error);
        return balanceResult;
    }
}

/**
 * Safe addition of KAS amounts (converts to sompi, adds, then back to KAS)
 * @param {string} kas1 - First KAS amount as string
 * @param {string} kas2 - Second KAS amount as string
 * @returns {string} Sum in KAS as string
 */
export function addKasAmounts(kas1, kas2) {
    try {
        const sompi1 = kasToSompi(kas1);
        const sompi2 = kasToSompi(kas2);
        const totalSompi = sompi1 + sompi2;
        return sompiToKas(totalSompi);
    } catch (error) {
        console.error('Error adding KAS amounts:', error);
        throw new Error(`Failed to add KAS amounts: ${error.message}`);
    }
}

/**
 * Safe subtraction of KAS amounts (converts to sompi, subtracts, then back to KAS)
 * @param {string} kas1 - First KAS amount as string
 * @param {string} kas2 - Second KAS amount as string
 * @returns {string} Difference in KAS as string
 */
export function subtractKasAmounts(kas1, kas2) {
    try {
        const sompi1 = kasToSompi(kas1);
        const sompi2 = kasToSompi(kas2);
        
        if (sompi1 < sompi2) {
            throw new Error('Insufficient funds for subtraction');
        }
        
        const differenceSompi = sompi1 - sompi2;
        return sompiToKas(differenceSompi);
    } catch (error) {
        console.error('Error subtracting KAS amounts:', error);
        throw new Error(`Failed to subtract KAS amounts: ${error.message}`);
    }
}

/**
 * Compare two KAS amounts
 * @param {string} kas1 - First KAS amount as string
 * @param {string} kas2 - Second KAS amount as string
 * @returns {number} -1 if kas1 < kas2, 0 if equal, 1 if kas1 > kas2
 */
export function compareKasAmounts(kas1, kas2) {
    try {
        const sompi1 = kasToSompi(kas1);
        const sompi2 = kasToSompi(kas2);
        
        if (sompi1 < sompi2) return -1;
        if (sompi1 > sompi2) return 1;
        return 0;
    } catch (error) {
        console.error('Error comparing KAS amounts:', error);
        throw new Error(`Failed to compare KAS amounts: ${error.message}`);
    }
}

/**
 * Check if KAS amount is zero
 * @param {string} kas - KAS amount as string
 * @returns {boolean} True if amount is zero
 */
export function isKasAmountZero(kas) {
    try {
        const sompi = kasToSompi(kas);
        return sompi === 0n;
    } catch (error) {
        console.error('Error checking if KAS amount is zero:', error);
        return false;
    }
}

/**
 * Get minimum transaction amount in KAS
 * @returns {string} Minimum amount in KAS as string
 */
export function getMinimumTransactionAmount() {
    // Kaspa minimum transaction amount is 0.2 KAS (anti-dust protection)
    return '0.20000000';
}

/**
 * Check if amount meets minimum transaction requirements
 * @param {string} kas - KAS amount as string
 * @returns {boolean} True if amount meets minimum requirements
 */
export function meetsMinimumTransactionAmount(kas) {
    try {
        return compareKasAmounts(kas, getMinimumTransactionAmount()) >= 0;
    } catch (error) {
        console.error('Error checking minimum transaction amount:', error);
        return false;
    }
} 