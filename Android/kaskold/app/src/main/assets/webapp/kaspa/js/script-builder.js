// Script Builder and Signing Module
// Global variables for script state
let currentScript = null;
let currentP2SHScript = null;
let currentScriptAddress = null;
let scriptHistory = [];

// Initialize Script Builder
async function initializeScriptBuilder() {
    console.log('Script Builder initialized');
    updateScriptDisplay();
}

// Create a new ScriptBuilder
function createScriptBuilder() {
    try {
        if (!kaspaWasm) {
            throw new Error('Kaspa WASM not loaded');
        }

        updateScriptStatus('Creating new ScriptBuilder...', 'info');

        const { ScriptBuilder } = kaspaWasm;
        
        if (!ScriptBuilder) {
            throw new Error('ScriptBuilder class not available in Kaspa SDK');
        }

        currentScript = new ScriptBuilder();
        
        // Add to history
        scriptHistory.push({
            action: 'Created ScriptBuilder',
            timestamp: new Date().toISOString(),
            script: 'Empty ScriptBuilder created'
        });

        updateScriptStatus('ScriptBuilder created successfully', 'success');
        updateScriptDisplay();
        
        return currentScript;
    } catch (error) {
        console.error('Error creating ScriptBuilder:', error);
        updateScriptStatus(`Error creating ScriptBuilder: ${error.message}`, 'error');
        throw error;
    }
}

// Add opcode to script
function addOpcode(opcodeName) {
    try {
        if (!currentScript) {
            throw new Error('No ScriptBuilder available. Create one first.');
        }

        if (!kaspaWasm.Opcodes || !kaspaWasm.Opcodes[opcodeName]) {
            throw new Error(`Opcode ${opcodeName} not available`);
        }

        updateScriptStatus(`Adding opcode: ${opcodeName}...`, 'info');

        const opcode = kaspaWasm.Opcodes[opcodeName];
        currentScript = currentScript.addOp(opcode);

        // Add to history
        scriptHistory.push({
            action: `Added Opcode: ${opcodeName}`,
            timestamp: new Date().toISOString(),
            script: `Added opcode ${opcodeName}`
        });

        updateScriptStatus(`Opcode ${opcodeName} added successfully`, 'success');
        updateScriptDisplay();
        
        return currentScript;
    } catch (error) {
        console.error('Error adding opcode:', error);
        updateScriptStatus(`Error adding opcode: ${error.message}`, 'error');
        throw error;
    }
}

// Add data to script
function addData(data) {
    try {
        if (!currentScript) {
            throw new Error('No ScriptBuilder available. Create one first.');
        }

        updateScriptStatus('Adding data to script...', 'info');

        // Convert hex string to Uint8Array if needed
        let dataBytes;
        if (typeof data === 'string') {
            if (data.startsWith('0x')) {
                data = data.slice(2);
            }
            // Convert hex string to bytes
            dataBytes = new Uint8Array(data.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
        } else {
            dataBytes = data;
        }

        currentScript = currentScript.addData(dataBytes);

        // Add to history
        scriptHistory.push({
            action: 'Added Data',
            timestamp: new Date().toISOString(),
            script: `Added data: ${data}`
        });

        updateScriptStatus('Data added to script successfully', 'success');
        updateScriptDisplay();
        
        return currentScript;
    } catch (error) {
        console.error('Error adding data:', error);
        updateScriptStatus(`Error adding data: ${error.message}`, 'error');
        throw error;
    }
}

// Create Pay-to-Script-Hash (P2SH) script
function createP2SHScript(networkType = 'mainnet') {
    try {
        if (!currentScript) {
            throw new Error('No ScriptBuilder available. Create one first.');
        }

        updateScriptStatus('Creating P2SH script...', 'info');

        const { addressFromScriptPublicKey } = kaspaWasm;
        
        // Create P2SH script
        currentP2SHScript = currentScript.createPayToScriptHashScript();
        
        // Generate address from P2SH script
        const networkTypeEnum = getNetworkTypeEnum(networkType);
        currentScriptAddress = addressFromScriptPublicKey(currentP2SHScript, networkTypeEnum);

        // Add to history
        scriptHistory.push({
            action: 'Created P2SH Script',
            timestamp: new Date().toISOString(),
            script: `P2SH script created for network: ${networkType}`,
            address: currentScriptAddress ? currentScriptAddress.toString() : 'Unknown'
        });

        updateScriptStatus('P2SH script created successfully', 'success');
        updateScriptDisplay();
        
        return {
            p2shScript: currentP2SHScript,
            address: currentScriptAddress
        };
    } catch (error) {
        console.error('Error creating P2SH script:', error);
        updateScriptStatus(`Error creating P2SH script: ${error.message}`, 'error');
        throw error;
    }
}

// Create signature script for P2SH
function createSignatureScript(signature = "") {
    try {
        if (!currentScript) {
            throw new Error('No ScriptBuilder available. Create one first.');
        }

        updateScriptStatus('Creating signature script...', 'info');

        const signatureScript = currentScript.encodePayToScriptHashSignatureScript(signature);

        // Add to history
        scriptHistory.push({
            action: 'Created Signature Script',
            timestamp: new Date().toISOString(),
            script: `Signature script created with signature: ${signature || '(empty)'}`
        });

        updateScriptStatus('Signature script created successfully', 'success');
        updateScriptDisplay();
        
        return signatureScript;
    } catch (error) {
        console.error('Error creating signature script:', error);
        updateScriptStatus(`Error creating signature script: ${error.message}`, 'error');
        throw error;
    }
}

// Sign script hash
async function signScriptHash(scriptHash, privateKeyHex) {
    try {
        if (!scriptHash) {
            throw new Error('Script hash is required');
        }

        if (!privateKeyHex) {
            throw new Error('Private key is required');
        }

        updateScriptStatus('Signing script hash...', 'info');

        const { signScriptHash, PrivateKey } = kaspaWasm;
        
        if (!signScriptHash) {
            throw new Error('signScriptHash function not available in Kaspa SDK');
        }

        // Create private key object
        const privateKey = new PrivateKey(privateKeyHex);
        
        // Sign the script hash
        const signature = signScriptHash(scriptHash, privateKey);

        // Add to history
        scriptHistory.push({
            action: 'Signed Script Hash',
            timestamp: new Date().toISOString(),
            script: `Signed script hash: ${scriptHash}`,
            signature: signature
        });

        updateScriptStatus('Script hash signed successfully', 'success');
        updateScriptDisplay();
        
        return signature;
    } catch (error) {
        console.error('Error signing script hash:', error);
        updateScriptStatus(`Error signing script hash: ${error.message}`, 'error');
        throw error;
    }
}

// Get available opcodes
function getAvailableOpcodes() {
    try {
        if (!kaspaWasm || !kaspaWasm.Opcodes) {
            return [];
        }
        return Object.keys(kaspaWasm.Opcodes);
    } catch (error) {
        console.error('Error getting opcodes:', error);
        return [];
    }
}

// Get network type enum
function getNetworkTypeEnum(networkType) {
    const { NetworkType } = kaspaWasm;
    
    switch (networkType.toLowerCase()) {
        case 'mainnet':
            return NetworkType.Mainnet;
        case 'testnet':
        case 'testnet-10':
            return NetworkType.Testnet;
        case 'testnet-11':
            return NetworkType.Testnet11;
        case 'devnet':
            return NetworkType.Devnet;
        case 'simnet':
            return NetworkType.Simnet;
        default:
            return NetworkType.Mainnet;
    }
}

// Export script data
function exportScriptData() {
    try {
        const scriptData = {
            version: '1.0',
            type: 'kaspa-script',
            hasScript: !!currentScript,
            hasP2SH: !!currentP2SHScript,
            address: currentScriptAddress ? currentScriptAddress.toString() : null,
            timestamp: new Date().toISOString(),
            history: scriptHistory
        };

        const dataStr = JSON.stringify(scriptData, null, 2);
        const dataBlob = new Blob([dataStr], { type: 'application/json' });
        const url = URL.createObjectURL(dataBlob);
        
        const link = document.createElement('a');
        link.href = url;
        link.download = `kaspa-script-${Date.now()}.json`;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);

        updateScriptStatus('Script data exported successfully', 'success');
        
        return scriptData;
    } catch (error) {
        console.error('Error exporting script data:', error);
        updateScriptStatus(`Error exporting script data: ${error.message}`, 'error');
        throw error;
    }
}

// Import script data
function importScriptData(fileContent) {
    try {
        const scriptData = JSON.parse(fileContent);
        
        if (scriptData.type !== 'kaspa-script') {
            throw new Error('Invalid script file format');
        }

        // Restore history if available
        if (scriptData.history) {
            scriptHistory = scriptData.history;
        }

        // Note: Cannot restore actual ScriptBuilder objects from JSON
        // User will need to recreate the script
        currentScript = null;
        currentP2SHScript = null;
        currentScriptAddress = null;

        updateScriptStatus('Script data imported successfully (history only)', 'success');
        updateScriptDisplay();
        
        return scriptData;
    } catch (error) {
        console.error('Error importing script data:', error);
        updateScriptStatus(`Error importing script data: ${error.message}`, 'error');
        throw error;
    }
}

// Clear script data
function clearScriptData() {
    currentScript = null;
    currentP2SHScript = null;
    currentScriptAddress = null;
    scriptHistory = [];
    updateScriptDisplay();
    updateScriptStatus('Script data cleared', 'info');
}

// Update script status display
function updateScriptStatus(message, type = 'info') {
    const statusElement = document.getElementById('script-status');
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
    console.log(`Script Status [${type}]: ${message}`);
}

// Update script display
function updateScriptDisplay() {
    // Update current script info
    const scriptStatusElement = document.getElementById('current-script-status');
    const addressElement = document.getElementById('current-script-address');
    
    if (scriptStatusElement) {
        scriptStatusElement.textContent = currentScript ? 'Active' : 'None';
    }
    
    if (addressElement) {
        addressElement.textContent = currentScriptAddress ? currentScriptAddress.toString() : 'No P2SH address';
    }
    
    // Update history display
    updateScriptHistoryDisplay();
    
    // Update opcodes dropdown
    updateOpcodesDropdown();
}

// Update script history display
function updateScriptHistoryDisplay() {
    const historyElement = document.getElementById('script-history-list');
    if (!historyElement) return;
    
    historyElement.innerHTML = '';
    
    if (scriptHistory.length === 0) {
        historyElement.innerHTML = '<p>No script history available.</p>';
        return;
    }
    
    scriptHistory.forEach((entry, index) => {
        const entryDiv = document.createElement('div');
        entryDiv.className = 'script-history-entry';
        entryDiv.innerHTML = `
            <div class="history-header">
                <span class="history-action">${entry.action}</span>
                <span class="history-timestamp">${new Date(entry.timestamp).toLocaleString()}</span>
            </div>
            <div class="history-data">
                <div>${entry.script}</div>
                ${entry.address ? `<div><strong>Address:</strong> ${entry.address}</div>` : ''}
                ${entry.signature ? `<div><strong>Signature:</strong> ${entry.signature.substring(0, 50)}...</div>` : ''}
            </div>
        `;
        historyElement.appendChild(entryDiv);
    });
}

// Update opcodes dropdown
function updateOpcodesDropdown() {
    const dropdown = document.getElementById('opcode-select');
    if (!dropdown) return;
    
    const opcodes = getAvailableOpcodes();
    dropdown.innerHTML = '<option value="">Select an opcode...</option>';
    
    opcodes.forEach(opcode => {
        const option = document.createElement('option');
        option.value = opcode;
        option.textContent = opcode;
        dropdown.appendChild(option);
    });
}

// Export functions for global access
window.scriptBuilder = {
    initializeScriptBuilder,
    createScriptBuilder,
    addOpcode,
    addData,
    createP2SHScript,
    createSignatureScript,
    signScriptHash,
    getAvailableOpcodes,
    exportScriptData,
    importScriptData,
    clearScriptData,
    updateScriptStatus,
    updateScriptDisplay
}; 