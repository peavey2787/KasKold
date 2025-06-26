// Kaspa WASM Initialization Module
// Use relative path to kaspa-wasm32-sdk from kaspa/js/ directory
import * as kaspa from '../../kaspa-wasm32-sdk/web/kaspa/kaspa.js';
import { NETWORK_TYPES, DEFAULT_ADDRESS_PATH, generateWallet } from './wallet-generator.js';
import { calculateAccurateTransactionFee } from './fee-calculator.js';
import { createTransaction, createTransactionWithManualFee, exportTransaction } from './transaction-create.js';
import { signTransaction, exportSignedTransaction } from './transaction-sign.js';
import { submitTransaction, getTransactionStatus } from './transaction-submit.js';
// Balance checking now handled by unified wallet manager
import { 
    restoreWalletFromMnemonic, 
    restoreWalletFromPrivateKey, 
    validateMnemonic, 
    validatePrivateKey, 
    getWalletInfo 
} from './wallet-restore.js';
import {
    signMessage,
    verifyMessage,
    exportMessageData,
    validateImportedMessageData
} from './message-signing.js';
import {
    generateUnsignedMessageQR,
    generateSignedMessageQR,
    readQRFromImage,
    downloadQRImage,
    validateQRData,
    createQRDisplay,
    initializeQRManager,
    generateUnsignedTransactionQR,
    generateSignedTransactionQR,
    generateSubmittedTransactionQR,
    validateTransactionQRData,
    createMultiPartQRDisplay,
    readMultiPartQRFromImages,
    openCameraQRScanner
} from './qr-manager.js';
// Wallet management now handled by unified wallet manager

let kaspaInitialized = false;
let currentTransactionData = null;
let currentSignedTransactionData = null;
let currentSubmittedTransactionData = null;
let currentWallet = null; // Store the current active wallet
let calculatedFees = null; // Store calculated fee options
let currentMessageData = null; // Store current unsigned message data
let currentSignedMessageData = null; // Store current signed message data
let currentVerificationData = null; // Store current verification result data
let currentUnsignedTxQR = null; // Store current unsigned transaction QR
let currentSignedTxQR = null; // Store current signed transaction QR
let currentSubmittedTxQR = null; // Store current submitted transaction QR

// Initialize Kaspa WASM once
async function initKaspa() {
    if (!kaspaInitialized) {
        await kaspa.default(new URL('../../kaspa-wasm32-sdk/web/kaspa/kaspa_bg.wasm', import.meta.url));
        kaspaInitialized = true;
        
        // Make kaspa available globally for other modules
        window.kaspaWasm = kaspa;
        window.getKaspa = getKaspa;
        
        // Make global functions available
        window.saveWalletWithPassword = saveWalletWithPassword;
        

    }
    return kaspa;
}

// Get Kaspa instance (must be called after initKaspa)
function getKaspa() {
    if (!kaspaInitialized) {
        throw new Error('Kaspa WASM not initialized. Call initKaspa() first.');
    }
    return kaspa;
}

// Check if initialized
function isInitialized() {
    return kaspaInitialized;
}

// Set current wallet and update UI
function setCurrentWallet(walletData) {
    currentWallet = {
        address: walletData.publicAddress || walletData.address,
        privateKey: walletData.privateKey,
        mnemonic: walletData.mnemonic || null,
        networkType: walletData.networkType,
        derivationPath: walletData.derivationPath || null,
        balance: null // Will be loaded when checking balance
    };
    
    updateCurrentWalletDisplay();
    updateBalanceWalletDisplay();
}

// Update the current wallet display in the UI
function updateCurrentWalletDisplay() {
    if (currentWallet) {
        // Transaction section wallet display
        document.getElementById('current-wallet-info').style.display = 'block';
        document.getElementById('current-wallet-address').textContent = currentWallet.address;
        document.getElementById('current-wallet-network').textContent = currentWallet.networkType;
        document.getElementById('current-wallet-balance').textContent = currentWallet.balance || 'Click "Check Balance" to load';
        
        // Message signing section wallet display
        const msgWalletInfo = document.getElementById('current-wallet-info-msg');
        if (msgWalletInfo) {
            msgWalletInfo.style.display = 'block';
            document.getElementById('current-wallet-address-msg').textContent = currentWallet.address;
            document.getElementById('current-wallet-network-msg').textContent = currentWallet.networkType;
        }
    } else {
        document.getElementById('current-wallet-info').style.display = 'none';
        const msgWalletInfo = document.getElementById('current-wallet-info-msg');
        if (msgWalletInfo) {
            msgWalletInfo.style.display = 'none';
        }
    }
}

// Get current wallet
function getCurrentWallet() {
    return currentWallet;
}

// Utility function to download data as file
function downloadFile(data, filename, mimeType = 'application/json') {
    const blob = new Blob([data], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
}

// Setup all event handlers
function setupAllEventHandlers() {
    
    setupWalletEventHandlers();
    setupWalletManagerEventHandlers();
    setupWalletRestoreEventHandlers();
    setupBalanceEventHandlers();
    setupTransactionEventHandlers(); // Called during initialization when all components are loaded
    setupMessageSigningEventHandlers();
    setupPSKTEventHandlers();
    setupScriptBuilderEventHandlers();
    
    // Initialize QR Manager
    initializeQRManager();
}

// Wallet Manager event handlers
function setupWalletManagerEventHandlers() {
    // Set up wallet manager event listeners
    walletManager.addWalletChangeListener((event, data) => {
        switch (event) {
            case 'wallet_login':
                // Update the current wallet when user logs in
                setCurrentWallet({
                    publicAddress: data.address,
                    privateKey: data.privateKey,
                    mnemonic: data.mnemonic,
                    networkType: data.network,
                    derivationPath: data.derivationPath
                });

                break;
                
            case 'wallet_logout':
                // Clear current wallet when user logs out
                currentWallet = null;
                updateCurrentWalletDisplay();
                updateBalanceWalletDisplay();

                break;
                
            case 'wallet_saved':

                break;
                
            case 'wallet_deleted':

                break;
        }
    });
}

// PSKT event handlers
function setupPSKTEventHandlers() {
    // Load PSKT manager script
    if (!window.psktManager) {
        const script = document.createElement('script');
        script.src = './kaspa/js/pskt-manager.js';
        script.onload = function() {
            if (window.psktManager && typeof window.psktManager.initializePSKTManager === 'function') {
                window.psktManager.initializePSKTManager();
            }
        };
        document.head.appendChild(script);
    } else {
        // Already loaded, just initialize
        if (typeof window.psktManager.initializePSKTManager === 'function') {
            window.psktManager.initializePSKTManager();
        }
    }
}

// Script Builder event handlers
function setupScriptBuilderEventHandlers() {
    // Load Script Builder script
    if (!window.scriptBuilder) {
        const script = document.createElement('script');
        script.src = './kaspa/js/script-builder.js';
        script.onload = function() {
            if (window.scriptBuilder && typeof window.scriptBuilder.initializeScriptBuilder === 'function') {
                window.scriptBuilder.initializeScriptBuilder();
            }
        };
        document.head.appendChild(script);
    } else {
        // Already loaded, just initialize
        if (typeof window.scriptBuilder.initializeScriptBuilder === 'function') {
            window.scriptBuilder.initializeScriptBuilder();
        }
    }
}

// Wallet event handlers
function setupWalletEventHandlers() {
    // Derivation path warning handler
    const derivationPathWarning = document.getElementById("derivationPathWarning");
    if (derivationPathWarning) {
        derivationPathWarning.addEventListener("click", () => {
            alert("WARNING: Only change the derivation path if you know what you are doing!\n\nThe derivation path determines how your wallet addresses are generated from your mnemonic. Using an incorrect path may result in:\n- Inability to recover your funds\n- Incompatibility with other wallets\n- Loss of access to previously generated addresses\n\nThe default path (m/44'/111111'/0'/0/0) is the standard for Kaspa wallets.");
        });
    }

    // Reset derivation path to default
    const resetDerivationPath = document.getElementById("resetDerivationPath");
    if (resetDerivationPath) {
        resetDerivationPath.addEventListener("click", () => {
            const derivationPathInput = document.getElementById("derivationPath");
            if (derivationPathInput) {
                derivationPathInput.value = DEFAULT_ADDRESS_PATH;
            }
        });
    }

    // Generate wallet handler
    const generateWalletBtn = document.getElementById("generateWallet");
    if (generateWalletBtn) {
        generateWalletBtn.addEventListener("click", () => {
            try {
                const networkType = document.getElementById("networkType")?.value || 'mainnet';
                const derivationPath = document.getElementById("derivationPath")?.value || DEFAULT_ADDRESS_PATH;
                const passphrase = document.getElementById("passphrase")?.value || null;

                const wallet = generateWallet(networkType, derivationPath, passphrase);

                const mnemonicEl = document.getElementById("mnemonic");
                if (mnemonicEl) mnemonicEl.textContent = wallet.mnemonic;
                
                const publicAddressEl = document.getElementById("publicAddress");
                if (publicAddressEl) publicAddressEl.textContent = wallet.publicAddress;
                
                const privateKeyEl = document.getElementById("privateKey");
                if (privateKeyEl) privateKeyEl.textContent = wallet.privateKey;

                // Show faucet link for testnets
                const testnetFaucetLink = document.getElementById("testnetFaucetLink");
                const faucetLink = document.getElementById("faucetLink");
                
                if (testnetFaucetLink && faucetLink) {
                    if (networkType === 'testnet-10') {
                        testnetFaucetLink.style.display = "block";
                        faucetLink.href = "https://faucet-tn10.kaspanet.io/";
                        faucetLink.textContent = "Get Free Test KAS from Testnet-10 Faucet";
                    } else if (networkType === 'testnet-11') {
                        testnetFaucetLink.style.display = "block";
                        faucetLink.href = "https://faucet-tn11.kaspanet.io/";
                        faucetLink.textContent = "Get Free Test KAS from Testnet-11 Faucet";
                    } else {
                        testnetFaucetLink.style.display = "none";
                    }
                }

                // Set as current wallet
                setCurrentWallet(wallet);

                // Store wallet data for potential saving in wallet manager
                window.currentGeneratedWallet = {
                    privateKey: wallet.privateKey,
                    address: wallet.publicAddress,
                    network: wallet.networkType,
                    mnemonic: wallet.mnemonic,
                    derivationPath: wallet.derivationPath
                };

                // Show save option in wallet manager
                showWalletSavePrompt();

            } catch (error) {
                alert("Error generating wallet: " + error.message);
            }
        });
    }
}

// Wallet restoration event handlers
function setupWalletRestoreEventHandlers() {
    // Tab switching for restoration methods
    window.switchRestoreTab = (tab) => {
        // Remove active class from all tabs and contents
        document.querySelectorAll('.tab-button').forEach(btn => btn.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));
        
        // Add active class to selected tab and content
        if (tab === 'mnemonic') {
            document.querySelector('.tab-button:first-child').classList.add('active');
            document.getElementById('mnemonic-restore-tab').classList.add('active');
        } else if (tab === 'privatekey') {
            document.querySelector('.tab-button:last-child').classList.add('active');
            document.getElementById('privatekey-restore-tab').classList.add('active');
        }
    };

    // Derivation path info and edit handlers for restoration
    window.showDerivationInfo = (context) => {
        alert("DERIVATION PATH INFO\n\nThe derivation path determines how your wallet addresses are generated from your mnemonic seed.\n\nDefault Kaspa path: m/44'/111111'/0'/0/0\n- m/44' = BIP44 standard\n- 111111' = Kaspa coin type\n- 0' = Account 0\n- 0 = External chain\n- 0 = Address index\n\nOnly change if you used a different path when creating the wallet!");
    };

    window.enableDerivationEdit = (context) => {
        const pathInput = document.getElementById(`${context}-derivation-path`);
        pathInput.readOnly = false;
        pathInput.style.backgroundColor = '#fff3cd';
        
        const warning = confirm("WARNING: Changing the derivation path can result in generating different addresses!\n\nOnly proceed if you know the exact derivation path used when the wallet was originally created.\n\nDo you want to continue?");
        
        if (!warning) {
            pathInput.readOnly = true;
            pathInput.style.backgroundColor = '';
        }
    };

    window.resetDerivationPath = (context) => {
        const pathInput = document.getElementById(`${context}-derivation-path`);
        pathInput.value = DEFAULT_ADDRESS_PATH;
        pathInput.readOnly = true;
        pathInput.style.backgroundColor = '';
    };

    // Mnemonic validation
    window.validateMnemonicInput = async () => {
        try {
            const mnemonicPhrase = document.getElementById('mnemonic-phrase').value;
            const statusDiv = document.getElementById('restore-status');
            
            if (!mnemonicPhrase.trim()) {
                showRestoreStatus('Please enter a mnemonic phrase', 'error');
                return;
            }

            const validation = validateMnemonic(mnemonicPhrase);
            
            if (validation.isValid) {
                showRestoreStatus(`Valid mnemonic phrase with ${validation.wordCount} words`, 'success');
            } else {
                showRestoreStatus(validation.error, 'error');
            }

        } catch (error) {
            showRestoreStatus(`Validation error: ${error.message}`, 'error');
        }
    };

    // Restore from mnemonic
    window.restoreFromMnemonic = async () => {
        try {
            const mnemonicPhrase = document.getElementById('mnemonic-phrase').value;
            const passphrase = document.getElementById('mnemonic-passphrase').value || null;
            const networkType = document.getElementById('networkType').value; // Use global network selector
            const derivationPath = document.getElementById('restore-derivation-path').value;

            if (!mnemonicPhrase.trim()) {
                showRestoreStatus('Please enter a mnemonic phrase', 'error');
                return;
            }

            showRestoreStatus('Restoring wallet from mnemonic...', 'info');

            const result = restoreWalletFromMnemonic(mnemonicPhrase, networkType, derivationPath, passphrase);
            
            if (result.success) {
                displayRestoredWallet(result);
                showRestoreStatus('Wallet restored successfully from mnemonic!', 'success');
            } else {
                showRestoreStatus(result.error, 'error');
            }

        } catch (error) {
            showRestoreStatus(`Restoration error: ${error.message}`, 'error');
        }
    };

    // Restore from private key
    window.restoreFromPrivateKey = async () => {
        try {
            const privateKeyString = document.getElementById('private-key-input').value;
            const networkType = document.getElementById('networkType').value; // Use global network selector

            if (!privateKeyString.trim()) {
                showRestoreStatus('Please enter a private key', 'error');
                return;
            }

            showRestoreStatus('Importing wallet from private key...', 'info');

            const result = restoreWalletFromPrivateKey(privateKeyString, networkType);
            
            if (result.success) {
                displayRestoredWallet(result);
                showRestoreStatus('Wallet imported successfully from private key!', 'success');
            } else {
                showRestoreStatus(result.error, 'error');
            }

        } catch (error) {
            showRestoreStatus(`Import error: ${error.message}`, 'error');
        }
    };

    // Display restored wallet results
    function displayRestoredWallet(restorationResult) {
        const walletInfo = getWalletInfo(restorationResult);
        
        // Show results container
        document.getElementById('restore-results').style.display = 'block';
        
        // Fill in the results
        document.getElementById('restored-from-method').textContent = 
            restorationResult.restoredFrom === 'mnemonic' ? 'Mnemonic Phrase' : 'Private Key';
        document.getElementById('restored-network').textContent = restorationResult.networkType;
        document.getElementById('restored-address').value = walletInfo.address;
        document.getElementById('restored-private-key').value = walletInfo.privateKey;
        
        // Show/hide mnemonic info based on restoration method
        if (restorationResult.restoredFrom === 'mnemonic' && walletInfo.mnemonic) {
            document.getElementById('restored-mnemonic-info').style.display = 'block';
            document.getElementById('restored-mnemonic').value = walletInfo.mnemonic;
        } else {
            document.getElementById('restored-mnemonic-info').style.display = 'none';
        }
        
        // Show/hide derivation path info
        if (walletInfo.derivationPath) {
            document.getElementById('restored-derivation-info').style.display = 'block';
            document.getElementById('restored-derivation-path').textContent = walletInfo.derivationPath;
        } else {
            document.getElementById('restored-derivation-info').style.display = 'none';
        }

        // Show faucet link for testnets
        const restoredTestnetFaucetLink = document.getElementById("restoredTestnetFaucetLink");
        const restoredFaucetLink = document.getElementById("restoredFaucetLink");
        
        if (restorationResult.networkType === 'testnet-10') {
            restoredTestnetFaucetLink.style.display = "block";
            restoredFaucetLink.href = "https://faucet-tn10.kaspanet.io/";
            restoredFaucetLink.textContent = "Get Free Test KAS from Testnet-10 Faucet";
        } else if (restorationResult.networkType === 'testnet-11') {
            restoredTestnetFaucetLink.style.display = "block";
            restoredFaucetLink.href = "https://faucet-tn11.kaspanet.io/";
            restoredFaucetLink.textContent = "Get Free Test KAS from Testnet-11 Faucet";
        } else {
            restoredTestnetFaucetLink.style.display = "none";
        }

        // Set as current wallet
        setCurrentWallet(restorationResult);

        // Store wallet data for potential saving in wallet manager
        const restoredWalletInfo = getWalletInfo(restorationResult);
        window.currentGeneratedWallet = {
            privateKey: restoredWalletInfo.privateKey,
            address: restoredWalletInfo.address,
            network: restoredWalletInfo.networkType,
            mnemonic: restoredWalletInfo.mnemonic,
            derivationPath: restoredWalletInfo.derivationPath
        };

        // Show save option in wallet manager
        showWalletSavePrompt();
    }

    // Clear restoration results
    window.clearRestoreResults = () => {
        document.getElementById('restore-results').style.display = 'none';
        document.getElementById('restore-status').className = 'status-container';
        document.getElementById('restore-status').textContent = '';
        
        // Clear form inputs
        document.getElementById('mnemonic-phrase').value = '';
        document.getElementById('mnemonic-passphrase').value = '';
        document.getElementById('private-key-input').value = '';
        
        // Reset derivation paths
        document.getElementById('restore-derivation-path').value = DEFAULT_ADDRESS_PATH;
        document.getElementById('restore-derivation-path').readOnly = true;
        document.getElementById('restore-derivation-path').style.backgroundColor = '';
        
        // Clear current wallet
        currentWallet = null;
        updateCurrentWalletDisplay();
    };

    // Download wallet info for restored wallet
    window.downloadWalletInfo = (context) => {
        const address = document.getElementById('restored-address').value;
        const privateKey = document.getElementById('restored-private-key').value;
        const mnemonic = document.getElementById('restored-mnemonic').value || null;
        const network = document.getElementById('restored-network').textContent;
        const derivationPath = document.getElementById('restored-derivation-path').textContent || null;
        
        const walletData = {
            type: 'kaspa-restored-wallet',
            timestamp: new Date().toISOString(),
            network: network,
            address: address,
            privateKey: privateKey,
            ...(mnemonic && { mnemonic: mnemonic }),
            ...(derivationPath && { derivationPath: derivationPath }),
            warning: 'Keep this file secure and private. Never share your private key or mnemonic with anyone.'
        };
        
        const filename = `kaspa-restored-wallet-${address.substring(0, 8)}-${Date.now()}.json`;
        downloadFile(JSON.stringify(walletData, null, 2), filename);
    };

    // Show restoration status messages
    function showRestoreStatus(message, type) {
        const statusDiv = document.getElementById('restore-status');
        statusDiv.textContent = message;
        statusDiv.className = `status-container ${type}`;
    }
}

// Balance section event handlers
function setupBalanceEventHandlers() {
    // Check balance handler
    const checkBalanceBtn = document.getElementById("checkBalance");
    if (checkBalanceBtn) {
        checkBalanceBtn.addEventListener("click", async () => {
        if (!currentWallet) {
            showBalanceStatus('No wallet selected. Please generate or restore a wallet first.', 'error');
            return;
        }

        try {
            showBalanceStatus('Checking balance...', 'info');
            document.getElementById("availableBalance").textContent = "Checking...";
            
            const balanceResult = await checkAddressBalance(currentWallet.address, currentWallet.networkType);
            
            if (balanceResult.success) {
                const formattedBalance = formatBalance(balanceResult);
                document.getElementById("availableBalance").textContent = formattedBalance;
                currentWallet.balance = formattedBalance;
                updateCurrentWalletDisplay();
                updateBalanceWalletDisplay();
                
                // Show explorer link with network-specific URL
                const explorerLink = document.getElementById("explorerLink");
                const kasExplorerLink = document.getElementById("kasExplorerLink");
                explorerLink.style.display = "block";
                
                // Determine explorer URL based on network type
                let explorerUrl;
                let explorerName;
                
                switch (currentWallet.networkType) {
                    case 'mainnet':
                        explorerUrl = `https://kas.fyi/address/${currentWallet.address}`;
                        explorerName = 'kas.fyi Explorer';
                        break;
                    case 'testnet-10':
                        explorerUrl = `https://explorer-tn10.kaspa.org/addresses/${currentWallet.address}?page=1`;
                        explorerName = 'Testnet-10 Explorer';
                        break;
                    case 'testnet-11':
                        explorerUrl = `https://explorer-tn11.kaspa.org/addresses/${currentWallet.address}?page=1`;
                        explorerName = 'Testnet-11 Explorer';
                        break;
                    default:
                        explorerUrl = `https://kas.fyi/address/${currentWallet.address}`;
                        explorerName = 'Blockchain Explorer';
                        break;
                }
                
                kasExplorerLink.href = explorerUrl;
                kasExplorerLink.textContent = `📊 View on ${explorerName}`;
                
                showBalanceStatus(`Balance loaded: ${formattedBalance}`, 'success');
            } else {
                document.getElementById("availableBalance").textContent = `Error: ${balanceResult.error}`;
                showBalanceStatus(`Balance check failed: ${balanceResult.error}`, 'error');
            }

        } catch (error) {
            document.getElementById("availableBalance").textContent = "Check failed";
            showBalanceStatus(`Balance check failed: ${error.message}`, 'error');
        }
        });
    }
}

// Update balance wallet display
function updateBalanceWalletDisplay() {
    const balanceWalletInfo = document.getElementById('balance-wallet-info');
    const checkBalanceButton = document.getElementById('checkBalance');
    
    if (currentWallet) {
        balanceWalletInfo.style.display = 'block';
        document.getElementById('balance-wallet-address').textContent = currentWallet.address;
        document.getElementById('balance-wallet-network').textContent = currentWallet.networkType;
        checkBalanceButton.disabled = false;
        document.getElementById("availableBalance").textContent = "Click 'Check Balance' to load";
    } else {
        balanceWalletInfo.style.display = 'none';
        checkBalanceButton.disabled = true;
        document.getElementById("availableBalance").textContent = "Select a wallet first";
    }
}



// Show balance status messages
function showBalanceStatus(message, type) {
    const statusDiv = document.getElementById('balance-status');
    statusDiv.textContent = message;
    statusDiv.className = `status-container ${type}`;
}

// Transaction event handlers
function setupTransactionEventHandlers() {
    
    // Check if we're in the right context (transaction page loaded)
    const amountElement = document.getElementById("amount");
    const toAddressElement = document.getElementById("toAddress");
    const createTransactionBtn = document.getElementById("createTransaction");
    
    if (!amountElement || !toAddressElement || !createTransactionBtn) {
        return;
    }
    
    // Store current mode state
    let isOfflineMode = false;

    // Amount input validation
    document.getElementById("amount").addEventListener("input", () => {
        validateAmountInput();
    });

    // Prevent entering amounts below minimum on blur
    document.getElementById("amount").addEventListener("blur", () => {
        const amountInput = document.getElementById("amount");
        const amountValue = parseFloat(amountInput.value) || 0;
        const MINIMUM_AMOUNT = 0.2;
        
        if (amountValue > 0 && amountValue < MINIMUM_AMOUNT) {
            // Auto-correct to minimum amount
            amountInput.value = MINIMUM_AMOUNT.toString();
            validateAmountInput(); // Re-validate after correction
        }
    });

    // Amount validation function
    function validateAmountInput() {
        const amountInput = document.getElementById("amount");
        const amountValue = parseFloat(amountInput.value) || 0;
        const MINIMUM_AMOUNT = 0.2;
        
        // Clear any existing amount warnings
        let warningElement = document.getElementById("amountWarning");
        if (!warningElement) {
            // Create warning element if it doesn't exist
            warningElement = document.createElement("div");
            warningElement.id = "amountWarning";
            warningElement.style.cssText = "color: #dc3545; font-size: 0.85em; font-weight: bold; margin-top: 5px; display: none;";
            amountInput.parentNode.appendChild(warningElement);
        }
        
        if (amountValue > 0 && amountValue < MINIMUM_AMOUNT) {
            // Show warning for amounts below minimum
            warningElement.textContent = `⚠️ Minimum amount is ${MINIMUM_AMOUNT} KAS (anti-dust protection)`;
            warningElement.style.display = "block";
            amountInput.style.borderColor = "#dc3545";
        } else {
            // Hide warning for valid amounts
            warningElement.style.display = "none";
            amountInput.style.borderColor = "";
        }
    }

    // Manual fee input handler for real-time total calculation
    const manualFeeInput = document.getElementById("manualFee");
    if (manualFeeInput) {
        manualFeeInput.addEventListener("input", () => {
        if (isOfflineMode) {
            const amount = parseFloat(document.getElementById("amount").value) || 0;
            const fee = parseFloat(document.getElementById("manualFee").value) || 0;
            
            if (amount > 0 && fee >= 0) {
                document.getElementById("estimatedFee").textContent = `${fee.toFixed(8)} KAS (manual)`;
                document.getElementById("totalCost").textContent = `${(amount + fee).toFixed(8)} KAS`;
                
                // Enable create transaction if we have valid inputs
                document.getElementById("createTransaction").disabled = false;
            } else {
                document.getElementById("estimatedFee").textContent = "Enter manual fee";
                document.getElementById("totalCost").textContent = "—";
                document.getElementById("createTransaction").disabled = true;
            }
        }
        });
    }

    // Calculate fee handler - tries automatic first, falls back to manual on failure
    const calculateFeeBtn = document.getElementById("calculateFee");
    if (calculateFeeBtn) {
        calculateFeeBtn.addEventListener("click", async () => {
        try {
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }
            
            // Reset QR buttons and clear previous QR data when starting new transaction
            resetTransactionQRButtons();

            const toAddress = document.getElementById("toAddress").value;
            const amount = document.getElementById("amount").value;

            if (!toAddress || !amount) {
                alert("Please fill in To Address and Amount");
                return;
            }
            
            // Validate amount before attempting fee calculation
            const amountValue = parseFloat(amount);
            const MINIMUM_AMOUNT = 0.2;
            if (amountValue < MINIMUM_AMOUNT) {
                document.getElementById("estimatedFee").textContent = "Amount too low";
                document.getElementById("totalCost").textContent = "—";
                document.getElementById("transactionError").textContent = `⚠️ Amount too low: minimum transaction amount is ${MINIMUM_AMOUNT} KAS (anti-dust protection). Try increasing the amount.`;
                return;
            }

            // Always try automatic fee calculation first
            document.getElementById("estimatedFee").textContent = "Calculating...";
            document.getElementById("totalCost").textContent = "Calculating...";
            document.getElementById("transactionError").textContent = "—";
            
            // Hide manual fee input initially
            document.getElementById("manualFeeInput").style.display = 'none';
            document.getElementById("feeOptions").style.display = 'none';
            isOfflineMode = false;
            
            // Get current network type
            const networkType = currentWallet.networkType;
            
            // Debug logging to see what's in currentWallet

            
            try {
                // Use the new accurate fee calculation
                const feeResult = await calculateAccurateTransactionFee(currentWallet.address, toAddress, amount, networkType, 'normal');
            
            if (feeResult.success) {
                // Online mode - fee calculation succeeded
                calculatedFees = feeResult.fee;
                
                // Show fee options in radio buttons
                document.getElementById("slowFeeDisplay").textContent = `${feeResult.fee.slow.toFixed(8)} KAS`;
                document.getElementById("normalFeeDisplay").textContent = `${feeResult.fee.normal.toFixed(8)} KAS`;
                document.getElementById("fastFeeDisplay").textContent = `${feeResult.fee.fast.toFixed(8)} KAS`;
                
                // Show the fee options section
                document.getElementById("feeOptions").style.display = 'block';
                
                // Update main display with default (normal) selection
                updateFeeDisplay();
                
                // Enable create transaction button
                document.getElementById("createTransaction").disabled = false;
                
                        } else {
                // Check if this is specifically a "No UTXOs" error (not a network issue)
                if (feeResult.isNoUTXOsError) {
                    // This is a valid response - address has no balance, stay in online mode

                    
                    // Provide helpful message for new wallets
                    document.getElementById("estimatedFee").textContent = "No balance available";
                    document.getElementById("totalCost").textContent = "—";
                    document.getElementById("transactionError").textContent = "💰 This wallet has no balance. To send transactions, you need to receive KAS first. Generated wallets start with zero balance.";
                    return; // Don't throw error, just return early
                }
                
                // Check if this is a low amount error
                if (feeResult.isLowAmountError || feeResult.isInvalidAmountError) {
                    // This is a validation error - stay in online mode

                    throw new Error(feeResult.error);
                }
                
                // Check if this is a network connectivity issue or other error
                const errorMessage = feeResult.error || '';
                const isNetworkError = errorMessage.includes('timeout') || 
                                     errorMessage.includes('connection') || 
                                     errorMessage.includes('network') ||
                                     errorMessage.includes('RPC connection') ||
                                     errorMessage.includes('fetch') ||
                                     errorMessage.includes('ENOTFOUND') ||
                                     errorMessage.includes('ECONNREFUSED');
                
                if (isNetworkError) {
                    // Network/connectivity issue - switch to offline mode

                    
                    isOfflineMode = true;
                    
                    // Show manual fee input
                    document.getElementById("manualFeeInput").style.display = 'block';
                    document.getElementById("feeOptions").style.display = 'none';
                    
                    // Update UI for offline mode
                    document.getElementById("estimatedFee").textContent = "Enter manual fee below";
                    document.getElementById("totalCost").textContent = "—";
                    document.getElementById("transactionError").textContent = `Network unavailable: ${errorMessage}`;
                    
                    // Focus on manual fee input
                    document.getElementById("manualFee").focus();
                    
                    // Disable create transaction until manual fee is entered
                    document.getElementById("createTransaction").disabled = true;
                } else {
                    // Other error (validation, insufficient funds, etc.) - stay in online mode
                    throw new Error(feeResult.error);
                }
            }
            
            } catch (jsError) {
                // JavaScript exception during fee calculation - check if it's network-related
                const errorMessage = jsError.message || '';
                const isNetworkError = errorMessage.includes('timeout') || 
                                     errorMessage.includes('connection') || 
                                     errorMessage.includes('network') ||
                                     errorMessage.includes('RPC connection') ||
                                     errorMessage.includes('fetch') ||
                                     errorMessage.includes('Failed to fetch') ||
                                     errorMessage.includes('NetworkError') ||
                                     errorMessage.includes('ENOTFOUND') ||
                                     errorMessage.includes('ECONNREFUSED');
                
                if (isNetworkError) {
                    // Network/connectivity issue - switch to offline mode
                    console.warn('Network connectivity issue detected, switching to offline mode:', errorMessage);
                    
                    isOfflineMode = true;
                    
                    // Show manual fee input
                    document.getElementById("manualFeeInput").style.display = 'block';
                    document.getElementById("feeOptions").style.display = 'none';
                    
                    // Update UI for offline mode
                    document.getElementById("estimatedFee").textContent = "Enter manual fee below";
                    document.getElementById("totalCost").textContent = "—";
                    document.getElementById("transactionError").textContent = `Network unavailable: ${errorMessage}`;
                    
                    // Focus on manual fee input
                    document.getElementById("manualFee").focus();
                    
                    // Disable create transaction until manual fee is entered
                    document.getElementById("createTransaction").disabled = true;
                } else {
                    // Other JavaScript error - re-throw to be handled by outer catch
                    throw jsError;
                }
            }

        } catch (error) {
            console.error('Fee calculation error:', error);
            
            // Check if error is due to amount being too low or invalid
            const errorMessage = error.message || '';
            const isLowAmountError = errorMessage.includes('Amount too low') || 
                                   errorMessage.includes('minimum transaction amount') ||
                                   errorMessage.includes('insufficient') || 
                                   errorMessage.includes('too low') ||
                                   errorMessage.includes('minimum') ||
                                   errorMessage.includes('dust');
            
            const isInvalidAmountError = errorMessage.includes('Invalid amount') ||
                                       errorMessage.includes('must be a positive number');
            
            if (isLowAmountError) {
                document.getElementById("estimatedFee").textContent = "Amount too low";
                document.getElementById("totalCost").textContent = "—";
                document.getElementById("transactionError").textContent = `⚠️ ${errorMessage}. Try increasing the amount.`;
            } else if (isInvalidAmountError) {
                document.getElementById("estimatedFee").textContent = "Invalid amount";
                document.getElementById("totalCost").textContent = "—";
                document.getElementById("transactionError").textContent = `⚠️ ${errorMessage}`;
            } else {
                document.getElementById("estimatedFee").textContent = "Calculation failed";
                document.getElementById("totalCost").textContent = "—";
                document.getElementById("transactionError").textContent = error.message;
            }
        }
        });
    }

    // Fee selection radio button handlers
    const feeSelectionRadios = document.querySelectorAll('input[name="feeSelection"]');
    if (feeSelectionRadios.length > 0) {
        feeSelectionRadios.forEach(radio => {
        radio.addEventListener('change', () => {
            const isCustom = radio.value === 'custom';
            const customInput = document.getElementById('customFeeInput');
            const warningElement = document.getElementById('customFeeWarning');
            
            // Enable/disable custom fee input based on selection
            customInput.disabled = !isCustom;
            
            if (isCustom) {
                customInput.focus();
                validateCustomFee(); // Validate existing value when switching to custom
            } else {
                // Hide warning when not using custom fee
                warningElement.style.display = 'none';
            }
            
            updateFeeDisplay();
        });
        });
    }

    // Custom fee input handler
    const customFeeInput = document.getElementById('customFeeInput');
    if (customFeeInput) {
        customFeeInput.addEventListener('input', () => {
        const customRadio = document.querySelector('input[name="feeSelection"][value="custom"]');
        if (customRadio && customRadio.checked) {
            validateCustomFee();
            updateFeeDisplay();
        }
        });
    }

    // Custom fee validation function
    function validateCustomFee() {
        const customFeeValue = parseFloat(document.getElementById('customFeeInput').value) || 0;
        const warningElement = document.getElementById('customFeeWarning');
        
        // Define minimum recommended fee thresholds based on actual network fees
        // Slow fee is typically around 0.00008333 KAS, so adjust thresholds accordingly
        const EXTREMELY_LOW_FEE = 0.00002; // 0.00002 KAS - extremely low, will likely fail
        const VERY_LOW_FEE = 0.00005; // 0.00005 KAS - too low, may not confirm
        
        if (customFeeValue > 0 && customFeeValue <= EXTREMELY_LOW_FEE) {
            // Extremely low fee - strong warning
            warningElement.textContent = '⚠️ Fee extremely low - transaction will likely fail';
            warningElement.style.color = '#dc3545'; // Red
            warningElement.style.display = 'block';
        } else if (customFeeValue > 0 && customFeeValue <= VERY_LOW_FEE) {
            // Low fee - warning
            warningElement.textContent = '⚠️ Fee too low - transaction may not confirm';
            warningElement.style.color = '#fd7e14'; // Orange
            warningElement.style.display = 'block';
        } else if (customFeeValue > VERY_LOW_FEE) {
            // Good fee (above the low threshold) - hide warning
            warningElement.style.display = 'none';
        } else {
            // No fee entered or zero - hide warning
            warningElement.style.display = 'none';
        }
    }

    // Create transaction handler
    
    if (createTransactionBtn) {
        
        createTransactionBtn.addEventListener("click", async () => {
        
        try {
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }

            const toAddress = document.getElementById("toAddress").value;
            const amount = document.getElementById("amount").value;

            document.getElementById("transactionStatus").textContent = "Creating...";
            
            let result;
            
            if (isOfflineMode) {
                // Offline mode - use manual fee
                const manualFee = document.getElementById("manualFee").value;
                
                if (!manualFee || parseFloat(manualFee) < 0) {
                    alert("Please enter a valid manual fee (must be 0 or positive)");
                    return;
                }
                
                result = await createTransactionWithManualFee(currentWallet.address, toAddress, amount, manualFee, currentWallet.networkType);
            } else {
                // Online mode - use automatic or custom fee calculation
                const selectedFeeOption = document.querySelector('input[name="feeSelection"]:checked')?.value || 'normal';
                
                if (selectedFeeOption === 'custom') {
                    // Use custom fee from input
                    const customFee = document.getElementById('customFeeInput').value;
                    
                    if (!customFee || parseFloat(customFee) < 0) {
                        alert("Please enter a valid custom fee (must be 0 or positive)");
                        return;
                    }
                    
                    result = await createTransactionWithManualFee(currentWallet.address, toAddress, amount, customFee, currentWallet.networkType);
                } else {
                    // Use preset fee options
                    result = await createTransaction(currentWallet.address, toAddress, amount, currentWallet.networkType, selectedFeeOption);
                }
            }
            
            if (result.success) {
                currentTransactionData = result;
                document.getElementById("transactionId").textContent = result.transactionId;
                document.getElementById("transactionStatus").textContent = "Created ✓";
                
                // Safe fee display with fee mode indication
                let feeDisplay = 'Fee: Unknown';
                if (result.fee && typeof result.fee.feeInKas === 'number') {
                    let feeType = isOfflineMode ? 'manual' : 'automatic';
                    
                    // Check if custom fee was used in online mode
                    const selectedFeeOption = document.querySelector('input[name="feeSelection"]:checked')?.value;
                    if (!isOfflineMode && selectedFeeOption === 'custom') {
                        feeType = 'custom';
                    }
                    
                    feeDisplay = `Fee: ${result.fee.feeInKas.toFixed(8)} KAS (${feeType})`;
                }
                document.getElementById("transactionDetails").textContent = feeDisplay;
                document.getElementById("transactionError").textContent = "—";
                
                // Enable next step buttons
                document.getElementById("downloadUnsigned").disabled = false;
                document.getElementById("signTransaction").disabled = false;
                document.getElementById("checkStatus").disabled = false;
                document.getElementById("generateUnsignedTxQR").disabled = false;
                
                // Auto-generate unsigned transaction QR
                try {
    
                    const qrResult = await generateUnsignedTransactionQR(currentTransactionData);
                    
                    if (qrResult.success) {
                        currentUnsignedTxQR = qrResult;
                        
                        // Display QR code
                        const qrContainer = document.getElementById("transactionQRContainer");
                        qrContainer.innerHTML = '';
                        const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Unsigned Transaction QR Code");
                        qrContainer.appendChild(qrDisplay);
                        
                        // Update QR info
                        document.getElementById("transactionQRType").textContent = "Unsigned Transaction";
                        document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                        
                        // Show QR area and enable download
                        document.getElementById("transactionQRDisplayArea").style.display = "block";
                        document.getElementById("downloadUnsignedTxQR").disabled = false;
                        
                        // Disable other QR download buttons since we're showing unsigned QR
                        document.getElementById("downloadSignedTxQR").disabled = true;
                        document.getElementById("downloadSubmittedTxQR").disabled = true;
                        
    
                    } else {
                        console.error("Failed to auto-generate unsigned QR:", qrResult.error);
                    }
                } catch (qrError) {
                    console.error("Error auto-generating unsigned QR:", qrError);
                }
            } else {
                document.getElementById("transactionStatus").textContent = "Failed ✗";
                document.getElementById("transactionError").textContent = result.error;
            }

        } catch (error) {
            document.getElementById("transactionStatus").textContent = "Error ✗";
            document.getElementById("transactionError").textContent = error.message;
        }
    });

    // Download unsigned transaction
    document.getElementById("downloadUnsigned").addEventListener("click", () => {
        if (currentTransactionData) {
            const exportData = exportTransaction(currentTransactionData);
            const filename = `kaspa-transaction-unsigned-${currentTransactionData.transactionId}.json`;
            downloadFile(exportData, filename);
        }
    });

    // Upload unsigned transaction
    document.getElementById("uploadUnsigned").addEventListener("click", () => {
        document.getElementById("unsignedFileInput").click();
    });

    document.getElementById("unsignedFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;


            const fileContent = await file.text();
            const transactionData = JSON.parse(fileContent);
            


            // Validate the uploaded transaction data
            if (!transactionData.transactionId || !transactionData.fromAddress || !transactionData.toAddress) {
                throw new Error('Invalid unsigned transaction file format');
            }

            // Validate it's the correct type
            if (transactionData.type !== 'kaspa-unsigned-transaction') {
                throw new Error('File is not a valid unsigned transaction file');
            }

            // For uploaded unsigned transactions, we need to recreate the transaction
            // since we can't reliably serialize/deserialize pending transactions

            
            // Mark this as an uploaded transaction that needs recreation
            transactionData.isUploaded = true;

            // Set as current transaction data
            currentTransactionData = transactionData;
            
            // Update UI with detailed information
            document.getElementById("transactionId").textContent = transactionData.transactionId;
            document.getElementById("transactionStatus").textContent = "Uploaded ✓";
            document.getElementById("transactionDetails").textContent = 
                `From: ${transactionData.fromAddress.substring(0, 20)}... | ` +
                `To: ${transactionData.toAddress.substring(0, 20)}... | ` +
                `Amount: ${transactionData.amount} KAS | ` +
                `Network: ${transactionData.networkType} | ` +
                `File: ${file.name}`;
            document.getElementById("transactionError").textContent = "—";
            
            // Enable signing and status check
            document.getElementById("signTransaction").disabled = false;
            document.getElementById("checkStatus").disabled = false;
            document.getElementById("generateUnsignedTxQR").disabled = false;
            
            alert('Unsigned transaction uploaded successfully!');

        } catch (error) {
            console.error('Error uploading unsigned transaction:', error);
            alert('Error uploading unsigned transaction: ' + error.message);
        }
        
        // Reset file input
        event.target.value = '';
    });

    // Sign transaction handler
    document.getElementById("signTransaction").addEventListener("click", async () => {
        try {
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }

            if (!currentTransactionData) {
                alert("Please create a transaction first");
                return;
            }

            document.getElementById("transactionStatus").textContent = "Signing...";
            
            const result = await signTransaction(currentTransactionData, currentWallet.privateKey);
            
            if (result.success) {
                // Serialize the signed transaction for QR/file export
                let serializedTransaction = null;
                try {
                    if (result.signedTransaction && result.signedTransaction.serializeToObject) {
                        const rawSerialized = result.signedTransaction.serializeToObject();
                        // Convert BigInt values to strings for JSON serialization (same as download handler)
                        serializedTransaction = JSON.parse(JSON.stringify(rawSerialized, (key, value) =>
                            typeof value === 'bigint' ? value.toString() : value
                        ));
                    }
                } catch (serializeError) {
                    console.warn("Could not serialize signed transaction:", serializeError);
                }
                
                // Preserve original transaction data and add signing result
                currentSignedTransactionData = {
                    ...currentTransactionData, // Include original transaction details
                    ...result, // Include signing result
                    status: 'signed',
                    serializedTransaction: serializedTransaction
                };
                document.getElementById("transactionStatus").textContent = "Signed ✓";
                document.getElementById("transactionDetails").textContent = `Transaction signed successfully`;
                document.getElementById("transactionError").textContent = "—";
                
                // Enable next step buttons
                document.getElementById("downloadSigned").disabled = false;
                document.getElementById("submitTransaction").disabled = false;
                document.getElementById("generateSignedTxQR").disabled = false;
                
                // Auto-generate signed transaction QR
                try {
                    
                    const qrResult = await generateSignedTransactionQR(currentSignedTransactionData);
                    
                    if (qrResult.success) {
                        currentSignedTxQR = qrResult;
                        
                        // Display QR code
                        const qrContainer = document.getElementById("transactionQRContainer");
                        qrContainer.innerHTML = '';
                        const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Signed Transaction QR Code");
                        qrContainer.appendChild(qrDisplay);
                        
                        // Update QR info
                        document.getElementById("transactionQRType").textContent = "Signed Transaction";
                        document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                        
                        // Show QR area and enable download
                        document.getElementById("transactionQRDisplayArea").style.display = "block";
                        document.getElementById("downloadSignedTxQR").disabled = false;
                        
                        // Disable other QR download buttons since we're showing signed QR
                        document.getElementById("downloadUnsignedTxQR").disabled = true;
                        document.getElementById("downloadSubmittedTxQR").disabled = true;
                        
    
                    } else {
                        console.error("Failed to auto-generate signed QR:", qrResult.error);
                    }
                } catch (qrError) {
                    console.error("Error auto-generating signed QR:", qrError);
                }
            } else {
                document.getElementById("transactionStatus").textContent = "Sign Failed ✗";
                document.getElementById("transactionError").textContent = result.error;
            }

        } catch (error) {
            document.getElementById("transactionStatus").textContent = "Sign Error ✗";
            document.getElementById("transactionError").textContent = error.message;
        }
    });

    // Download signed transaction
    document.getElementById("downloadSigned").addEventListener("click", () => {
        try {
            if (currentSignedTransactionData) {    
                
                // Use the SDK's serializeToObject method to get the actual transaction data
                const serializedTransaction = currentSignedTransactionData.signedTransaction.serializeToObject();
                
                // Convert BigInt values to strings for JSON serialization
                const jsonSafeTransaction = JSON.parse(JSON.stringify(serializedTransaction, (key, value) =>
                    typeof value === 'bigint' ? value.toString() : value
                ));
                
                const exportData = {
                    type: 'kaspa-signed-transaction',
                    transactionId: currentSignedTransactionData.transactionId,
                    status: currentSignedTransactionData.status,
                    timestamp: new Date().toISOString(),
                    networkType: currentWallet ? currentWallet.networkType : 'unknown',
                    // Include original transaction parameters for recreation
                    fromAddress: currentWallet ? currentWallet.address : null,
                    toAddress: document.getElementById("toAddress").value,
                    amount: document.getElementById("amount").value,
                    transaction: jsonSafeTransaction
                };
                
                const filename = `kaspa-transaction-signed-${currentSignedTransactionData.transactionId}.json`;
                downloadFile(JSON.stringify(exportData, null, 2), filename);
            } else {
                alert('No signed transaction data available');
            }
        } catch (error) {
            console.error('Error downloading signed transaction:', error);
            alert('Error downloading signed transaction: ' + error.message);
        }
    });

    // Upload signed transaction
    document.getElementById("uploadSigned").addEventListener("click", () => {
        document.getElementById("signedFileInput").click();
    });

    document.getElementById("signedFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const fileContent = await file.text();
            const signedTransactionData = JSON.parse(fileContent);

            // Validate the uploaded signed transaction data
            if (!signedTransactionData.transactionId || !signedTransactionData.transaction) {
                throw new Error('Invalid signed transaction file format');
            }

            // Validate it's the correct type
            if (signedTransactionData.type !== 'kaspa-signed-transaction') {
                throw new Error('File is not a valid signed transaction file');
            }

            // Store the serialized transaction data - we'll reconstruct it when needed for submission
            // This avoids the complex type conversion issues during upload
            currentSignedTransactionData = {
                transactionId: signedTransactionData.transactionId,
                status: 'uploaded',
                serializedTransaction: signedTransactionData.transaction, // Keep the raw serialized data
                networkType: signedTransactionData.networkType || 'mainnet',
                // Include original transaction parameters for recreation
                fromAddress: signedTransactionData.fromAddress,
                toAddress: signedTransactionData.toAddress,
                amount: signedTransactionData.amount,
                isUploaded: true // Mark as uploaded so submission knows to handle it specially
            };
            
            // Update UI with detailed information
            document.getElementById("transactionId").textContent = signedTransactionData.transactionId;
            document.getElementById("transactionStatus").textContent = "Signed & Uploaded ✓";
            document.getElementById("transactionDetails").textContent = 
                `Network: ${signedTransactionData.networkType} | ` +
                `Status: Ready for submission | ` +
                `File: ${file.name}`;
            document.getElementById("transactionError").textContent = "—";
            
            // Enable submission and status check
            document.getElementById("submitTransaction").disabled = false;
            document.getElementById("checkStatus").disabled = false;
            document.getElementById("generateSignedTxQR").disabled = false;
            
            alert('Signed transaction uploaded successfully!');

        } catch (error) {
            console.error('Error uploading signed transaction:', error);
            alert('Error uploading signed transaction: ' + error.message);
        }
        
        // Reset file input
        event.target.value = '';
    });

    // Submit transaction handler
    document.getElementById("submitTransaction").addEventListener("click", async () => {
        try {
            if (!currentSignedTransactionData) {
                alert("Please sign the transaction first");
                return;
            }



            // Add network type to signed transaction data if missing
            if (!currentSignedTransactionData.networkType && currentWallet) {
                currentSignedTransactionData.networkType = currentWallet.networkType;

            }

            document.getElementById("transactionStatus").textContent = "Submitting...";
            
            const result = await submitTransaction(currentSignedTransactionData);
            
            if (result.success) {
                // Update the transaction status and ID
                currentSignedTransactionData.status = 'submitted';
                if (result.transactionId && result.transactionId !== currentSignedTransactionData.transactionId) {
                    // Update with the real transaction ID from the network
                    currentSignedTransactionData.transactionId = result.transactionId;
                    document.getElementById("transactionId").textContent = result.transactionId;
                }
                
                document.getElementById("transactionStatus").textContent = "Submitted ✓";
                document.getElementById("transactionDetails").textContent = result.networkResponse.message;
                document.getElementById("transactionError").textContent = "—";
                
                // Enable status check and download submitted buttons
                document.getElementById("checkStatus").disabled = false;
                document.getElementById("downloadSubmitted").disabled = false;
                document.getElementById("generateSubmittedTxQR").disabled = false;
                
                // Store submitted transaction data for QR generation
                currentSubmittedTransactionData = {
                    ...currentSignedTransactionData,
                    status: 'submitted',
                    networkResponse: result.networkResponse,
                    submissionTimestamp: new Date().toISOString()
                };
                
                // Auto-generate submitted transaction QR
                try {
    
                    const qrResult = await generateSubmittedTransactionQR(currentSubmittedTransactionData);
                    
                    if (qrResult.success) {
                        currentSubmittedTxQR = qrResult;
                        
                        // Display QR code
                        const qrContainer = document.getElementById("transactionQRContainer");
                        qrContainer.innerHTML = '';
                        const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Submitted Transaction QR Code");
                        qrContainer.appendChild(qrDisplay);
                        
                        // Update QR info
                        document.getElementById("transactionQRType").textContent = "Submitted Transaction";
                        document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                        
                        // Show QR area and enable download
                        document.getElementById("transactionQRDisplayArea").style.display = "block";
                        document.getElementById("downloadSubmittedTxQR").disabled = false;
                        
                        // Disable other QR download buttons since we're showing submitted QR
                        document.getElementById("downloadUnsignedTxQR").disabled = true;
                        document.getElementById("downloadSignedTxQR").disabled = true;
                        
    
                    } else {
                        console.error("Failed to auto-generate submitted QR:", qrResult.error);
                    }
                } catch (qrError) {
                    console.error("Error auto-generating submitted QR:", qrError);
                }
            } else {
                console.error('Transaction submission failed:', result.error);
                document.getElementById("transactionStatus").textContent = "Submit Failed ✗";
                document.getElementById("transactionError").textContent = result.error;
            }

        } catch (error) {
            console.error('Error in transaction submission event handler:', error);
            document.getElementById("transactionStatus").textContent = "Submit Error ✗";
            document.getElementById("transactionError").textContent = error.message;
        }
    });

    // Check status handler
    document.getElementById("checkStatus").addEventListener("click", async () => {
        try {
            const currentTransaction = getCurrentTransactionId();
            if (!currentTransaction) {
                alert("No transaction to check");
                return;
            }

            document.getElementById("transactionDetails").textContent = "Checking status...";
            
            // Check if this is a local/pending transaction that hasn't been submitted
            if (currentTransaction.id.startsWith('pending_')) {
                // This is a locally generated transaction that hasn't been submitted to the network
                const transactionData = currentTransaction.source === 'signed' ? currentSignedTransactionData : currentTransactionData;
                const isSubmitted = transactionData.status === 'submitted';
                
                if (isSubmitted) {
                    document.getElementById("transactionDetails").textContent = 
                        `Status: Submitted to network, awaiting confirmation (${currentTransaction.source} transaction)`;
                } else {
                    document.getElementById("transactionDetails").textContent = 
                        `Status: Ready for submission - not yet sent to network (${currentTransaction.source} transaction)`;
                }
                return;
            }
            
            // For real transaction IDs, check network status
            const status = await getTransactionStatus(currentTransaction.id, currentTransaction.networkType);
            
            let statusText = '';
            if (status.status === 'confirmed') {
                statusText = `Status: Confirmed ✓, Confirmations: ${status.confirmations}`;
            } else if (status.status === 'pending') {
                statusText = `Status: Pending confirmation, Confirmations: ${status.confirmations}`;
            } else if (status.status === 'not_found') {
                statusText = `Status: Not found on network - may not be submitted yet`;
            } else {
                statusText = `Status: ${status.status}, Confirmations: ${status.confirmations || 0}`;
            }
            
            document.getElementById("transactionDetails").textContent = 
                `${statusText} (${currentTransaction.source} transaction)`;

        } catch (error) {
            document.getElementById("transactionError").textContent = error.message;
        }
    });

    // Download submitted transaction
    document.getElementById("downloadSubmitted").addEventListener("click", () => {
        try {
            if (currentSignedTransactionData && currentSignedTransactionData.status === 'submitted') {
                const exportData = {
                    type: 'kaspa-submitted-transaction',
                    transactionId: currentSignedTransactionData.transactionId,
                    status: currentSignedTransactionData.status,
                    timestamp: new Date().toISOString(),
                    networkType: currentSignedTransactionData.networkType || 'mainnet',
                    // Include original transaction parameters for recreation
                    fromAddress: currentSignedTransactionData.fromAddress,
                    toAddress: currentSignedTransactionData.toAddress,
                    amount: currentSignedTransactionData.amount,
                    // Include serialized transaction if available
                    transaction: currentSignedTransactionData.serializedTransaction || null
                };
                
                const filename = `kaspa-transaction-submitted-${currentSignedTransactionData.transactionId}.json`;
                downloadFile(JSON.stringify(exportData, null, 2), filename);
            } else {
                alert('No submitted transaction data available');
            }
        } catch (error) {
            console.error('Error downloading submitted transaction:', error);
            alert('Error downloading submitted transaction: ' + error.message);
        }
    });

    // Upload submitted transaction
    document.getElementById("uploadSubmitted").addEventListener("click", () => {
        document.getElementById("submittedFileInput").click();
    });

    document.getElementById("submittedFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const fileContent = await file.text();
            const submittedTransactionData = JSON.parse(fileContent);

            // Validate the uploaded submitted transaction data
            if (!submittedTransactionData.transactionId) {
                throw new Error('Invalid submitted transaction file format');
            }

            // Validate it's the correct type
            if (submittedTransactionData.type !== 'kaspa-submitted-transaction') {
                throw new Error('File is not a valid submitted transaction file');
            }

            // Store the submitted transaction data
            currentSignedTransactionData = {
                transactionId: submittedTransactionData.transactionId,
                status: 'submitted',
                serializedTransaction: submittedTransactionData.transaction,
                networkType: submittedTransactionData.networkType || 'mainnet',
                fromAddress: submittedTransactionData.fromAddress,
                toAddress: submittedTransactionData.toAddress,
                amount: submittedTransactionData.amount,
                isUploaded: true
            };
            
            // Update UI with detailed information
            document.getElementById("transactionId").textContent = submittedTransactionData.transactionId;
            document.getElementById("transactionStatus").textContent = "Submitted & Uploaded ✓";
            document.getElementById("transactionDetails").textContent = 
                `Network: ${submittedTransactionData.networkType} | ` +
                `Status: Submitted to network | ` +
                `File: ${file.name}`;
            document.getElementById("transactionError").textContent = "—";
            
            // Enable status check
            document.getElementById("checkStatus").disabled = false;
            document.getElementById("generateSubmittedTxQR").disabled = false;
            
            // Store submitted transaction data for QR generation
            currentSubmittedTransactionData = submittedTransactionData;
            
            alert('Submitted transaction uploaded successfully!');

        } catch (error) {
            console.error('Error uploading submitted transaction:', error);
            alert('Error uploading submitted transaction: ' + error.message);
        }
        
        // Reset file input
        event.target.value = '';
    });

    // Transaction QR Code Event Handlers
    
    // Generate Unsigned Transaction QR
    document.getElementById("generateUnsignedTxQR").addEventListener("click", async () => {
        try {
            if (!currentTransactionData) {
                alert("Please create a transaction first");
                return;
            }

            const qrResult = await generateUnsignedTransactionQR(currentTransactionData);
            
            if (qrResult.success) {
                currentUnsignedTxQR = qrResult;
                
                // Display QR code
                const qrContainer = document.getElementById("transactionQRContainer");
                qrContainer.innerHTML = '';
                const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Unsigned Transaction QR Code");
                qrContainer.appendChild(qrDisplay);
                
                // Update QR info
                document.getElementById("transactionQRType").textContent = "Unsigned Transaction";
                document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                
                // Show QR area and enable download
                document.getElementById("transactionQRDisplayArea").style.display = "block";
                document.getElementById("downloadUnsignedTxQR").disabled = false;
                
                // Disable other QR download buttons since we're showing unsigned QR
                document.getElementById("downloadSignedTxQR").disabled = true;
                document.getElementById("downloadSubmittedTxQR").disabled = true;
                
                alert("Unsigned transaction QR code generated successfully!");
            } else {
                alert("Error generating QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error generating unsigned transaction QR:", error);
            alert("Error generating QR code: " + error.message);
        }
    });
    
    // Download Unsigned Transaction QR
    const downloadUnsignedTxQRBtn = document.getElementById("downloadUnsignedTxQR");
    if (downloadUnsignedTxQRBtn) {
        downloadUnsignedTxQRBtn.addEventListener("click", () => {
            try {
                if (!currentUnsignedTxQR) {
                    alert("No unsigned transaction QR code available");
                    return;
                }
                
                const filename = `kaspa-unsigned-tx-qr-${currentTransactionData.transactionId}.png`;
                downloadQRImage(currentUnsignedTxQR.qrDataURL, filename);
            } catch (error) {
                alert("Error downloading QR code: " + error.message);
            }
        });
    }
    
    // Upload Unsigned Transaction QR
    const uploadUnsignedTxQRBtn = document.getElementById("uploadUnsignedTxQR");
    if (uploadUnsignedTxQRBtn) {
        uploadUnsignedTxQRBtn.addEventListener("click", () => {
            const unsignedTxQRInput = document.getElementById("unsignedTxQRInput");
            if (unsignedTxQRInput) {
                unsignedTxQRInput.click();
            }
        });
    }
    
    const unsignedTxQRInput = document.getElementById("unsignedTxQRInput");
    if (unsignedTxQRInput) {
        unsignedTxQRInput.addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;
            
            const qrResult = await readQRFromImage(file);
                        
            if (qrResult.success) {
                
                // Handle both parsed and unparsed QR data
                let qrData;
                if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                    qrData = qrResult.qrData;
                } else if (qrResult.data && typeof qrResult.data === 'string') {
                    qrData = JSON.parse(qrResult.data);
                } else {
                    throw new Error('Invalid QR data format received');
                }
                
                const validation = validateTransactionQRData(qrData, 'unsigned-transaction');
                
                if (!validation.isValid) {
                    throw new Error(validation.error);
                }
                
                // Load the transaction data
                const txData = validation.data;
                currentTransactionData = {
                    transactionId: txData.transactionId,
                    fromAddress: txData.fromAddress,
                    toAddress: txData.toAddress,
                    amount: txData.amount,
                    amountInSompi: txData.amountInSompi,
                    fee: txData.fee,
                    feeMode: txData.feeMode,
                    networkType: txData.networkType,
                    timestamp: txData.timestamp,
                    status: 'created',
                    isUploaded: true
                };
                
                // Update UI
                document.getElementById("toAddress").value = txData.toAddress;
                document.getElementById("amount").value = txData.amount;
                document.getElementById("transactionId").textContent = txData.transactionId;
                document.getElementById("transactionStatus").textContent = "Unsigned (from QR)";
                document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                
                // Enable appropriate buttons
                document.getElementById("signTransaction").disabled = false;
                document.getElementById("downloadUnsigned").disabled = false;
                document.getElementById("generateUnsignedTxQR").disabled = false;
                
                // Clear any existing QR display since we uploaded new data
                document.getElementById("transactionQRDisplayArea").style.display = "none";
                resetTransactionQRButtons();
                
                alert("Unsigned transaction QR uploaded successfully!");
            } else {
                alert("Error reading QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error uploading unsigned transaction QR:", error);
            alert("Error uploading QR code: " + error.message);
        }
        
        event.target.value = '';
        });
    }
    
    // Generate Signed Transaction QR
    const generateSignedTxQRBtn = document.getElementById("generateSignedTxQR");
    if (generateSignedTxQRBtn) {
        generateSignedTxQRBtn.addEventListener("click", async () => {
        try {
            if (!currentSignedTransactionData) {
                alert("Please sign a transaction first");
                return;
            }

            const qrResult = await generateSignedTransactionQR(currentSignedTransactionData);
            
            if (qrResult.success) {
                currentSignedTxQR = qrResult;
                
                // Display QR code (handle both single and multi-part)
                const qrContainer = document.getElementById("transactionQRContainer");
                qrContainer.innerHTML = '';
                
                if (qrResult.isMultiPart) {
                    // Multi-part QR display
                    const multiQRDisplay = createMultiPartQRDisplay(qrResult, "Signed Transaction QR Code");
                    qrContainer.appendChild(multiQRDisplay);
                    
                    // Update QR info for multi-part
                    document.getElementById("transactionQRType").textContent = `Signed Transaction (${qrResult.totalParts} parts)`;
                    document.getElementById("transactionQRDataSize").textContent = qrResult.originalSize;
                } else {
                    // Single QR display
                    const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Signed Transaction QR Code");
                    qrContainer.appendChild(qrDisplay);
                    
                    // Update QR info for single QR
                    document.getElementById("transactionQRType").textContent = "Signed Transaction";
                    document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                }
                
                // Show QR area and enable download
                document.getElementById("transactionQRDisplayArea").style.display = "block";
                document.getElementById("downloadSignedTxQR").disabled = false;
                
                // Disable other QR download buttons since we're showing signed QR
                document.getElementById("downloadUnsignedTxQR").disabled = true;
                document.getElementById("downloadSubmittedTxQR").disabled = true;
                
                const message = qrResult.isMultiPart ? 
                    `Signed transaction QR code generated successfully! (${qrResult.totalParts} parts)` :
                    "Signed transaction QR code generated successfully!";
                alert(message);
            } else {
                alert("Error generating QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error generating signed transaction QR:", error);
            alert("Error generating QR code: " + error.message);
        }
        });
    }
    
    // Download Signed Transaction QR
    document.getElementById("downloadSignedTxQR").addEventListener("click", () => {
        try {
            if (!currentSignedTxQR) {
                alert("No signed transaction QR code available");
                return;
            }
            
            if (currentSignedTxQR.isMultiPart) {
                // Download all parts of multi-part QR
                const baseFilename = `kaspa-signed-tx-qr-${currentSignedTransactionData.transactionId}`;
                
                for (let i = 0; i < currentSignedTxQR.qrParts.length; i++) {
                    const part = currentSignedTxQR.qrParts[i];
                    const filename = `${baseFilename}-part${part.part}of${part.totalParts}.png`;
                    downloadQRImage(part.qrDataURL, filename);
                }
                
                alert(`Downloaded ${currentSignedTxQR.totalParts} QR code parts`);
            } else {
                // Download single QR
                const filename = `kaspa-signed-tx-qr-${currentSignedTransactionData.transactionId}.png`;
                downloadQRImage(currentSignedTxQR.qrDataURL, filename);
                alert("QR code downloaded successfully");
            }
        } catch (error) {
            alert("Error downloading QR code: " + error.message);
        }
    });
    
    // Upload Signed Transaction QR
    document.getElementById("uploadSignedTxQR").addEventListener("click", () => {
        document.getElementById("signedTxQRInput").click();
    });
    
    document.getElementById("signedTxQRInput").addEventListener("change", async (event) => {
        try {
            const files = event.target.files;
            if (!files || files.length === 0) return;
            
            let qrResult;
            
            if (files.length === 1) {
                // Single QR upload
                qrResult = await readQRFromImage(files[0]);
            } else {
                // Multi-part QR upload
                qrResult = await readMultiPartQRFromImages(files);
            }
            
            if (qrResult.success) {
                
                // Handle both parsed and unparsed QR data
                let qrData;
                if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                    qrData = qrResult.qrData;
                } else if (qrResult.data && typeof qrResult.data === 'string') {
                    qrData = JSON.parse(qrResult.data);
                } else {
                    throw new Error('Invalid QR data format received');
                }
                
                const validation = validateTransactionQRData(qrData, 'signed-transaction');
                
                if (!validation.isValid) {
                    throw new Error(validation.error);
                }
                
                // Load the signed transaction data
                const txData = validation.data;
                currentSignedTransactionData = {
                    transactionId: txData.transactionId,
                    fromAddress: txData.fromAddress,
                    toAddress: txData.toAddress,
                    amount: txData.amount,
                    amountInSompi: txData.amountInSompi,
                    fee: txData.fee,
                    feeMode: txData.feeMode,
                    networkType: txData.networkType,
                    timestamp: txData.timestamp,
                    status: 'signed',
                    isUploaded: true,
                    serializedTransaction: txData.serializedTransaction
                };
                
                // Update UI
                document.getElementById("toAddress").value = txData.toAddress;
                document.getElementById("amount").value = txData.amount;
                document.getElementById("transactionId").textContent = txData.transactionId;
                document.getElementById("transactionStatus").textContent = qrResult.isMultiPart ? 
                    `Signed (from ${files.length}-part QR)` : "Signed (from QR)";
                document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                
                // Enable appropriate buttons
                document.getElementById("submitTransaction").disabled = false;
                document.getElementById("downloadSigned").disabled = false;
                document.getElementById("generateSignedTxQR").disabled = false;
                
                // Clear any existing QR display since we uploaded new data
                document.getElementById("transactionQRDisplayArea").style.display = "none";
                resetTransactionQRButtons();
                
                const message = qrResult.isMultiPart ? 
                    `Signed transaction QR uploaded successfully! (${files.length} parts combined)` :
                    "Signed transaction QR uploaded successfully!";
                alert(message);
            } else {
                alert("Error reading QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error uploading signed transaction QR:", error);
            alert("Error uploading QR code: " + error.message);
        }
        
        event.target.value = '';
    });
    
    // Scan Unsigned Transaction QR
    document.getElementById("scanUnsignedTxQR").addEventListener("click", async () => {
        try {
            
            await openCameraQRScanner(async (qrResult) => {
                try {
                    
                    // Handle both parsed and unparsed QR data
                    let qrData;
                    if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                        qrData = qrResult.qrData;
                    } else if (qrResult.data && typeof qrResult.data === 'string') {
                        qrData = JSON.parse(qrResult.data);
                    } else {
                        throw new Error('Invalid QR data format received');
                    }
                    
                    const validation = validateTransactionQRData(qrData, 'unsigned-transaction');
                    
                    if (!validation.isValid) {
                        throw new Error(validation.error);
                    }
                    
                    // Load the unsigned transaction data
                    const txData = validation.data;
                    currentTransactionData = {
                        transactionId: txData.transactionId,
                        fromAddress: txData.fromAddress,
                        toAddress: txData.toAddress,
                        amount: txData.amount,
                        amountInSompi: txData.amountInSompi,
                        fee: txData.fee,
                        feeMode: txData.feeMode,
                        networkType: txData.networkType,
                        timestamp: txData.timestamp,
                        status: 'created',
                        isUploaded: true,
                        pendingTransaction: txData.pendingTransaction
                    };
                    
                    // Update UI
                    document.getElementById("toAddress").value = txData.toAddress;
                    document.getElementById("amount").value = txData.amount;
                    document.getElementById("transactionId").textContent = txData.transactionId;
                    document.getElementById("transactionStatus").textContent = qrResult.isMultiPart ? 
                        `Created (from ${qrResult.totalParts}-part camera QR)` : "Created (from camera QR)";
                    document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                    
                    // Enable appropriate buttons
                    document.getElementById("signTransaction").disabled = false;
                    document.getElementById("downloadUnsigned").disabled = false;
                    document.getElementById("generateUnsignedTxQR").disabled = false;
                    
                    // Clear any existing QR display since we scanned new data
                    document.getElementById("transactionQRDisplayArea").style.display = "none";
                    resetTransactionQRButtons();
                    
                    const message = qrResult.isMultiPart ? 
                        `Unsigned transaction QR scanned successfully! (${qrResult.totalParts} parts combined)` :
                        "Unsigned transaction QR scanned successfully!";
                    alert(message);
                    
                } catch (error) {
                    console.error("Error processing scanned unsigned transaction QR:", error);
                    alert("Error processing scanned QR: " + error.message);
                }
            });
            
        } catch (error) {
            console.error("Error opening camera scanner for unsigned transaction QR:", error);
            alert("Error opening camera scanner: " + error.message);
        }
    });
    
    // Scan Signed Transaction QR
    document.getElementById("scanSignedTxQR").addEventListener("click", async () => {
        try {
            
            await openCameraQRScanner(async (qrResult) => {
                try {
                    
                    // Handle both parsed and unparsed QR data
                    let qrData;
                    if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                        qrData = qrResult.qrData;
                    } else if (qrResult.data && typeof qrResult.data === 'string') {
                        qrData = JSON.parse(qrResult.data);
                    } else {
                        throw new Error('Invalid QR data format received');
                    }
                    
                    const validation = validateTransactionQRData(qrData, 'signed-transaction');
                    
                    if (!validation.isValid) {
                        throw new Error(validation.error);
                    }
                    
                    // Load the signed transaction data
                    const txData = validation.data;
                    currentSignedTransactionData = {
                        transactionId: txData.transactionId,
                        fromAddress: txData.fromAddress,
                        toAddress: txData.toAddress,
                        amount: txData.amount,
                        amountInSompi: txData.amountInSompi,
                        fee: txData.fee,
                        feeMode: txData.feeMode,
                        networkType: txData.networkType,
                        timestamp: txData.timestamp,
                        status: 'signed',
                        isUploaded: true,
                        serializedTransaction: txData.serializedTransaction
                    };
                    
                    // Update UI
                    document.getElementById("toAddress").value = txData.toAddress;
                    document.getElementById("amount").value = txData.amount;
                    document.getElementById("transactionId").textContent = txData.transactionId;
                    document.getElementById("transactionStatus").textContent = qrResult.isMultiPart ? 
                        `Signed (from ${qrResult.totalParts}-part camera QR)` : "Signed (from camera QR)";
                    document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                    
                    // Enable appropriate buttons
                    document.getElementById("submitTransaction").disabled = false;
                    document.getElementById("downloadSigned").disabled = false;
                    document.getElementById("generateSignedTxQR").disabled = false;
                    
                    // Clear any existing QR display since we scanned new data
                    document.getElementById("transactionQRDisplayArea").style.display = "none";
                    resetTransactionQRButtons();
                    
                    const message = qrResult.isMultiPart ? 
                        `Signed transaction QR scanned successfully! (${qrResult.totalParts} parts combined)` :
                        "Signed transaction QR scanned successfully!";
                    alert(message);
                    
                } catch (error) {
                    console.error("Error processing scanned signed transaction QR:", error);
                    alert("Error processing scanned QR: " + error.message);
                }
            });
            
        } catch (error) {
            console.error("Error opening camera scanner for signed transaction QR:", error);
            alert("Error opening camera scanner: " + error.message);
        }
    });
    
    // Scan Submitted Transaction QR
    document.getElementById("scanSubmittedTxQR").addEventListener("click", async () => {
        try {
            
            await openCameraQRScanner(async (qrResult) => {
                try {
                    
                    // Handle both parsed and unparsed QR data
                    let qrData;
                    if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                        qrData = qrResult.qrData;
                    } else if (qrResult.data && typeof qrResult.data === 'string') {
                        qrData = JSON.parse(qrResult.data);
                    } else {
                        throw new Error('Invalid QR data format received');
                    }
                    
                    const validation = validateTransactionQRData(qrData, 'submitted-transaction');
                    
                    if (!validation.isValid) {
                        throw new Error(validation.error);
                    }
                    
                    // Load the submitted transaction data
                    const txData = validation.data;
                    currentSubmittedTransactionData = {
                        transactionId: txData.transactionId,
                        fromAddress: txData.fromAddress,
                        toAddress: txData.toAddress,
                        amount: txData.amount,
                        amountInSompi: txData.amountInSompi,
                        fee: txData.fee,
                        feeMode: txData.feeMode,
                        networkType: txData.networkType,
                        timestamp: txData.timestamp,
                        status: 'submitted',
                        networkResponse: txData.networkResponse,
                        submissionTimestamp: txData.submissionTimestamp
                    };
                    
                    // Update UI
                    document.getElementById("toAddress").value = txData.toAddress;
                    document.getElementById("amount").value = txData.amount;
                    document.getElementById("transactionId").textContent = txData.transactionId;
                    document.getElementById("transactionStatus").textContent = qrResult.isMultiPart ? 
                        `Submitted (from ${qrResult.totalParts}-part camera QR)` : "Submitted (from camera QR)";
                    document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                    
                    // Enable appropriate buttons
                    document.getElementById("checkStatus").disabled = false;
                    document.getElementById("downloadSubmitted").disabled = false;
                    document.getElementById("generateSubmittedTxQR").disabled = false;
                    
                    // Clear any existing QR display since we scanned new data
                    document.getElementById("transactionQRDisplayArea").style.display = "none";
                    resetTransactionQRButtons();
                    
                    const message = qrResult.isMultiPart ? 
                        `Submitted transaction QR scanned successfully! (${qrResult.totalParts} parts combined)` :
                        "Submitted transaction QR scanned successfully!";
                    alert(message);
                    
                } catch (error) {
                    console.error("Error processing scanned submitted transaction QR:", error);
                    alert("Error processing scanned QR: " + error.message);
                }
            });
            
        } catch (error) {
            console.error("Error opening camera scanner for submitted transaction QR:", error);
            alert("Error opening camera scanner: " + error.message);
        }
    });
    
    // Generate Submitted Transaction QR
    document.getElementById("generateSubmittedTxQR").addEventListener("click", async () => {
        try {
            if (!currentSubmittedTransactionData) {
                alert("Please submit a transaction first");
                return;
            }

            const qrResult = await generateSubmittedTransactionQR(currentSubmittedTransactionData);
            
            if (qrResult.success) {
                currentSubmittedTxQR = qrResult;
                
                // Display QR code
                const qrContainer = document.getElementById("transactionQRContainer");
                qrContainer.innerHTML = '';
                const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Submitted Transaction QR Code");
                qrContainer.appendChild(qrDisplay);
                
                // Update QR info
                document.getElementById("transactionQRType").textContent = "Submitted Transaction";
                document.getElementById("transactionQRDataSize").textContent = qrResult.size;
                
                // Show QR area and enable download
                document.getElementById("transactionQRDisplayArea").style.display = "block";
                document.getElementById("downloadSubmittedTxQR").disabled = false;
                
                // Disable other QR download buttons since we're showing submitted QR
                document.getElementById("downloadUnsignedTxQR").disabled = true;
                document.getElementById("downloadSignedTxQR").disabled = true;
                
                alert("Submitted transaction QR code generated successfully!");
            } else {
                alert("Error generating QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error generating submitted transaction QR:", error);
            alert("Error generating QR code: " + error.message);
        }
    });
    
    // Download Submitted Transaction QR
    document.getElementById("downloadSubmittedTxQR").addEventListener("click", () => {
        try {
            if (!currentSubmittedTxQR) {
                alert("No submitted transaction QR code available");
                return;
            }
            
            const filename = `kaspa-submitted-tx-qr-${currentSubmittedTransactionData.transactionId}.png`;
            downloadQRImage(currentSubmittedTxQR.qrDataURL, filename);
        } catch (error) {
            alert("Error downloading QR code: " + error.message);
        }
    });
    
    // Upload Submitted Transaction QR
    document.getElementById("uploadSubmittedTxQR").addEventListener("click", () => {
        document.getElementById("submittedTxQRInput").click();
    });
    
    document.getElementById("submittedTxQRInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const qrResult = await readQRFromImage(file);
            
            if (qrResult.success) {
                
                // Handle both parsed and unparsed QR data
                let qrData;
                if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                    qrData = qrResult.qrData;
                } else if (qrResult.data && typeof qrResult.data === 'string') {
                    qrData = JSON.parse(qrResult.data);
                } else {
                    throw new Error('Invalid QR data format received');
                }
                
                const validation = validateTransactionQRData(qrData, 'submitted-transaction');
                
                if (!validation.isValid) {
                    throw new Error(validation.error);
                }
                
                // Load the submitted transaction data
                const txData = validation.data;
                currentSubmittedTransactionData = {
                    transactionId: txData.transactionId,
                    fromAddress: txData.fromAddress,
                    toAddress: txData.toAddress,
                    amount: txData.amount,
                    amountInSompi: txData.amountInSompi,
                    fee: txData.fee,
                    feeMode: txData.feeMode,
                    networkType: txData.networkType,
                    timestamp: txData.timestamp,
                    status: 'submitted',
                    networkResponse: txData.networkResponse,
                    submissionTimestamp: txData.submissionTimestamp
                };
                
                // Update UI
                document.getElementById("toAddress").value = txData.toAddress;
                document.getElementById("amount").value = txData.amount;
                document.getElementById("transactionId").textContent = txData.transactionId;
                document.getElementById("transactionStatus").textContent = "Submitted (from QR)";
                document.getElementById("transactionDetails").textContent = `${txData.amount} KAS to ${txData.toAddress}`;
                
                // Enable appropriate buttons
                document.getElementById("checkStatus").disabled = false;
                document.getElementById("downloadSubmitted").disabled = false;
                document.getElementById("generateSubmittedTxQR").disabled = false;
                
                // Clear any existing QR display since we uploaded new data
                document.getElementById("transactionQRDisplayArea").style.display = "none";
                resetTransactionQRButtons();
                
                alert("Submitted transaction QR uploaded successfully!");
            } else {
                alert("Error reading QR code: " + qrResult.error);
            }
        } catch (error) {
            console.error("Error uploading submitted transaction QR:", error);
            alert("Error uploading QR code: " + error.message);
        }
        
        event.target.value = '';
    });
}

// End of setupTransactionEventHandlers function
}

// Message signing event handlers
function setupMessageSigningEventHandlers() {
    // Message character count
    const messageInput = document.getElementById("messageToSign");
    const charCount = document.getElementById("messageCharCount");
    
    if (messageInput && charCount) {
        messageInput.addEventListener("input", () => {
            charCount.textContent = messageInput.value.length;
        });
    }
    
    // Sign message handler
    document.getElementById("signMessage").addEventListener("click", async () => {
        try {
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }

            const message = document.getElementById("messageToSign").value;
            if (!message.trim()) {
                alert("Please enter a message to sign");
                return;
            }

            document.getElementById("signingStatus").textContent = "Signing...";
            document.getElementById("signingError").textContent = "—";
            
            const result = await signMessage(message, currentWallet.privateKey, currentWallet.address, currentWallet.networkType);
            
            if (result.success) {
                currentSignedMessageData = result;
                document.getElementById("signingId").textContent = result.signingId;
                document.getElementById("signingStatus").textContent = "Signed ✓";
                document.getElementById("messageSignature").textContent = result.signature;
                document.getElementById("signingError").textContent = "—";
                
                // Enable download button
                document.getElementById("downloadSignedMessage").disabled = false;
                document.getElementById("copySignature").disabled = false;
                document.getElementById("generateSignedQR").disabled = false;
                
                // Automatically generate signed QR code
                try {
                    document.getElementById("signingStatus").textContent = "Signed ✓ - Generating QR...";
                    
                    const qrResult = await generateSignedMessageQR(currentSignedMessageData);
                    
                    if (qrResult.success) {
                        currentSignedQR = qrResult;
                        
                        // Display QR code
                        const qrContainer = document.getElementById("qrContainer");
                        qrContainer.innerHTML = '';
                        const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Signed Message QR Code");
                        qrContainer.appendChild(qrDisplay);
                        
                        // Update QR info
                        document.getElementById("qrType").textContent = "Signed Message";
                        document.getElementById("qrDataSize").textContent = qrResult.size;
                        
                        // Show QR area and enable download
                        document.getElementById("qrDisplayArea").style.display = "block";
                        document.getElementById("downloadSignedQR").disabled = false;
                        
                        document.getElementById("signingStatus").textContent = "Signed ✓ - QR Generated";
                    } else {
                        console.error("Failed to generate signed QR:", qrResult.error);
                        document.getElementById("signingStatus").textContent = "Signed ✓ - QR Failed";
                    }
                } catch (qrError) {
                    console.error("Error generating signed QR:", qrError);
                    document.getElementById("signingStatus").textContent = "Signed ✓ - QR Error";
                }
            } else {
                document.getElementById("signingStatus").textContent = "Sign Failed ✗";
                document.getElementById("signingError").textContent = result.error;
            }

        } catch (error) {
            document.getElementById("signingStatus").textContent = "Sign Error ✗";
            document.getElementById("signingError").textContent = error.message;
        }
    });
    
    // Download unsigned message
    document.getElementById("downloadUnsignedMessage").addEventListener("click", () => {
        try {
            const message = document.getElementById("messageToSign").value;
            if (!message.trim()) {
                alert("Please enter a message first");
                return;
            }
            
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }
            
            const messageData = {
                message: message,
                signerAddress: currentWallet.address,
                networkType: currentWallet.networkType,
                timestamp: new Date().toISOString(),
                messageLength: message.length
            };
            
            const exportData = exportMessageData(messageData, 'unsigned-message');
            const filename = `kaspa-message-unsigned-${Date.now()}.json`;
            downloadFile(exportData, filename);
        } catch (error) {
            alert('Error downloading unsigned message: ' + error.message);
        }
    });
    
    // Upload unsigned message
    document.getElementById("uploadUnsignedMessage").addEventListener("click", () => {
        document.getElementById("unsignedMessageFileInput").click();
    });

    document.getElementById("unsignedMessageFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const fileContent = await file.text();
            const messageData = JSON.parse(fileContent);
            
            const validation = validateImportedMessageData(messageData, 'unsigned-message');
            if (!validation.isValid) {
                throw new Error(validation.error);
            }
            
            // Load the message data
            currentMessageData = validation.data;
            document.getElementById("messageToSign").value = messageData.message;
            document.getElementById("messageCharCount").textContent = messageData.message.length;
            
            alert('Unsigned message uploaded successfully!');

        } catch (error) {
            alert('Error uploading unsigned message: ' + error.message);
        }
        
        event.target.value = '';
    });
    
    // Download signed message
    document.getElementById("downloadSignedMessage").addEventListener("click", () => {
        try {
            if (!currentSignedMessageData) {
                alert('No signed message data available');
                return;
            }
            
            const exportData = exportMessageData(currentSignedMessageData, 'signed-message');
            const filename = `kaspa-message-signed-${currentSignedMessageData.signingId}.json`;
            downloadFile(exportData, filename);
        } catch (error) {
            alert('Error downloading signed message: ' + error.message);
        }
    });
    
    // Upload signed message
    document.getElementById("uploadSignedMessage").addEventListener("click", () => {
        document.getElementById("signedMessageFileInput").click();
    });

    document.getElementById("signedMessageFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const fileContent = await file.text();
            const messageData = JSON.parse(fileContent);
            
            const validation = validateImportedMessageData(messageData, 'signed-message');
            if (!validation.isValid) {
                throw new Error(validation.error);
            }
            
            // Load the signed message data
            currentSignedMessageData = validation.data;
            document.getElementById("messageToSign").value = messageData.message;
            document.getElementById("messageCharCount").textContent = messageData.message.length;
            document.getElementById("signingId").textContent = messageData.signingId;
            document.getElementById("signingStatus").textContent = "Signed & Uploaded ✓";
            document.getElementById("messageSignature").textContent = messageData.signature;
            document.getElementById("signingError").textContent = "—";
            
            // Enable buttons
            document.getElementById("downloadSignedMessage").disabled = false;
            document.getElementById("copySignature").disabled = false;
            document.getElementById("generateSignedQR").disabled = false;
            
            alert('Signed message uploaded successfully!');

        } catch (error) {
            alert('Error uploading signed message: ' + error.message);
        }
        
        event.target.value = '';
    });
    
    // Verify message handler
    document.getElementById("verifyMessage").addEventListener("click", async () => {
        try {
            const message = document.getElementById("messageToSign").value;
            const signature = document.getElementById("signatureToVerify").value;
            const address = document.getElementById("signerAddress").value;
            
            if (!message.trim()) {
                alert("Please enter a message to verify");
                return;
            }
            
            if (!signature.trim()) {
                alert("Please enter a signature to verify");
                return;
            }
            
            if (!address.trim()) {
                alert("Please enter a signer address or public key");
                return;
            }

            document.getElementById("verificationStatus").textContent = "Verifying...";
            document.getElementById("verificationError").textContent = "—";
            
            const networkType = currentWallet ? currentWallet.networkType : 'mainnet';
            const result = await verifyMessage(message, signature, address, networkType);
            
            if (result.success) {
                currentVerificationData = result;
                document.getElementById("verificationId").textContent = result.verificationId;
                document.getElementById("verificationStatus").textContent = "Verified ✓";
                document.getElementById("verificationResult").textContent = result.isValid ? "✅ Valid Signature" : "❌ Invalid Signature";
                document.getElementById("verificationError").textContent = "—";
                
                // Enable download button
                document.getElementById("downloadVerificationResult").disabled = false;
            } else {
                document.getElementById("verificationStatus").textContent = "Verify Failed ✗";
                document.getElementById("verificationError").textContent = result.error;
            }

        } catch (error) {
            document.getElementById("verificationStatus").textContent = "Verify Error ✗";
            document.getElementById("verificationError").textContent = error.message;
        }
    });
    
    // Download verification result
    document.getElementById("downloadVerificationResult").addEventListener("click", () => {
        try {
            if (!currentVerificationData) {
                alert('No verification result data available');
                return;
            }
            
            const exportData = exportMessageData(currentVerificationData, 'verification-result');
            const filename = `kaspa-verification-result-${currentVerificationData.verificationId}.json`;
            downloadFile(exportData, filename);
        } catch (error) {
            alert('Error downloading verification result: ' + error.message);
        }
    });
    
    // Upload verification result
    document.getElementById("uploadVerificationResult").addEventListener("click", () => {
        document.getElementById("verificationResultFileInput").click();
    });

    document.getElementById("verificationResultFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const fileContent = await file.text();
            const verificationData = JSON.parse(fileContent);
            
            const validation = validateImportedMessageData(verificationData, 'verification-result');
            if (!validation.isValid) {
                throw new Error(validation.error);
            }
            
            // Load the verification result data
            currentVerificationData = validation.data;
            document.getElementById("messageToSign").value = verificationData.message;
            document.getElementById("messageCharCount").textContent = verificationData.message.length;
            document.getElementById("signatureToVerify").value = verificationData.signature;
            document.getElementById("signerAddress").value = verificationData.signerAddress;
            document.getElementById("verificationId").textContent = verificationData.verificationId;
            document.getElementById("verificationStatus").textContent = "Uploaded ✓";
            document.getElementById("verificationResult").textContent = verificationData.isValid ? "✅ Valid Signature" : "❌ Invalid Signature";
            document.getElementById("verificationError").textContent = "—";
            
            // Enable download button
            document.getElementById("downloadVerificationResult").disabled = false;
            
            alert('Verification result uploaded successfully!');

        } catch (error) {
            alert('Error uploading verification result: ' + error.message);
        }
        
        event.target.value = '';
    });
    
    // Use current signing data for verification
    document.getElementById("useSigningData").addEventListener("click", () => {
        try {
            if (!currentSignedMessageData) {
                alert("No signed message data available. Please sign a message first.");
                return;
            }
            
            document.getElementById("signatureToVerify").value = currentSignedMessageData.signature;
            
            // Use public key if available, otherwise fall back to address
            if (currentSignedMessageData.signerPublicKey) {
                document.getElementById("signerAddress").value = currentSignedMessageData.signerPublicKey;
                alert("Signing data copied to verification fields! Using public key for verification.");
            } else {
                document.getElementById("signerAddress").value = currentSignedMessageData.signerAddress;
                alert("Signing data copied to verification fields! Using address for verification.");
            }
            
            // Clear previous verification status/result/error
            document.getElementById("verificationId").textContent = "—";
            document.getElementById("verificationStatus").textContent = "—";
            document.getElementById("verificationResult").textContent = "—";
            document.getElementById("verificationError").textContent = "—";
            document.getElementById("downloadVerificationResult").disabled = true;
            currentVerificationData = null;
            
        } catch (error) {
            alert('Error using signing data: ' + error.message);
        }
    });
    
    // Quick action handlers
    document.getElementById("clearMessage").addEventListener("click", () => {
        document.getElementById("messageToSign").value = "";
        document.getElementById("messageCharCount").textContent = "0";
    });
    
    document.getElementById("clearSignature").addEventListener("click", () => {
        document.getElementById("signingId").textContent = "—";
        document.getElementById("signingStatus").textContent = "—";
        document.getElementById("messageSignature").textContent = "—";
        document.getElementById("signingError").textContent = "—";
        document.getElementById("downloadSignedMessage").disabled = true;
        document.getElementById("copySignature").disabled = true;
        currentSignedMessageData = null;
    });
    
    document.getElementById("clearVerification").addEventListener("click", () => {
        document.getElementById("signatureToVerify").value = "";
        document.getElementById("signerAddress").value = "";
        document.getElementById("verificationId").textContent = "—";
        document.getElementById("verificationStatus").textContent = "—";
        document.getElementById("verificationResult").textContent = "—";
        document.getElementById("verificationError").textContent = "—";
        document.getElementById("downloadVerificationResult").disabled = true;
        currentVerificationData = null;
    });
    
    document.getElementById("copySignature").addEventListener("click", () => {
        if (currentSignedMessageData && currentSignedMessageData.signature) {
            navigator.clipboard.writeText(currentSignedMessageData.signature).then(() => {
                alert("Signature copied to clipboard!");
            }).catch(() => {
                // Fallback for older browsers
                const textArea = document.createElement('textarea');
                textArea.value = currentSignedMessageData.signature;
                document.body.appendChild(textArea);
                textArea.select();
                document.execCommand('copy');
                document.body.removeChild(textArea);
                alert("Signature copied to clipboard!");
            });
        }
    });

    // QR Code Event Handlers
    let currentUnsignedQR = null;
    let currentSignedQR = null;

    // Generate unsigned message QR
    document.getElementById("generateUnsignedQR").addEventListener("click", async () => {
        try {
            const message = document.getElementById("messageToSign").value;
            if (!message.trim()) {
                alert("Please enter a message first");
                return;
            }
            
            if (!currentWallet) {
                alert("Please generate or restore a wallet first");
                return;
            }
            
            const messageData = {
                message: message,
                signerAddress: currentWallet.address,
                networkType: currentWallet.networkType,
                timestamp: new Date().toISOString(),
                messageId: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
            };
            
            const qrResult = await generateUnsignedMessageQR(messageData);
            
            if (qrResult.success) {
                currentUnsignedQR = qrResult;
                
                // Display QR code
                const qrContainer = document.getElementById("qrContainer");
                qrContainer.innerHTML = '';
                const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Unsigned Message QR Code");
                qrContainer.appendChild(qrDisplay);
                
                // Update QR info
                document.getElementById("qrType").textContent = "Unsigned Message";
                document.getElementById("qrDataSize").textContent = qrResult.size;
                
                // Show QR area and enable download
                document.getElementById("qrDisplayArea").style.display = "block";
                document.getElementById("downloadUnsignedQR").disabled = false;
                
                alert("Unsigned message QR code generated successfully!");
            } else {
                alert("Error generating QR code: " + qrResult.error);
            }
        } catch (error) {
            alert("Error generating unsigned QR: " + error.message);
        }
    });

    // Download unsigned QR
    document.getElementById("downloadUnsignedQR").addEventListener("click", () => {
        try {
            if (!currentUnsignedQR) {
                alert("No unsigned QR code available");
                return;
            }
            
            const filename = `kaspa-unsigned-message-qr-${Date.now()}.png`;
            downloadQRImage(currentUnsignedQR.qrDataURL, filename);
        } catch (error) {
            alert("Error downloading QR image: " + error.message);
        }
    });

    // Generate signed message QR
    document.getElementById("generateSignedQR").addEventListener("click", async () => {
        try {
            if (!currentSignedMessageData) {
                alert("No signed message data available. Please sign a message first.");
                return;
            }
            
            const qrResult = await generateSignedMessageQR(currentSignedMessageData);
            
            if (qrResult.success) {
                currentSignedQR = qrResult;
                
                // Display QR code
                const qrContainer = document.getElementById("qrContainer");
                qrContainer.innerHTML = '';
                const qrDisplay = createQRDisplay(qrResult.qrDataURL, "Signed Message QR Code");
                qrContainer.appendChild(qrDisplay);
                
                // Update QR info
                document.getElementById("qrType").textContent = "Signed Message";
                document.getElementById("qrDataSize").textContent = qrResult.size;
                
                // Show QR area and enable download
                document.getElementById("qrDisplayArea").style.display = "block";
                document.getElementById("downloadSignedQR").disabled = false;
                
                alert("Signed message QR code generated successfully!");
            } else {
                alert("Error generating QR code: " + qrResult.error);
            }
        } catch (error) {
            alert("Error generating signed QR: " + error.message);
        }
    });

    // Download signed QR
    document.getElementById("downloadSignedQR").addEventListener("click", () => {
        try {
            if (!currentSignedQR) {
                alert("No signed QR code available");
                return;
            }
            
            const filename = `kaspa-signed-message-qr-${currentSignedMessageData.signingId}.png`;
            downloadQRImage(currentSignedQR.qrDataURL, filename);
        } catch (error) {
            alert("Error downloading QR image: " + error.message);
        }
    });

    // Upload QR image
    document.getElementById("uploadQRImage").addEventListener("click", () => {
        document.getElementById("qrImageFileInput").click();
    });

    document.getElementById("qrImageFileInput").addEventListener("change", async (event) => {
        try {
            const file = event.target.files[0];
            if (!file) return;

            const qrResult = await readQRFromImage(file);
            
            if (qrResult.success) {
                const qrData = qrResult.qrData;
                
                // Validate QR type and handle accordingly
                if (qrData.type === 'kaspa-unsigned-message-qr') {
                    // Handle unsigned message QR
                    const validation = validateQRData(qrData, 'unsigned-message');
                    if (!validation.isValid) {
                        throw new Error(validation.error);
                    }
                    
                    // Load message data
                    document.getElementById("messageToSign").value = qrData.message;
                    document.getElementById("messageCharCount").textContent = qrData.message.length;
                    
                    alert("Unsigned message QR uploaded successfully! You can now sign the message.");
                    
                } else if (qrData.type === 'kaspa-signed-message-qr') {
                    // Handle signed message QR
                    const validation = validateQRData(qrData, 'signed-message');
                    if (!validation.isValid) {
                        throw new Error(validation.error);
                    }
                    
                    // Load signed message data
                    currentSignedMessageData = {
                        success: true,
                        signingId: qrData.signingId,
                        message: qrData.message,
                        signature: qrData.signature,
                        signerAddress: qrData.signerAddress,
                        signerPublicKey: qrData.signerPublicKey,
                        networkType: qrData.networkType,
                        timestamp: qrData.timestamp
                    };
                    
                    // Update UI
                    document.getElementById("messageToSign").value = qrData.message;
                    document.getElementById("messageCharCount").textContent = qrData.message.length;
                    document.getElementById("signingId").textContent = qrData.signingId;
                    document.getElementById("signingStatus").textContent = "Signed & Uploaded ✓";
                    document.getElementById("messageSignature").textContent = qrData.signature;
                    document.getElementById("signingError").textContent = "—";
                    
                    // Enable buttons
                    document.getElementById("downloadSignedMessage").disabled = false;
                    document.getElementById("copySignature").disabled = false;
                    document.getElementById("generateSignedQR").disabled = false;
                    
                    alert("Signed message QR uploaded successfully!");
                    
                } else {
                    throw new Error("Unknown QR code type: " + qrData.type);
                }
                
            } else {
                alert("Error reading QR code: " + qrResult.error);
            }
            
        } catch (error) {
            alert("Error uploading QR image: " + error.message);
        }
        
        // Reset file input
        event.target.value = '';
    });
    
    // Scan Message QR
    document.getElementById("scanMessageQR").addEventListener("click", async () => {
        try {
            
            await openCameraQRScanner(async (qrResult) => {
                try {
                    
                    // Handle both parsed and unparsed QR data
                    let qrData;
                    if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                        qrData = qrResult.qrData;
                    } else if (qrResult.data && typeof qrResult.data === 'string') {
                        qrData = JSON.parse(qrResult.data);
                    } else {
                        throw new Error('Invalid QR data format received');
                    }
                    
                    // Validate QR type and handle accordingly
                    if (qrData.type === 'kaspa-unsigned-message-qr') {
                        // Handle unsigned message QR
                        const validation = validateQRData(qrData, 'unsigned-message');
                        if (!validation.isValid) {
                            throw new Error(validation.error);
                        }
                        
                        // Load message data
                        document.getElementById("messageToSign").value = qrData.message;
                        document.getElementById("messageCharCount").textContent = qrData.message.length;
                        
                        const message = qrResult.isMultiPart ? 
                            `Unsigned message QR scanned successfully! (${qrResult.totalParts} parts combined) You can now sign the message.` :
                            "Unsigned message QR scanned successfully! You can now sign the message.";
                        alert(message);
                        
                    } else if (qrData.type === 'kaspa-signed-message-qr') {
                        // Handle signed message QR
                        const validation = validateQRData(qrData, 'signed-message');
                        if (!validation.isValid) {
                            throw new Error(validation.error);
                        }
                        
                        // Load signed message data
                        currentSignedMessageData = {
                            success: true,
                            signingId: qrData.signingId,
                            message: qrData.message,
                            signature: qrData.signature,
                            signerAddress: qrData.signerAddress,
                            signerPublicKey: qrData.signerPublicKey,
                            networkType: qrData.networkType,
                            timestamp: qrData.timestamp
                        };
                        
                        // Update UI
                        document.getElementById("messageToSign").value = qrData.message;
                        document.getElementById("messageCharCount").textContent = qrData.message.length;
                        document.getElementById("signingId").textContent = qrData.signingId;
                        document.getElementById("signingStatus").textContent = qrResult.isMultiPart ? 
                            `Signed & Scanned ✓ (${qrResult.totalParts} parts)` : "Signed & Scanned ✓";
                        document.getElementById("messageSignature").textContent = qrData.signature;
                        document.getElementById("signingError").textContent = "—";
                        
                        // Enable buttons
                        document.getElementById("downloadSignedMessage").disabled = false;
                        document.getElementById("copySignature").disabled = false;
                        document.getElementById("generateSignedQR").disabled = false;
                        
                        const message = qrResult.isMultiPart ? 
                            `Signed message QR scanned successfully! (${qrResult.totalParts} parts combined)` :
                            "Signed message QR scanned successfully!";
                        alert(message);
                        
                    } else {
                        throw new Error("Unknown QR code type: " + qrData.type);
                    }
                    
                } catch (error) {
                    console.error("Error processing scanned message QR:", error);
                    alert("Error processing scanned QR: " + error.message);
                }
            });
            
        } catch (error) {
            console.error("Error opening camera scanner for message QR:", error);
            alert("Error opening camera scanner: " + error.message);
        }
    });


}

// Update fee display based on selected radio button
function updateFeeDisplay() {
    const selectedFee = document.querySelector('input[name="feeSelection"]:checked')?.value || 'normal';
    const amount = parseFloat(document.getElementById("amount").value) || 0;
    
    let selectedFeeAmount = 0;
    let feeLabel = selectedFee;
    
    if (selectedFee === 'custom') {
        // Handle custom fee input
        const customFeeValue = parseFloat(document.getElementById('customFeeInput').value) || 0;
        selectedFeeAmount = customFeeValue;
        feeLabel = 'custom';
    } else {
        // Handle preset fees (only if calculatedFees are available)
        if (!calculatedFees) return;
        
        switch (selectedFee) {
            case 'slow':
                selectedFeeAmount = calculatedFees.slow;
                break;
            case 'fast':
                selectedFeeAmount = calculatedFees.fast;
                break;
            default:
                selectedFeeAmount = calculatedFees.normal;
        }
    }
    
    const totalCost = amount + selectedFeeAmount;
    
    document.getElementById("estimatedFee").textContent = `${selectedFeeAmount.toFixed(8)} KAS (${feeLabel})`;
    document.getElementById("totalCost").textContent = `${totalCost.toFixed(8)} KAS`;
}

// Global utility functions
function copyToClipboard(elementId) {
    const element = document.getElementById(elementId);
    if (element) {
        if (element.tagName === 'INPUT' || element.tagName === 'TEXTAREA') {
            element.select();
            element.setSelectionRange(0, 99999); // For mobile devices
            document.execCommand('copy');
            
            // Visual feedback
            const originalBG = element.style.backgroundColor;
            element.style.backgroundColor = '#d4edda';
            setTimeout(() => {
                element.style.backgroundColor = originalBG;
            }, 1000);
        } else {
            // For other elements, copy their text content
            const textArea = document.createElement('textarea');
            textArea.value = element.textContent;
            document.body.appendChild(textArea);
            textArea.select();
            document.execCommand('copy');
            document.body.removeChild(textArea);
        }
    }
}

function toggleVisibility(elementId) {
    const element = document.getElementById(elementId);
    const toggleButton = element.nextElementSibling;
    
    if (element.type === 'password') {
        element.type = 'text';
        if (toggleButton && toggleButton.classList.contains('toggle-button')) {
            toggleButton.textContent = 'Hide';
        }
    } else {
        element.type = 'password';
        if (toggleButton && toggleButton.classList.contains('toggle-button')) {
            toggleButton.textContent = 'Show';
        }
    }
}

// Make utility functions globally available
window.copyToClipboard = copyToClipboard;
window.toggleVisibility = toggleVisibility;

// Get the current transaction ID from the most recent transaction (uploaded, created, or signed)
function resetTransactionQRButtons() {
    // Disable all QR download buttons
    document.getElementById("downloadUnsignedTxQR").disabled = true;
    document.getElementById("downloadSignedTxQR").disabled = true;
    document.getElementById("downloadSubmittedTxQR").disabled = true;
    
    // Clear QR data
    currentUnsignedTxQR = null;
    currentSignedTxQR = null;
    currentSubmittedTxQR = null;
    
    // Hide QR display area
    document.getElementById("transactionQRDisplayArea").style.display = "none";
}

function getCurrentTransactionId() {
    // Priority order: signed/submitted transaction > unsigned transaction
    if (currentSignedTransactionData && currentSignedTransactionData.transactionId) {
        let source = 'signed';
        if (currentSignedTransactionData.status === 'submitted') {
            source = 'submitted';
        }
        
        return {
            id: currentSignedTransactionData.transactionId,
            networkType: currentSignedTransactionData.networkType || (currentWallet ? currentWallet.networkType : 'mainnet'),
            source: source
        };
    }
    
    if (currentTransactionData && currentTransactionData.transactionId) {
        return {
            id: currentTransactionData.transactionId,
            networkType: currentTransactionData.networkType || (currentWallet ? currentWallet.networkType : 'mainnet'),
            source: 'unsigned'
        };
    }
    
    return null;
}

// Show wallet save prompt
function showWalletSavePrompt() {
    // Update the save current wallet button to be enabled
    const saveButton = document.getElementById('save-current-wallet');
    if (saveButton && window.currentGeneratedWallet) {
        saveButton.style.display = 'inline-block';
        saveButton.disabled = false;
        
        // Add visual indicator
        saveButton.style.backgroundColor = '#28a745';
        saveButton.textContent = '💾 Save Current Wallet (Ready!)';
        
        // Auto-scroll to wallet manager if visible
        const walletManagerSection = document.querySelector('#wallet-manager-container .section');
        if (walletManagerSection) {
            walletManagerSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    }
}

// Get current generated/restored wallet data
function getCurrentWalletData() {
    return window.currentGeneratedWallet || null;
}

// Save wallet with password using wallet manager
async function saveWalletWithPassword(walletData, password, label = null) {
    try {
    
        
        // Import wallet manager (this should be safe since it doesn't access DOM directly)
        // Wallet management now handled by unified wallet manager
        
        // Save the wallet
        const walletId = await walletManager.saveWallet(walletData, password, label);
        

        return walletId;
    } catch (error) {
        console.error("Error in saveWalletWithPassword:", error);
        throw error;
    }
}

// Make saveWalletWithPassword globally available
window.saveWalletWithPassword = saveWalletWithPassword;

// Export functions
export {
    initKaspa,
    getKaspa,
    isInitialized,
    setupAllEventHandlers,
    setupTransactionEventHandlers,
    getCurrentTransactionId,
    showWalletSavePrompt,
    getCurrentWalletData,
    getCurrentWallet,
    setCurrentWallet,
    downloadFile,
    saveWalletWithPassword
}; 