// Kaspa Fee Calculator Module
import { getKaspa, isInitialized } from './init.js';
import { fetchUTXOsForAddress } from './utxo-fetcher.js';

// Fee calculation constants (kept for backward compatibility)
const FEE_CONSTANTS = {
    SOMPI_PER_GRAM: BigInt(1000), // Base fee rate
    MINIMUM_FEE: BigInt(1000),    // Minimum transaction fee
    INPUT_SIZE: 181,              // Bytes per input (approx)
    OUTPUT_SIZE: 34,              // Bytes per output (approx)
    BASE_SIZE: 10                 // Base transaction size
};

// Calculate transaction size in grams (Kaspa's fee unit) - legacy method
function calculateTransactionSize(inputCount, outputCount) {
    const inputBytes = inputCount * FEE_CONSTANTS.INPUT_SIZE;
    const outputBytes = outputCount * FEE_CONSTANTS.OUTPUT_SIZE;
    const totalBytes = FEE_CONSTANTS.BASE_SIZE + inputBytes + outputBytes;
    
    // Convert bytes to grams (Kaspa uses grams for fee calculation)
    return Math.ceil(totalBytes / 1000);
}

// Legacy fee calculation method
function calculateTransactionFee(inputCount, outputCount, feeRatePerGram = null) {
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    const feeRate = feeRatePerGram || FEE_CONSTANTS.SOMPI_PER_GRAM;
    const sizeInGrams = calculateTransactionSize(inputCount, outputCount);
    const calculatedFee = BigInt(sizeInGrams) * feeRate;
    
    // Ensure minimum fee
    const finalFee = calculatedFee > FEE_CONSTANTS.MINIMUM_FEE ? calculatedFee : FEE_CONSTANTS.MINIMUM_FEE;
    
    return {
        feeInSompi: finalFee,
        feeInKas: Number(finalFee) / 100000000,
        sizeInGrams: sizeInGrams,
        feeRateUsed: feeRate
    };
}

// New accurate fee calculation using SDK's Generator.estimate()
async function calculateAccurateTransactionFee(fromAddress, toAddress, amount, networkType, feeOption = 'normal') {
    
    if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
    }

    try {
        const kaspa = getKaspa();
        const { RpcClient, Resolver, Address, createTransactions } = kaspa;

        if (!createTransactions) {
            throw new Error('createTransactions function not available in Kaspa WASM SDK');
        }

        // Create RPC client using the SDK's resolver for public nodes
        const rpc = new RpcClient({
            networkId: networkType,
            resolver: new Resolver()
        });

        const connectPromise = rpc.connect();
        const connectTimeoutPromise = new Promise((_, reject) => {
            setTimeout(() => reject(new Error('RPC connection timeout after 15 seconds')), 15000);
        });
        
        await Promise.race([connectPromise, connectTimeoutPromise]);

        try {
            // Use centralized UTXO fetcher
            const utxoResult = await fetchUTXOsForAddress(fromAddress, networkType);
            
            if (!utxoResult.success) {
                throw new Error(utxoResult.error);
            }
            
            const entries = utxoResult.utxos;
            
            if (!entries || entries.length === 0) {
                // Return a specific response for no UTXOs - this is not a network error
                return {
                    success: false,
                    error: 'No UTXOs found for this address. The address has no available balance to spend.',
                    isNoUTXOsError: true // Flag to indicate this is not a network issue
                };
            }

            // Validate UTXO entries format
            for (let i = 0; i < entries.length; i++) {
                const entry = entries[i];
                if (!entry.amount) {
                    throw new Error(`UTXO entry ${i} is missing amount field`);
                }
            }

            // Validate addresses
            try {
                new Address(fromAddress);
                new Address(toAddress);
            } catch (error) {
                throw new Error(`Invalid address: ${error.message}`);
            }

            // Convert amount to sompi
            const amountFloat = parseFloat(amount);
            if (isNaN(amountFloat) || amountFloat <= 0) {
                return {
                    success: false,
                    error: 'Invalid amount: must be a positive number',
                    isInvalidAmountError: true
                };
            }
            
            // Check for minimum amount (dust limit - KIP anti-dust fix)
            const MINIMUM_AMOUNT = 0.2; // 0.2 KAS minimum due to dust attack prevention
            if (amountFloat < MINIMUM_AMOUNT) {
                return {
                    success: false,
                    error: `Amount too low: minimum transaction amount is ${MINIMUM_AMOUNT} KAS (anti-dust protection)`,
                    isLowAmountError: true
                };
            }
            
            const amountInSompi = BigInt(Math.floor(amountFloat * 100000000));
            
            // Priority fee rates in sompi
            const priorityFeeRates = {
                'slow': 0n,
                'normal': 1000n, 
                'fast': 5000n
            };
            const priorityFee = priorityFeeRates[feeOption] || priorityFeeRates['normal'];
            
            // Use createTransactions to get fee estimate
            const { summary } = await createTransactions({
                entries,
                outputs: [{
                    address: toAddress,
                    amount: amountInSompi
                }],
                priorityFee: priorityFee,
                changeAddress: fromAddress,
                networkId: networkType
            });
            
            // Calculate fees for all options
            const feeEstimates = {};
            for (const [option, fee] of Object.entries(priorityFeeRates)) {
                try {
                    const { summary: tempSummary } = await createTransactions({
                        entries,
                        outputs: [{
                            address: toAddress,
                            amount: amountInSompi
                        }],
                        priorityFee: fee,
                        changeAddress: fromAddress,
                        networkId: networkType
                    });
                    feeEstimates[option] = Number(tempSummary.fees) / 100000000;
                } catch (error) {
                    // If individual fee calculation fails, use the main estimate
                    feeEstimates[option] = Number(summary.fees) / 100000000;
                }
            }
            
            const selectedFee = Number(summary.fees) / 100000000; // Convert to KAS
            
            return {
                success: true,
                fee: {
                    slow: feeEstimates.slow,
                    normal: feeEstimates.normal,
                    fast: feeEstimates.fast,
                    selected: selectedFee,
                    option: feeOption,
                    // Include raw estimate data
                    estimate: {
                        fees: summary.fees.toString(),
                        transactions: summary.transactions || 1,
                        finalTransactionId: 'estimated'
                    }
                },
                totalCost: amountFloat + selectedFee,
                networkType: networkType
            };

        } finally {
            // Always disconnect RPC client
            await rpc.disconnect();
        }

    } catch (error) {
        return {
            success: false,
            error: error.message
        };
    }
}

// Get recommended fee rate from network (mock for now)
async function getNetworkFeeRate() {
    // In real implementation, this would query the network
    return {
        fast: BigInt(1500),    // Higher fee for faster confirmation
        normal: BigInt(1000),  // Standard fee
        slow: BigInt(500)      // Lower fee for slower confirmation
    };
}

// Calculate fees for different priorities
async function calculateFeeOptions(inputCount, outputCount) {
    const feeRates = await getNetworkFeeRate();
    
    return {
        fast: calculateTransactionFee(inputCount, outputCount, feeRates.fast),
        normal: calculateTransactionFee(inputCount, outputCount, feeRates.normal),
        slow: calculateTransactionFee(inputCount, outputCount, feeRates.slow)
    };
}

export {
    calculateTransactionFee,
    calculateAccurateTransactionFee,
    calculateFeeOptions,
    getNetworkFeeRate,
    FEE_CONSTANTS
}; 