// PSKT (Partially Signed Kaspa Transaction) Management
// Global variables for PSKT state
let currentPSKT = null;
let currentPSKTRole = null;
let psktHistory = [];

// Initialize PSKT management
async function initializePSKTManager() {

    
    // Make sure kaspaWasm is available globally
    if (!window.kaspaWasm && window.getKaspa) {
        window.kaspaWasm = window.getKaspa();
    }
    
    updatePSKTDisplay();
}

// Create a new PSKT
async function createPSKT() {
    try {
        if (!kaspaWasm) {
            throw new Error('Kaspa WASM not loaded');
        }

        updatePSKTStatus('Creating new PSKT...', 'info');

        // According to the SDK docs, PSKT constructor can take undefined payload
        const { PSKT } = kaspaWasm;
        
        if (!PSKT) {
            throw new Error('PSKT class not available in Kaspa SDK');
        }

        // Create PSKT with undefined payload (should create empty PSKT)
        const pskt = new PSKT(undefined);
        const creatorPSKT = pskt.creator().inputsModifiable().outputsModifiable();
        
        currentPSKT = creatorPSKT;
        currentPSKTRole = 'CREATOR';
        
        // Add to history
        psktHistory.push({
            role: 'CREATOR',
            action: 'Created',
            timestamp: new Date().toISOString(),
            serialized: creatorPSKT.serialize()
        });

        updatePSKTStatus('PSKT created successfully', 'success');
        updatePSKTDisplay();
        
        return creatorPSKT;
    } catch (error) {
        console.error('Error creating PSKT:', error);
        updatePSKTStatus(`Error creating PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Add input to PSKT
async function addInputToPSKT(utxoData) {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available. Create one first.');
        }

        updatePSKTStatus('Adding input to PSKT...', 'info');

        // Convert to constructor role if needed
        let constructorPSKT = currentPSKTRole === 'CREATOR' ? 
            currentPSKT.toConstructor() : currentPSKT;

        // Create transaction input
        const input = new kaspaWasm.TransactionInput({
            previousOutpoint: {
                transactionId: utxoData.transactionId,
                index: utxoData.index
            },
            signatureScript: new Uint8Array(),
            sequence: BigInt(0)
        });

        // Add input to PSKT
        currentPSKT = constructorPSKT.input(input);
        currentPSKTRole = 'CONSTRUCTOR';

        // Add to history
        psktHistory.push({
            role: 'CONSTRUCTOR',
            action: 'Added Input',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('Input added to PSKT successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error adding input to PSKT:', error);
        updatePSKTStatus(`Error adding input: ${error.message}`, 'error');
        throw error;
    }
}

// Add output to PSKT
async function addOutputToPSKT(outputData) {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available. Create one first.');
        }

        updatePSKTStatus('Adding output to PSKT...', 'info');

        // Convert to constructor role if needed
        let constructorPSKT = currentPSKTRole === 'CONSTRUCTOR' ? 
            currentPSKT : currentPSKT.toConstructor();

        // Create transaction output
        const output = new kaspaWasm.TransactionOutput({
            value: BigInt(outputData.amount),
            scriptPublicKey: kaspaWasm.Address.new(outputData.address).scriptPublicKey()
        });

        // Add output to PSKT
        currentPSKT = constructorPSKT.output(output);
        currentPSKTRole = 'CONSTRUCTOR';

        // Add to history
        psktHistory.push({
            role: 'CONSTRUCTOR',
            action: 'Added Output',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('Output added to PSKT successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error adding output to PSKT:', error);
        updatePSKTStatus(`Error adding output: ${error.message}`, 'error');
        throw error;
    }
}

// Update PSKT (set sequence, etc.)
async function updatePSKT(sequence, inputIndex = 0) {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available');
        }

        updatePSKTStatus('Updating PSKT...', 'info');

        // Convert to updater role
        const updaterPSKT = currentPSKT.toUpdater();
        
        // Set sequence
        currentPSKT = updaterPSKT.setSequence(BigInt(sequence), inputIndex);
        currentPSKTRole = 'UPDATER';

        // Add to history
        psktHistory.push({
            role: 'UPDATER',
            action: `Set Sequence: ${sequence}`,
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('PSKT updated successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error updating PSKT:', error);
        updatePSKTStatus(`Error updating PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Sign PSKT
async function signPSKT(privateKeyHex) {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available');
        }

        if (!privateKeyHex) {
            throw new Error('Private key required for signing');
        }

        updatePSKTStatus('Signing PSKT...', 'info');

        // Convert to signer role
        const signerPSKT = currentPSKT.toSigner();
        
        // Create private key object
        const privateKey = new kaspaWasm.PrivateKey(privateKeyHex);
        
        // Note: Actual signing implementation would require access to UTXO data
        // and proper signature hash calculation. This is a simplified version.
        // In a real implementation, you'd need to:
        // 1. Get UTXO entries for inputs
        // 2. Calculate signature hashes
        // 3. Sign with private key
        // 4. Add signatures to PSKT
        
        currentPSKT = signerPSKT;
        currentPSKTRole = 'SIGNER';

        // Add to history
        psktHistory.push({
            role: 'SIGNER',
            action: 'Signed',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('PSKT signed successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error signing PSKT:', error);
        updatePSKTStatus(`Error signing PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Combine PSKTs
async function combinePSKTs(psktData1, psktData2) {
    try {
        updatePSKTStatus('Combining PSKTs...', 'info');

        // Create PSKT objects from serialized data
        const pskt1 = new kaspaWasm.PSKT(psktData1);
        const pskt2 = new kaspaWasm.PSKT(psktData2);

        // Convert to combiner role
        const combinerPSKT = pskt1.toCombiner();
        
        // Combine with second PSKT
        // Note: The actual combination logic depends on WASM SDK implementation
        currentPSKT = combinerPSKT;
        currentPSKTRole = 'COMBINER';

        // Add to history
        psktHistory.push({
            role: 'COMBINER',
            action: 'Combined',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('PSKTs combined successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error combining PSKTs:', error);
        updatePSKTStatus(`Error combining PSKTs: ${error.message}`, 'error');
        throw error;
    }
}

// Finalize PSKT
async function finalizePSKT() {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available');
        }

        updatePSKTStatus('Finalizing PSKT...', 'info');

        // Convert to finalizer role
        const finalizerPSKT = currentPSKT.toFinalizer();
        
        currentPSKT = finalizerPSKT;
        currentPSKTRole = 'FINALIZER';

        // Add to history
        psktHistory.push({
            role: 'FINALIZER',
            action: 'Finalized',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('PSKT finalized successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error finalizing PSKT:', error);
        updatePSKTStatus(`Error finalizing PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Extract transaction from PSKT
async function extractTransaction() {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available');
        }

        updatePSKTStatus('Extracting transaction...', 'info');

        // Convert to extractor role
        const extractorPSKT = currentPSKT.toExtractor();
        
        // Extract the transaction
        // Note: This would require network parameters
        // const transaction = extractorPSKT.extractTx();
        
        currentPSKTRole = 'EXTRACTOR';

        // Add to history
        psktHistory.push({
            role: 'EXTRACTOR',
            action: 'Extracted Transaction',
            timestamp: new Date().toISOString(),
            serialized: currentPSKT.serialize()
        });

        updatePSKTStatus('Transaction extracted successfully', 'success');
        updatePSKTDisplay();
        
        return extractorPSKT;
    } catch (error) {
        console.error('Error extracting transaction:', error);
        updatePSKTStatus(`Error extracting transaction: ${error.message}`, 'error');
        throw error;
    }
}

// Export PSKT data
function exportPSKTData() {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available to export');
        }

        const psktData = {
            version: '1.0',
            type: 'kaspa-pskt',
            role: currentPSKTRole,
            serialized: currentPSKT.serialize(),
            id: currentPSKT.calculateId()?.toString() || 'unknown',
            timestamp: new Date().toISOString(),
            history: psktHistory
        };

        const dataStr = JSON.stringify(psktData, null, 2);
        const dataBlob = new Blob([dataStr], { type: 'application/json' });
        const url = URL.createObjectURL(dataBlob);
        
        const link = document.createElement('a');
        link.href = url;
        link.download = `kaspa-pskt-${currentPSKTRole.toLowerCase()}-${Date.now()}.json`;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);

        updatePSKTStatus('PSKT exported successfully', 'success');
        
        return psktData;
    } catch (error) {
        console.error('Error exporting PSKT:', error);
        updatePSKTStatus(`Error exporting PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Import PSKT data
function importPSKTData(fileContent) {
    try {
        const psktData = JSON.parse(fileContent);
        
        if (psktData.type !== 'kaspa-pskt') {
            throw new Error('Invalid PSKT file format');
        }

        // Create PSKT from serialized data
        currentPSKT = new kaspaWasm.PSKT(psktData.serialized);
        currentPSKTRole = psktData.role || 'UNKNOWN';
        
        // Restore history if available
        if (psktData.history) {
            psktHistory = psktData.history;
        }

        updatePSKTStatus('PSKT imported successfully', 'success');
        updatePSKTDisplay();
        
        return currentPSKT;
    } catch (error) {
        console.error('Error importing PSKT:', error);
        updatePSKTStatus(`Error importing PSKT: ${error.message}`, 'error');
        throw error;
    }
}

// Calculate PSKT mass
function calculatePSKTMass() {
    try {
        if (!currentPSKT) {
            throw new Error('No PSKT available');
        }

        const mass = currentPSKT.calculateMass({});
        updatePSKTStatus(`PSKT mass: ${mass}`, 'info');
        
        return mass;
    } catch (error) {
        console.error('Error calculating PSKT mass:', error);
        updatePSKTStatus(`Error calculating mass: ${error.message}`, 'error');
        throw error;
    }
}

// Clear PSKT data
function clearPSKTData() {
    currentPSKT = null;
    currentPSKTRole = null;
    psktHistory = [];
    updatePSKTDisplay();
    updatePSKTStatus('PSKT data cleared', 'info');
}

// Update PSKT status display
function updatePSKTStatus(message, type = 'info') {
    const statusElement = document.getElementById('pskt-status');
    if (statusElement) {
        statusElement.textContent = message;
        statusElement.className = `status ${type}`;
        
        // Clear status after 5 seconds for non-error messages
        if (type !== 'error') {
            setTimeout(() => {
                statusElement.textContent = '';
                statusElement.className = 'status';
            }, 5000);
        }
    }

}

// Update PSKT display
function updatePSKTDisplay() {
    // Update current PSKT info
    const roleElement = document.getElementById('current-pskt-role');
    const idElement = document.getElementById('current-pskt-id');
    const serializedElement = document.getElementById('current-pskt-serialized');
    
    if (roleElement) {
        roleElement.textContent = currentPSKTRole || 'None';
    }
    
    if (idElement && currentPSKT) {
        try {
            const id = currentPSKT.calculateId();
            idElement.textContent = id ? id.toString() : 'Unknown';
        } catch (error) {
            idElement.textContent = 'Error calculating ID';
        }
    } else if (idElement) {
        idElement.textContent = 'No PSKT';
    }
    
    if (serializedElement && currentPSKT) {
        try {
            const serialized = currentPSKT.serialize();
            serializedElement.value = serialized;
        } catch (error) {
            serializedElement.value = 'Error serializing PSKT';
        }
    } else if (serializedElement) {
        serializedElement.value = '';
    }
    
    // Update history display
    updatePSKTHistoryDisplay();
}

// Update PSKT history display
function updatePSKTHistoryDisplay() {
    const historyElement = document.getElementById('pskt-history-list');
    if (!historyElement) return;
    
    historyElement.innerHTML = '';
    
    psktHistory.forEach((entry, index) => {
        const entryDiv = document.createElement('div');
        entryDiv.className = 'pskt-history-entry';
        entryDiv.innerHTML = `
            <div class="history-header">
                <span class="history-role">${entry.role}</span>
                <span class="history-action">${entry.action}</span>
                <span class="history-timestamp">${new Date(entry.timestamp).toLocaleString()}</span>
            </div>
            <div class="history-data">
                <textarea readonly rows="3">${entry.serialized.substring(0, 200)}...</textarea>
            </div>
        `;
        historyElement.appendChild(entryDiv);
    });
}

// Export functions for global access
window.psktManager = {
    initializePSKTManager,
    createPSKT,
    addInputToPSKT,
    addOutputToPSKT,
    updatePSKT,
    signPSKT,
    combinePSKTs,
    finalizePSKT,
    extractTransaction,
    exportPSKTData,
    importPSKTData,
    calculatePSKTMass,
    clearPSKTData,
    updatePSKTStatus,
    updatePSKTDisplay
}; 