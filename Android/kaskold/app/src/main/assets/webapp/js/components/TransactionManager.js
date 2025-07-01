const { useState, useEffect, useRef } = React;

// Import transaction constants
import { 
  DEFAULT_TRANSACTION_AMOUNT, 
  MIN_TRANSACTION_AMOUNT, 
  MAX_TRANSACTION_AMOUNT,
  MAX_UTXOS_PER_TRANSACTION,
  UTXO_CONSOLIDATION_THRESHOLD 
} from '../../kaspa/js/constants.js';

// Import centralized serialization utilities
import { convertBigIntToString, convertStringToBigInt, serializeWasmObject } from '../../kaspa/js/serialization-utils.js';

export function TransactionManager({ walletState, onNavigate, addNotification, onGenerateChangeAddress, onGenerateNewAddress, onMarkAddressUsed, cachedUTXOs, onClearCachedUTXOs, navigationData }) {
  const [amount, setAmount] = useState(DEFAULT_TRANSACTION_AMOUNT.toString());
  const [toAddress, setToAddress] = useState('');
  const [resolvedAddress, setResolvedAddress] = useState('');
  const [domainLookupStatus, setDomainLookupStatus] = useState(null);
  const [isLookingUpDomain, setIsLookingUpDomain] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isSigning, setIsSigning] = useState(false);
  const [transactionData, setTransactionData] = useState(null);
  const [signedTransactionData, setSignedTransactionData] = useState(null);
  const [submittedTransactionData, setSubmittedTransactionData] = useState(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [qrCodeData, setQrCodeData] = useState(null);
  const [uploadedFile, setUploadedFile] = useState(null);
  const [uploadedQrData, setUploadedQrData] = useState(null);
  const [showUploadArea, setShowUploadArea] = useState(false);
  const [isScanning, setIsScanning] = useState(false);
  const [multiPartQRs, setMultiPartQRs] = useState([]);
  const [useOfflineMode, setUseOfflineMode] = useState(false);
  const transactionHandlersSetup = useRef(false);
  const fileInputRef = useRef();
  const qrInputRef = useRef();

  // Error boundary-like error handling
  useEffect(() => {
    const handleError = (error, errorInfo) => {
      console.error('TransactionManager error:', error, errorInfo);
      addNotification('A component error occurred. Please try refreshing the page.', 'error');
    };

    window.addEventListener('error', handleError);
    window.addEventListener('unhandledrejection', handleError);

    return () => {
      window.removeEventListener('error', handleError);
      window.removeEventListener('unhandledrejection', handleError);
    };
  }, [addNotification]);

  useEffect(() => {
    // Set up transaction event handlers from init.js when component mounts
    if (!transactionHandlersSetup.current) {
      setupTransactionHandlers();
      transactionHandlersSetup.current = true;
    }
  }, []);

  // Handle pre-filled data from navigation
  useEffect(() => {
    if (navigationData && navigationData.type === 'compound-utxos') {
      setAmount(navigationData.amount || '');
      setToAddress(navigationData.toAddress || '');
      setUseOfflineMode(true); // Enable offline mode for compound transactions
      
      if (navigationData.showInfo) {
        addNotification(
          `Compound transaction prepared: ${navigationData.inputCount} UTXOs → 1 UTXO. This will reduce transaction fees for future sends.`,
          'info'
        );
      }
    }
  }, [navigationData]);

  const setupTransactionHandlers = async () => {
    try {
      const { setupTransactionEventHandlers } = await import('../../kaspa/js/init.js');
      setupTransactionEventHandlers();
    } catch (error) {
      console.error('Failed to set up transaction handlers:', error);
      addNotification('Failed to initialize transaction system', 'error');
    }
  };

  // Handle .kas domain lookup
  const handleDomainLookup = async (input) => {
    const { isKasDomain, resolveDomain } = await import('../../kaspa/js/address-lookup.js');

    if (!isKasDomain(input)) {
      return null;
    }

    setIsLookingUpDomain(true);
    setDomainLookupStatus(null);

    try {
      const result = await resolveDomain(input, walletState.network);

      if (result.success) {
        setResolvedAddress(result.address);
        setDomainLookupStatus({
          type: 'success',
          message: `✅ ${input} resolved to ${result.address.substring(0, 20)}...`
        });
        return result.address;
      } else {
        setResolvedAddress('');
        setDomainLookupStatus({
          type: 'error',
          message: `❌ ${result.error}`
        });
        return null;
      }
    } catch (error) {
      setResolvedAddress('');
      setDomainLookupStatus({
        type: 'error',
        message: '❌ KNS service unavailable. Please enter a valid Kaspa address.'
      });
      return null;
    } finally {
      setIsLookingUpDomain(false);
    }
  };

  // Handle address input change
  const handleAddressChange = async (value) => {
    setToAddress(value);
    setResolvedAddress('');
    setDomainLookupStatus(null);

    if (!value.trim()) {
      return;
    }

    // Check if it's a .kas domain
    const { isKasDomain } = await import('../../kaspa/js/address-lookup.js');
    if (isKasDomain(value.trim())) {
      await handleDomainLookup(value.trim());
    }
  };

  // Validate address for current network
  const validateAddressForNetwork = async (address, network) => {
    try {
      const { getKaspa, isInitialized } = await import('../../kaspa/js/init.js');
      
      if (!isInitialized()) {
        throw new Error('Kaspa WASM not initialized');
      }

      const kaspa = getKaspa();
      const { Address } = kaspa;
      
      // Check network compatibility by looking at address prefix FIRST
      const networkPrefixes = {
        'mainnet': 'kaspa:',
        'testnet-10': 'kaspatest:',
        'testnet-11': 'kaspatest:',
        'devnet': 'kaspadev:',
        'simnet': 'kaspasim:'
      };
      
      const expectedPrefix = networkPrefixes[network] || 'kaspa:';
      
      if (!address.startsWith(expectedPrefix)) {
        const actualNetwork = Object.keys(networkPrefixes).find(net => 
          address.startsWith(networkPrefixes[net])
        ) || 'unknown';
        
        throw new Error(`Address is for ${actualNetwork} network, but ${network} is selected. Please switch to ${actualNetwork} network or use a ${network} address.`);
      }
      
      // Try to create address object to validate format
      try {
        const addressObj = new Address(address);
        return { isValid: true, address: addressObj.toString() };
      } catch (addrError) {
        throw new Error(`Invalid address format: ${addrError.message}`);
      }
      
    } catch (error) {
      return { isValid: false, error: error.message };
    }
  };

  const handleCreateTransaction = async (e) => {
    e.preventDefault();
    
    if (!amount || !toAddress) {
      addNotification('Please fill in all required fields', 'warning');
      return;
    }

    // Validate transaction amount
    const transactionAmount = parseFloat(amount);
    if (isNaN(transactionAmount) || transactionAmount <= 0) {
      addNotification('Please enter a valid amount', 'error');
      return;
    }

    if (transactionAmount < MIN_TRANSACTION_AMOUNT) {
      addNotification(`Minimum transaction amount is ${MIN_TRANSACTION_AMOUNT} KAS`, 'error');
      return;
    }

    if (transactionAmount > MAX_TRANSACTION_AMOUNT) {
      addNotification(`Maximum transaction amount is ${MAX_TRANSACTION_AMOUNT} KAS`, 'error');
      return;
    }

    if (!walletState.currentWallet) {
      addNotification('No wallet selected', 'error');
      return;
    }

    // Determine final address to use (resolved domain or direct input)
    const finalAddress = resolvedAddress || toAddress.trim();

    // Validate address for current network (use network from header dropdown)
    const currentNetwork = walletState.network;

    const addressValidation = await validateAddressForNetwork(finalAddress, currentNetwork);

    if (!addressValidation.isValid) {
      addNotification(addressValidation.error, 'error');
      return;
    }

    setIsCreating(true);
    
    try {
      let changeAddress = null;
      
      // Always generate a new change address for HD wallets (best practice for privacy)
      if (walletState.isHDWallet && onGenerateChangeAddress) {
        try {
          const changeAddressInfo = await onGenerateChangeAddress();
          changeAddress = changeAddressInfo?.address;
          
          if (!changeAddress) {
            addNotification('Warning: Could not generate new change address', 'warning');
          } else {
            addNotification('New change address generated for enhanced privacy', 'info');
          }
        } catch (changeAddressError) {
          addNotification('Warning: Failed to generate new change address - trying alternative method', 'warning');
          
          // Try to get change address directly from HD wallet
          try {
            if (walletState.hdWallet && walletState.hdWallet.getCurrentChangeAddress) {
              changeAddress = await walletState.hdWallet.getCurrentChangeAddress();
            } else {
              throw new Error('HD wallet change address method not available');
            }
          } catch (fallbackError) {
            // Final fallback to using the main address as change address
            changeAddress = walletState.address;
            console.log('Using main address as final fallback for change address');
          }
        }
      } else if (walletState.isHDWallet && !onGenerateChangeAddress) {
        addNotification('Warning: HD wallet without change address generation capability', 'warning');
        changeAddress = walletState.address;
      } else {
        // For single-address wallets, use the main address (no choice)
        changeAddress = walletState.address;
      }

      let transaction;
      
      if (useOfflineMode && cachedUTXOs) {
        // Use offline transaction creation with cached UTXOs
        const { createOfflineTransaction, validateCachedUTXOs } = await import('../../kaspa/js/transaction-create-offline.js');
        
        // Validate cached UTXOs first
        const validation = validateCachedUTXOs(cachedUTXOs, currentNetwork);
        if (!validation.valid) {
          throw new Error(validation.error);
        }
        
        // Show warning if UTXOs are old
        if (validation.warning) {
          addNotification(validation.warning, 'warning');
        }
        
        transaction = await createOfflineTransaction(
          walletState.address,
          addressValidation.address,
          parseFloat(amount),
          currentNetwork,
          cachedUTXOs,
          { 
            changeAddress: changeAddress
          }
        );
        
        if (transaction.success) {
          addNotification('Transaction created offline using cached UTXOs', 'success');
        }
      } else {
        
        let createTransaction;
        try {
          const module = await import('../../kaspa/js/transaction-create.js?t=' + Date.now());   
          createTransaction = module.createTransaction;
        } catch (importError) {
          console.error('Failed to import transaction-create module:', importError);
          throw new Error('Failed to import transaction creation module: ' + importError.message);
        }        
        
        try {
          transaction = await createTransaction(
            walletState.address,
            addressValidation.address,
            parseFloat(amount),
            currentNetwork,
            { 
              changeAddress: changeAddress,
              hdWallet: walletState.hdWallet
            }
          );
        } catch (funcError) {
          console.error('Error calling createTransaction:', funcError);          
          throw funcError;
        }
        
        if (transaction.success) {
          addNotification('Transaction created successfully', 'success');
        }
      }



      if (transaction && transaction.success) {
        // The transaction data is in the transaction object itself, not nested under 'data'
        setTransactionData(transaction);
        setSignedTransactionData(null); // Clear any previous signed transaction
        await generateQRCode(transaction, 'unsigned');
      } else if (transaction && transaction.error) {
        throw new Error(transaction.error);
      } else if (transaction && transaction.transactionId) {
        // Handle case where transaction is returned directly without success wrapper
        setTransactionData(transaction);
        setSignedTransactionData(null); // Clear any previous signed transaction
        await generateQRCode(transaction, 'unsigned');
      } else {
        throw new Error('Transaction creation failed - no valid response received');
      }
      
    } catch (error) {
      console.error('Transaction creation failed:', error);
      console.error('Error details:', {
        message: error.message,
        stack: error.stack,
        name: error.name,
        error: error
      });
      addNotification('Transaction creation failed: ' + (error.message || 'Unknown error'), 'error');
    } finally {
      setIsCreating(false);
    }
  };

  const handleSignTransaction = async () => {
    if (!transactionData) {
      addNotification('No transaction to sign', 'warning');
      return;
    }

    setIsSigning(true);

    try {


      const { signTransaction } = await import('../../kaspa/js/transaction-sign.js?t=' + Date.now());
      
      let result;
      
      if (walletState.isHDWallet && walletState.hdWallet) {
        // Extract input addresses from the transaction data
        let inputAddresses = [];
        
        if (transactionData.summary?.utxos && Array.isArray(transactionData.summary.utxos)) {
          inputAddresses = [...new Set(transactionData.summary.utxos.map(utxo => utxo.address))];
        } else if (transactionData.inputs && Array.isArray(transactionData.inputs)) {
          inputAddresses = [...new Set(transactionData.inputs.map(input => input.previous_outpoint.address))];
        }
        
        // For uploaded transactions, we might need to use current wallet addresses instead
        if (inputAddresses.length === 0 || transactionData.isUploaded) {
          console.log('🔄 Using current wallet addresses for uploaded/reconstructed transaction');
          const allWalletAddresses = walletState.hdWallet.getAllAddresses();
          
          if (!Array.isArray(allWalletAddresses)) {
            throw new Error('HD wallet getAllAddresses() did not return an array');
          }
          
          const addressesWithBalance = allWalletAddresses
            .filter(addr => addr.balance > 0n)
            .map(addr => addr.address);
          
          if (addressesWithBalance.length === 0) {
            // If no addresses with balance, use all addresses
            inputAddresses = allWalletAddresses.map(addr => addr.address);
            console.log('⚠️ No addresses with balance found, using all addresses:', inputAddresses);
          } else {
            inputAddresses = addressesWithBalance;
            console.log('✅ Using addresses with balance:', inputAddresses);
          }
        } else {
          console.log('Using input addresses from transaction:', inputAddresses);
        }
        
        const privateKeys = {};
        
        // Get all addresses with their private keys from HD wallet
        const allAddresses = walletState.hdWallet.getAllAddresses();
        
        if (!Array.isArray(allAddresses)) {
          throw new Error('HD wallet getAllAddresses() did not return an array');
        }
        
        console.log('Available addresses in HD wallet:', allAddresses.map(addr => ({ 
          address: addr.address, 
          hasPrivateKey: !!addr.privateKey,
          balance: addr.balance?.toString() || '0'
        })));
        
        for (const address of inputAddresses) {
          try {
            // Find the address info object that contains the private key
            const addressInfo = allAddresses.find(addr => addr.address === address);
            
            if (addressInfo && addressInfo.privateKey) {
              privateKeys[address] = addressInfo.privateKey;
            } else {
              console.warn('Could not find private key for address:', address);
            }
          } catch (error) {
            console.warn('Could not get private key for address:', address, error.message);
          }
        }
        
        // Also add any addresses with private keys that have balance, even if not in inputAddresses
        for (const addressInfo of allAddresses) {
          if (addressInfo.privateKey && addressInfo.balance > 0n && !privateKeys[addressInfo.address]) {
            console.log('➕ Adding address with balance to available keys:', addressInfo.address);
            privateKeys[addressInfo.address] = addressInfo.privateKey;
          }
        }
        
        // Check if we have any valid private keys
        const validPrivateKeys = Object.keys(privateKeys);
        if (validPrivateKeys.length === 0) {
          throw new Error('No valid private keys found for transaction signing. Please ensure your HD wallet has addresses with private keys and funds.');
        }
        
        console.log(`Found private keys for ${validPrivateKeys.length} addresses:`, validPrivateKeys);
        
        // Add diagnostic check for uploaded transactions
        if (transactionData.isUploaded) {
          console.log('🔍 DIAGNOSTIC: Checking wallet status for uploaded transaction signing...');
          
          // Check each address for UTXOs via wallet state
          for (const address of validPrivateKeys) {
            const addressInfo = allAddresses.find(addr => addr.address === address);
            if (addressInfo) {
              console.log(`📊 Address ${address}:`, {
                balance: addressInfo.balance?.toString() || '0',
                hasPrivateKey: !!addressInfo.privateKey,
                utxoCount: addressInfo.utxos?.length || 0
              });
            }
          }
          
          // Check if we have cached UTXOs available
          if (cachedUTXOs) {
            console.log('💾 Cached UTXOs available:', {
              count: cachedUTXOs.count,
              timestamp: new Date(cachedUTXOs.timestamp).toLocaleString(),
              addresses: (cachedUTXOs.utxos && Array.isArray(cachedUTXOs.utxos)) ? [...new Set(cachedUTXOs.utxos.map(u => u.address))] : []
            });
          } else {
            console.log('❌ No cached UTXOs available - this may cause network lookup');
          }
          
          addNotification('Signing uploaded transaction - checking for available UTXOs...', 'info');
        }
        
        // Check if this is an uploaded transaction that needs special handling
        if (transactionData.isUploaded && transactionData.serializedPendingTransaction) {
          console.log('🔄 Signing uploaded transaction with serialized pending transaction data');
          
          // For uploaded transactions, we need to reconstruct the pending transaction
          // Try to restore the WASM pending transaction object
          try {
            const { getKaspa } = await import('../../kaspa/js/init.js');
            const kaspa = getKaspa();
            
            // If we have the serialized pending transaction, try to reconstruct it
            if (transactionData.originalTransactionData?.pendingTransaction) {
              const pendingTxData = transactionData.originalTransactionData.pendingTransaction;
              
              // Create a modified transaction data structure for signing
              const signingTransactionData = {
                ...transactionData,
                pendingTransaction: pendingTxData,
                // Include any additional data needed for signing
                summary: transactionData.originalTransactionData?.summary
              };
              
              result = await signTransaction(signingTransactionData, privateKeys);
            } else {
              // Fallback to original method
              result = await signTransaction(transactionData, privateKeys);
            }
          } catch (reconstructError) {
            console.warn('Could not reconstruct WASM transaction, using original data:', reconstructError);
            result = await signTransaction(transactionData, privateKeys);
          }
        } else {
          // Normal transaction signing
          result = await signTransaction(transactionData, privateKeys);
        }
      } else {
        // For single address wallets
        if (!walletState.currentWallet || !walletState.currentWallet.privateKey) {
          addNotification('No private key available for signing', 'error');
          return;
        }
        
        result = await signTransaction(transactionData, walletState.currentWallet.privateKey);
      }

      if (result && result.success) {
        // Create comprehensive signed transaction data structure
        let serializedTransaction = null;
        
        // Serialize the signed transaction using centralized utilities
        if (result.signedTransaction) {
          try {
            console.log('✅ Serializing signed transaction using centralized utilities');
            serializedTransaction = serializeWasmObject(result.signedTransaction);
          } catch (serializeError) {
            console.warn('⚠️ Could not serialize signed transaction:', serializeError);
          }
        }
        
        const signedTxData = {
          ...transactionData, // Include original transaction data
          ...result, // Include signing result
          status: 'signed',
          signedAt: new Date().toISOString(),
          // Ensure we have serialized transaction data for submission
          serializedTransaction: serializedTransaction
        };

        console.log('🔍 Created signed transaction data with fields:', Object.keys(signedTxData));
        
        setSignedTransactionData(signedTxData);
        await generateQRCode(signedTxData, 'signed');
        addNotification('Transaction signed successfully', 'success');
      } else {
        throw new Error(result?.error || 'Transaction signing failed');
      }

    } catch (error) {
      console.error('Transaction signing failed:', error);
      
      // Check if this is a UTXO-related error for uploaded transactions
      if (transactionData?.isUploaded && error.message.includes('No UTXOs found')) {
        // Offer to refresh wallet state
        const shouldRefresh = confirm(
          'No UTXOs found for transaction signing. This might be because:\n\n' +
          '• The wallet needs to refresh its UTXO data\n' +
          '• The transaction was created with different addresses\n' +
          '• Network connectivity issues\n\n' +
          'Would you like to go back to the wallet dashboard to refresh the wallet state and check for available funds?'
        );
        
        if (shouldRefresh) {
          addNotification('Redirecting to wallet dashboard to refresh UTXO data...', 'info');
          onNavigate('wallet-dashboard');
          return;
        }
      }
      
      addNotification('Transaction signing failed: ' + error.message, 'error');
    } finally {
      setIsSigning(false);
    }
  };

  const handleSubmitTransaction = async () => {
    if (!signedTransactionData) {
      addNotification('No signed transaction to submit', 'warning');
      return;
    }

    setIsSubmitting(true);

    try {


      const { submitTransaction } = await import('../../kaspa/js/transaction-submit.js');
      
      const result = await submitTransaction(signedTransactionData, walletState.network);



      if (result && result.success) {
        // Create submitted transaction data structure
        const submittedTxData = {
          ...signedTransactionData, // Include signed transaction data
          ...result, // Include submission result
          status: 'submitted',
          submittedAt: new Date().toISOString()
        };
        
        setSubmittedTransactionData(submittedTxData);
        
        // Safely generate QR code with defensive data handling
        try {
          await generateQRCode(submittedTxData, 'submitted');
        } catch (qrError) {
          console.error('QR generation failed for submitted transaction:', qrError);
          addNotification('Transaction submitted successfully, but QR generation failed: ' + qrError.message, 'warning');
        }
        
        // Mark addresses as used for HD wallets
        if (walletState.isHDWallet && onMarkAddressUsed) {
          // Mark input addresses as used
          if (Array.isArray(submittedTxData.inputs)) {
            submittedTxData.inputs.forEach(input => {
              if (input && input.address) {
                onMarkAddressUsed(input.address);
              }
            });
          }
          
          // Mark change address as used if it exists
          if (Array.isArray(submittedTxData.outputs)) {
            submittedTxData.outputs.forEach(output => {
              if (output && output.address && output.address !== toAddress) {
                // This is likely the change address
                onMarkAddressUsed(output.address);
              }
            });
          }
        }
        
        addNotification('Transaction submitted successfully to the network', 'success');
        
        // Generate new receive address for enhanced privacy (HD wallets only)
        if (walletState.isHDWallet && onGenerateNewAddress) {
          try {
            await onGenerateNewAddress();
            addNotification('New receive address generated for enhanced privacy', 'info');
          } catch (error) {
            console.warn('Failed to generate new receive address:', error);
          }
        }
      } else {
        throw new Error(result?.error || 'Transaction submission failed');
      }

    } catch (error) {
      console.error('Transaction submission failed:', error);
      addNotification('Transaction submission failed: ' + error.message, 'error');
    } finally {
      setIsSubmitting(false);
    }
  };

  const generateQRCode = async (txData, type = 'unsigned') => {
    try {
      let qrFunction;
      if (type === 'signed') {
        qrFunction = 'generateSignedTransactionQR';
      } else if (type === 'submitted') {
        qrFunction = 'generateSubmittedTransactionQR';
      } else {
        qrFunction = 'generateUnsignedTransactionQR';
      }
      
      const { [qrFunction]: generateQR } = await import('../../kaspa/js/qr-manager.js');
      
      // Convert BigInt values to strings before generating QR code
      const serializedTxData = convertBigIntToString(txData);
      const qrResult = await generateQR(serializedTxData);
      
      if (qrResult.success) {
        // Handle both single and multi-part QR codes
        let normalizedQRData;
        
        if (qrResult.isMultiPart) {
          // For multi-part QRs, use the first part for display and store all parts
          normalizedQRData = {
            ...qrResult,
            type,
            qrDataURL: qrResult.qrParts[0].qrDataURL, // Show first part by default
            size: qrResult.originalSize,
            currentPart: 1,
            displayPart: qrResult.qrParts[0]
          };
        } else {
          // Single QR code
          normalizedQRData = {
            ...qrResult,
            type,
            isMultiPart: false,
            totalParts: 1,
            currentPart: 1
          };
        }
        
        setQrCodeData(normalizedQRData);
        console.log('✅ QR code generated successfully:', normalizedQRData);
      } else {
        console.error('QR generation failed:', qrResult.error);
        addNotification('Failed to generate QR code: ' + qrResult.error, 'warning');
      }
    } catch (error) {
      console.error('QR generation error:', error);
      addNotification('Failed to generate QR code: ' + error.message, 'warning');
    }
  };

  const handleDownloadQR = () => {
    if (!qrCodeData) {
      addNotification('No QR code available for download', 'error');
      return;
    }
    
    try {
      const transactionType = submittedTransactionData ? 'submitted' : signedTransactionData ? 'signed' : 'unsigned';
      const txId = (submittedTransactionData || signedTransactionData || transactionData)?.transactionId || 'unknown';
      
      if (qrCodeData.isMultiPart) {
        // Download current displayed part for multi-part QRs
        const currentPart = qrCodeData.currentPart || 1;
        const currentQRPart = qrCodeData.qrParts[currentPart - 1];
        
        if (!currentQRPart || !currentQRPart.qrDataURL) {
          addNotification('QR code data not available for download', 'error');
          return;
        }
        
        const filename = `kaspa-${transactionType}-transaction-${txId}-part${currentPart}of${qrCodeData.totalParts}.png`;
        
        // Create download link
        const link = document.createElement('a');
        link.href = currentQRPart.qrDataURL;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        
        addNotification(`Downloaded part ${currentPart} of ${qrCodeData.totalParts} for ${transactionType} transaction QR`, 'success');
      } else {
        // Single QR code download
        if (!qrCodeData.qrDataURL) {
          addNotification('QR code data not available for download', 'error');
          return;
        }
        
        const filename = `kaspa-${transactionType}-transaction-${txId}.png`;
        
        // Create download link
        const link = document.createElement('a');
        link.href = qrCodeData.qrDataURL;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        
        addNotification(`${transactionType} transaction QR code downloaded`, 'success');
      }
    } catch (error) {
      console.error('Download failed:', error);
      addNotification('Failed to download QR code: ' + error.message, 'error');
    }
  };

  const handleDownloadAllQRParts = async () => {
    if (!qrCodeData || !qrCodeData.isMultiPart) {
      addNotification('No multi-part QR code available', 'error');
      return;
    }
    
    try {
      const transactionType = submittedTransactionData ? 'submitted' : signedTransactionData ? 'signed' : 'unsigned';
      const txId = (submittedTransactionData || signedTransactionData || transactionData)?.transactionId || 'unknown';
      
      // Download each part sequentially with a small delay
      for (let i = 0; i < qrCodeData.qrParts.length; i++) {
        const part = qrCodeData.qrParts[i];
        const filename = `kaspa-${transactionType}-transaction-${txId}-part${i + 1}of${qrCodeData.totalParts}.png`;
        
        const link = document.createElement('a');
        link.href = part.qrDataURL;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        
        // Small delay between downloads to avoid overwhelming the browser
        if (i < qrCodeData.qrParts.length - 1) {
          await new Promise(resolve => setTimeout(resolve, 100));
        }
      }
      
      addNotification(`Downloaded all ${qrCodeData.totalParts} parts of ${transactionType} transaction QR`, 'success');
    } catch (error) {
      console.error('Download all parts failed:', error);
      addNotification('Failed to download all QR parts: ' + error.message, 'error');
    }
  };

  // Conversion functions now imported from centralized utilities

  const handleDownloadJSON = () => {
    const currentTxData = submittedTransactionData || signedTransactionData || transactionData;
    if (!currentTxData) {
      addNotification('No transaction data available for download', 'error');
      return;
    }
    
    try {
      const transactionType = submittedTransactionData ? 'submitted' : signedTransactionData ? 'signed' : 'unsigned';
      
      // Convert BigInt values to strings before JSON.stringify
      const serializedData = convertBigIntToString(currentTxData);
      const jsonData = JSON.stringify(serializedData, null, 2);
      const blob = new Blob([jsonData], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      
      const link = document.createElement('a');
      link.href = url;
      link.download = `kaspa-${transactionType}-transaction-${currentTxData.transactionId}.json`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      
      URL.revokeObjectURL(url);
      addNotification(`${transactionType} transaction JSON downloaded`, 'success');
    } catch (error) {
      console.error('JSON download failed:', error);
      addNotification('Failed to download JSON: ' + error.message, 'error');
    }
  };

  const handleFileUpload = async (event) => {
    const file = event.target.files[0];
    if (!file) return;

    try {
      const text = await file.text();
      const data = JSON.parse(text);
      
      // Detect transaction type BEFORE BigInt conversion (since detectTransactionType expects string values)
      const transactionType = detectTransactionType(data);
      
      if (transactionType === 'unknown') {
        throw new Error('Unrecognized transaction format. Please ensure this is a valid Kaspa transaction file.');
      }
      
      // Convert string values back to BigInt where needed AFTER type detection
      const processedData = convertStringToBigInt(data);
      
      // Reset transaction progress states based on uploaded transaction type
      if (transactionType === 'unsigned') {
        setTransactionData(processedData);
        setSignedTransactionData(null);
        setSubmittedTransactionData(null);
        await generateQRCode(processedData, 'unsigned');
      } else if (transactionType === 'signed') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData(processedData);
        setSubmittedTransactionData(null);
        await generateQRCode(processedData, 'signed');
      } else if (transactionType === 'submitted') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData(null); // Clear signed data
        setSubmittedTransactionData(processedData);
        await generateQRCode(processedData, 'submitted');
      }
      
      setUploadedFile({ file, data: processedData, type: transactionType });
      setUploadedQrData(null); // Clear QR data if file was uploaded
      addNotification(`${transactionType} transaction uploaded successfully`, 'success');
      
    } catch (error) {
      console.error('File upload failed:', error);
      addNotification('Failed to parse transaction file: ' + error.message, 'error');
    }
    
    event.target.value = '';
  };

  const handleQRUpload = async (event) => {
    const files = Array.from(event.target.files);
    if (files.length === 0) return;

    try {
      // If multiple files, try multi-part QR processing
      if (files.length > 1) {
        const { readMultiPartQRFromImages } = await import('../../kaspa/js/qr-manager.js');
        
        const result = await readMultiPartQRFromImages(files);
        
        if (result.success) {
          const qrData = result.qrData;

          // Detect transaction type (handle both single and multi-part QR formats)
          let transactionType = 'unknown';
          if (qrData.type === 'kaspa-unsigned-transaction-qr' || qrData.type === 'kaspa-unsigned-transaction-multipart-qr') {
            transactionType = 'unsigned';
          } else if (qrData.type === 'kaspa-signed-transaction-qr' || qrData.type === 'kaspa-signed-transaction-multipart-qr') {
            transactionType = 'signed';
          } else if (qrData.type === 'kaspa-submitted-transaction-qr' || qrData.type === 'kaspa-submitted-transaction-multipart-qr') {
            transactionType = 'submitted';
          }

          if (transactionType === 'unknown') {
            throw new Error(`Unrecognized QR transaction format. Found type: ${qrData.type || 'undefined'}`);
          }

          // Convert string values back to BigInt where needed
          const processedQrData = convertStringToBigInt(qrData);

          // Reset transaction progress states
          if (transactionType === 'unsigned') {
            const uploadedTransactionData = {
              ...processedQrData,
              isUploaded: true
            };
            setTransactionData(uploadedTransactionData);
            setSignedTransactionData(null);
            setSubmittedTransactionData(null);
            await generateQRCode(uploadedTransactionData, 'unsigned');
          } else if (transactionType === 'signed') {
            setTransactionData(null);
            const uploadedSignedData = {
              ...processedQrData,
              isUploaded: true
            };
            setSignedTransactionData(uploadedSignedData);
            setSubmittedTransactionData(null);
            await generateQRCode(uploadedSignedData, 'signed');
          } else if (transactionType === 'submitted') {
            setTransactionData(null);
            setSignedTransactionData(null);
            const uploadedSubmittedData = {
              ...processedQrData,
              isUploaded: true
            };
            setSubmittedTransactionData(uploadedSubmittedData);
            await generateQRCode(uploadedSubmittedData, 'submitted');
          }

          setUploadedQrData({ 
            source: 'multipart-files', 
            data: processedQrData, 
            type: transactionType,
            files: files,
            totalParts: result.totalParts
          });
          setUploadedFile(null);
          
          addNotification(`${transactionType} transaction uploaded from ${files.length} multi-part QR images`, 'success');
          event.target.value = '';
          return;
        } else {
          // Fall through to single QR processing
        }
      }

      // Single QR processing (original logic)
      const file = files[0];
      const { readQRFromImage, validateTransactionQRData } = await import('../../kaspa/js/qr-manager.js');
      
      const qrResult = await readQRFromImage(file);
      
      if (!qrResult.success) {
        throw new Error(qrResult.error || 'Failed to read QR code');
      }
      
      let qrData;
      try {
        qrData = JSON.parse(qrResult.data);
      } catch (parseError) {
        console.error('QR data parsing failed:', parseError.message);
        throw new Error('Invalid QR code data format');
      }
      
      // Detect QR transaction type (handle both single and multi-part QR formats)
      let transactionType = 'unknown';
      if (qrData.type === 'kaspa-unsigned-transaction-qr' || qrData.type === 'kaspa-unsigned-transaction-multipart-qr') {
        transactionType = 'unsigned';
      } else if (qrData.type === 'kaspa-signed-transaction-qr' || qrData.type === 'kaspa-signed-transaction-multipart-qr') {
        transactionType = 'signed';
      } else if (qrData.type === 'kaspa-submitted-transaction-qr' || qrData.type === 'kaspa-submitted-transaction-multipart-qr') {
        transactionType = 'submitted';
      }
      
      if (transactionType === 'unknown') {
        throw new Error(`Unrecognized QR transaction format. Found type: ${qrData.type || 'undefined'}`);
      }
      
      // Convert string values back to BigInt where needed
      const processedQrData = convertStringToBigInt(qrData);
      
      // Validate QR data
      const validation = validateTransactionQRData(processedQrData, `${transactionType}-transaction`);
      if (!validation.isValid) {
        throw new Error(validation.error);
      }
      
      // Reset transaction progress states based on uploaded QR transaction type
      if (transactionType === 'unsigned') {
        // Mark as uploaded and include serialized transaction data
        const uploadedTransactionData = {
          ...processedQrData,
          isUploaded: true
        };
        setTransactionData(uploadedTransactionData);
        setSignedTransactionData(null);
        setSubmittedTransactionData(null);
        setQrCodeData({ qrDataURL: URL.createObjectURL(file), size: file.size, type: 'unsigned' });
      } else if (transactionType === 'signed') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData({
          ...processedQrData,
          isUploaded: true
        });
        setSubmittedTransactionData(null);
        setQrCodeData({ qrDataURL: URL.createObjectURL(file), size: file.size, type: 'signed' });
      } else if (transactionType === 'submitted') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData(null); // Clear signed data
        setSubmittedTransactionData({
          ...processedQrData,
          isUploaded: true
        });
        setQrCodeData({ qrDataURL: URL.createObjectURL(file), size: file.size, type: 'submitted' });
      }
      
      setUploadedQrData({ file, data: processedQrData, type: transactionType });
      setUploadedFile(null); // Clear file data if QR was uploaded
      addNotification(`${transactionType} transaction QR uploaded successfully`, 'success');
      
    } catch (error) {
      console.error('QR upload failed:', error);
      addNotification('Failed to read QR code: ' + error.message, 'error');
    }
    
    event.target.value = '';
  };

  const handleCameraQRScan = async () => {
    setIsScanning(true);
    
    try {
      const { openCameraQRScanner } = await import('../../kaspa/js/qr-manager.js');
      
      await openCameraQRScanner(async (qrResult) => {
        try {
          if (!qrResult.success) {
            throw new Error(qrResult.error || 'Failed to scan QR code');
          }

          const qrData = qrResult.qrData;

          // Detect transaction type (handle both single and multi-part QR formats)
          let transactionType = 'unknown';
          if (qrData.type === 'kaspa-unsigned-transaction-qr' || qrData.type === 'kaspa-unsigned-transaction-multipart-qr') {
            transactionType = 'unsigned';
          } else if (qrData.type === 'kaspa-signed-transaction-qr' || qrData.type === 'kaspa-signed-transaction-multipart-qr') {
            transactionType = 'signed';
          } else if (qrData.type === 'kaspa-submitted-transaction-qr' || qrData.type === 'kaspa-submitted-transaction-multipart-qr') {
            transactionType = 'submitted';
          }

          if (transactionType === 'unknown') {
            throw new Error(`Unrecognized QR transaction format. Found type: ${qrData.type || 'undefined'}`);
          }

          // Convert string values back to BigInt where needed
          const processedQrData = convertStringToBigInt(qrData);

          // Reset transaction progress states based on scanned QR transaction type
          if (transactionType === 'unsigned') {
            const scannedTransactionData = {
              ...processedQrData,
              isUploaded: true
            };
            setTransactionData(scannedTransactionData);
            setSignedTransactionData(null);
            setSubmittedTransactionData(null);
            await generateQRCode(scannedTransactionData, 'unsigned');
          } else if (transactionType === 'signed') {
            setTransactionData(null);
            const scannedSignedData = {
              ...processedQrData,
              isUploaded: true
            };
            setSignedTransactionData(scannedSignedData);
            setSubmittedTransactionData(null);
            await generateQRCode(scannedSignedData, 'signed');
          } else if (transactionType === 'submitted') {
            setTransactionData(null);
            setSignedTransactionData(null);
            const scannedSubmittedData = {
              ...processedQrData,
              isUploaded: true
            };
            setSubmittedTransactionData(scannedSubmittedData);
            await generateQRCode(scannedSubmittedData, 'submitted');
          }

          // Clear file uploads since we scanned from camera
          setUploadedFile(null);
          setUploadedQrData({ 
            source: 'camera', 
            data: processedQrData, 
            type: transactionType,
            isMultiPart: qrResult.isMultiPart,
            totalParts: qrResult.totalParts || 1
          });

          const scanMessage = qrResult.isMultiPart ? 
            `${transactionType} transaction QR scanned from camera (${qrResult.totalParts} parts combined)` :
            `${transactionType} transaction QR scanned from camera`;
          
          addNotification(scanMessage, 'success');

        } catch (error) {
          console.error('Error processing scanned QR:', error);
          addNotification('Error processing scanned QR: ' + error.message, 'error');
        }
      });

    } catch (error) {
      console.error('Error opening camera scanner:', error);
      addNotification('Error opening camera scanner: ' + error.message, 'error');
    } finally {
      setIsScanning(false);
    }
  };

  const handleScanRecipientAddress = async () => {
    try {
      setIsScanning(true);
      
      // Import QR manager
      const { openCameraQRScanner } = await import('../../kaspa/js/qr-manager.js');
      
      // Open camera scanner with callback
      await openCameraQRScanner(async (scanResult) => {
        if (scanResult.success && scanResult.qrData) {
          const qrData = scanResult.qrData;
          
          // Check if it's a simple address string or .kas domain
          if (typeof qrData === 'string') {
            if (qrData.startsWith('kaspa:') || qrData.startsWith('kaspatest:')) {
              setToAddress(qrData);
              addNotification('Recipient address scanned successfully', 'success');
              return;
            } else if (qrData.endsWith('.kas')) {
              await handleAddressChange(qrData);
              addNotification('Domain scanned, resolving address...', 'info');
              return;
            }
          }
          
          // Check if it's a Kaspa address QR (receiving address)
          if (qrData.type === 'kaspa-address-qr' && qrData.address) {
            setToAddress(qrData.address);
            addNotification('Recipient address scanned successfully', 'success');
            return;
          }
          
          // Check if it's a payment request QR
          if (qrData.type === 'kaspa-payment-request-qr' && qrData.address) {
            setToAddress(qrData.address);
            if (qrData.amount) {
              setAmount(qrData.amount.toString());
              addNotification(`Payment request scanned: ${qrData.amount} KAS to ${qrData.address}`, 'success');
            } else {
              addNotification('Recipient address scanned successfully', 'success');
            }
            return;
          }
          
          // If we get here, it's not a recognized address format
          addNotification('QR code does not contain a valid Kaspa address', 'warning');
        } else {
          addNotification('Failed to scan QR code: ' + (scanResult.error || 'Unknown error'), 'error');
        }
      });
      
    } catch (error) {
      console.error('Error opening camera scanner for recipient address:', error);
      addNotification('Error opening camera scanner: ' + error.message, 'error');
    } finally {
      setIsScanning(false);
    }
  };

  const handleUploadRecipientQR = () => {
    // Create a file input element
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.accept = 'image/*';
    fileInput.multiple = false;
    
    fileInput.onchange = async (event) => {
      const file = event.target.files[0];
      if (!file) return;
      
      try {
        // Import QR reading function
        const { readQRFromImage } = await import('../../kaspa/js/qr-manager.js');
        
        // Read QR code from uploaded image
        const qrResult = await readQRFromImage(file);
        
        if (qrResult.success && qrResult.qrData) {
          const qrData = qrResult.qrData;
          
          // Check if it's a simple address string
          if (typeof qrData === 'string' && (qrData.startsWith('kaspa:') || qrData.startsWith('kaspatest:'))) {
            setToAddress(qrData);
            addNotification('Recipient address loaded from QR image', 'success');
            return;
          }
          
          // Check if it's a Kaspa address QR (receiving address)
          if (qrData.type === 'kaspa-address-qr' && qrData.address) {
            setToAddress(qrData.address);
            addNotification('Recipient address loaded from QR image', 'success');
            return;
          }
          
          // Check if it's a payment request QR
          if (qrData.type === 'kaspa-payment-request-qr' && qrData.address) {
            setToAddress(qrData.address);
            if (qrData.amount) {
              setAmount(qrData.amount.toString());
              addNotification(`Payment request loaded: ${qrData.amount} KAS to ${qrData.address}`, 'success');
            } else {
              addNotification('Recipient address loaded from QR image', 'success');
            }
            return;
          }
          
          // If we get here, it's not a recognized address format
          addNotification('QR image does not contain a valid Kaspa address', 'warning');
        } else {
          addNotification('Failed to read QR code from image: ' + (qrResult.error || 'Unknown error'), 'error');
        }
        
      } catch (error) {
        console.error('Error reading QR from uploaded image:', error);
        addNotification('Error reading QR code: ' + error.message, 'error');
      }
    };
    
    // Trigger file selection
    fileInput.click();
  };

  const handleResetTransaction = () => {
    // Reset all transaction states
    setTransactionData(null);
    setSignedTransactionData(null);
    setSubmittedTransactionData(null);
    setQrCodeData(null);
    setUploadedFile(null);
    setUploadedQrData(null);
    setMultiPartQRs([]);
    
    // Reset form
    setAmount('');
    setToAddress('');
    
    // Close upload area
    setShowUploadArea(false);
    
    addNotification('Transaction progress reset', 'info');
  };

  const detectTransactionType = (data) => {
    // Check for explicit status field first
    if (data.status) {
      if (data.status === 'unsigned') return 'unsigned';
      if (data.status === 'signed') return 'signed';
      if (data.status === 'submitted') return 'submitted';
    }
    
    // Check for transaction signatures (signed transaction)
    if (data.signature || data.signatures || (data.inputs && data.inputs.some(input => input.signature))) {
      return 'signed';
    }
    
    // Check for submitted transaction indicators
    if (data.txId || data.submittedTxId || data.networkTxId) {
      return 'submitted';
    }
    
    // Check for basic transaction structure (unsigned)
    if (data.transactionId && (data.inputs || data.outputs || data.toAddress)) {
      return 'unsigned';
    }
    
    // Check for QR-specific transaction types
    if (data.type) {
      if (data.type.includes('unsigned')) return 'unsigned';
      if (data.type.includes('signed')) return 'signed';
      if (data.type.includes('submitted')) return 'submitted';
    }
    
    return 'unknown';
  };

  const validateForm = () => {
    const errors = [];
    
    if (!amount || isNaN(amount) || parseFloat(amount) <= 0) {
      errors.push('Valid amount is required');
    }
    
    if (!toAddress || toAddress.length < 10) {
      errors.push('Valid recipient address is required');
    }
    
    return errors;
  };

  const formErrors = validateForm();

  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row' },
      // Main transaction form
      React.createElement('div', { className: 'col-lg-8' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header d-flex justify-content-between align-items-center' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-send me-2' }),
              'Transaction Manager'
            ),
            React.createElement('div', { className: 'btn-group' },
              React.createElement('button', {
                className: 'btn btn-outline-primary btn-sm',
                onClick: () => setShowUploadArea(!showUploadArea)
              },
                React.createElement('i', { className: 'bi bi-upload me-1' }),
                'Upload Transaction'
              ),
              (transactionData || signedTransactionData || submittedTransactionData) && React.createElement('button', {
                className: 'btn btn-outline-secondary btn-sm',
                onClick: handleResetTransaction,
                title: 'Reset transaction progress'
              },
                React.createElement('i', { className: 'bi bi-arrow-clockwise me-1' }),
                'Reset'
              )
            )
          ),
          React.createElement('div', { className: 'card-body' },
            
            // Upload area
            showUploadArea && React.createElement('div', { className: 'alert alert-info mb-4' },
              React.createElement('h6', { className: 'alert-heading' },
                React.createElement('i', { className: 'bi bi-upload me-2' }),
                'Upload or Scan Transaction'
              ),
              React.createElement('p', { className: 'mb-3' }, 
                'Upload transaction files, QR code images, or use your camera to scan QR codes. Supports multi-part QR codes for large transactions.'
              ),
              React.createElement('div', { className: 'row g-2' },
                React.createElement('div', { className: 'col-md-6' },
                  React.createElement('label', { className: 'form-label small' }, 'Upload JSON File'),
                  React.createElement('input', {
                    ref: fileInputRef,
                    type: 'file',
                    className: 'form-control form-control-sm',
                    accept: '.json',
                    onChange: handleFileUpload
                  })
                ),
                React.createElement('div', { className: 'col-md-6' },
                  React.createElement('label', { className: 'form-label small' }, 'Upload QR Code Image(s)'),
                  React.createElement('input', {
                    ref: qrInputRef,
                    type: 'file',
                    className: 'form-control form-control-sm',
                    accept: 'image/*',
                    multiple: true,
                    onChange: handleQRUpload,
                    title: 'Select single image or multiple images for multi-part QR codes'
                  })
                )
              ),
              React.createElement('div', { className: 'row g-2 mt-2' },
                React.createElement('div', { className: 'col-12 text-center' },
                  React.createElement('button', {
                    className: `btn btn-outline-primary btn-sm ${isScanning ? 'disabled' : ''}`,
                    onClick: handleCameraQRScan,
                    disabled: isScanning
                  },
                    isScanning ? 
                      React.createElement('span', null,
                        React.createElement('span', { 
                          className: 'spinner-border spinner-border-sm me-2' 
                        }),
                        'Opening Camera...'
                      ) :
                      React.createElement('span', null,
                        React.createElement('i', { className: 'bi bi-camera me-2' }),
                        'Scan QR with Camera'
                      )
                  )
                )
              ),
              (uploadedFile || uploadedQrData) && React.createElement('div', { className: 'mt-3 p-2 bg-success bg-opacity-10 border border-success rounded' },
                React.createElement('small', { className: 'text-success' },
                  React.createElement('i', { className: 'bi bi-check-circle me-1' }),
                  uploadedFile ? 
                    `File uploaded: ${uploadedFile.type} transaction (${uploadedFile.file.name})` :
                    uploadedQrData.source === 'camera' ?
                      `Camera scan: ${uploadedQrData.type} transaction${uploadedQrData.isMultiPart ? ` (${uploadedQrData.totalParts} parts)` : ''}` :
                      uploadedQrData.source === 'multipart-files' ?
                        `Multi-part QR: ${uploadedQrData.type} transaction (${uploadedQrData.totalParts} parts from ${uploadedQrData.files.length} files)` :
                        uploadedQrData.file ?
                          `QR uploaded: ${uploadedQrData.type} transaction (${uploadedQrData.file.name})` :
                          `QR uploaded: ${uploadedQrData.type} transaction`
                )
              )
            ),
            
            // UTXO Status and Offline Mode Section
            React.createElement('div', { className: 'mb-4 p-3 border rounded bg-light' },
              React.createElement('div', { className: 'row align-items-center' },
                React.createElement('div', { className: 'col-md-8' },
                  React.createElement('h6', { className: 'mb-2' },
                    React.createElement('i', { className: 'bi bi-database me-2' }),
                    'Transaction Mode'
                  ),
                  cachedUTXOs ? 
                    React.createElement('div', null,
                      React.createElement('span', { className: 'badge bg-success me-2' },
                        React.createElement('i', { className: 'bi bi-check-circle me-1' }),
                        'UTXOs Available'
                      ),
                      React.createElement('small', { className: 'text-muted' },
                        `${cachedUTXOs.count} UTXOs cached | Last updated: ${new Date(cachedUTXOs.timestamp).toLocaleString()}`
                      ),
                      cachedUTXOs.imported && React.createElement('span', { className: 'badge bg-info ms-2' }, 'Imported from QR')
                    ) :
                    React.createElement('div', null,
                      React.createElement('span', { className: 'badge bg-warning me-2' },
                        React.createElement('i', { className: 'bi bi-exclamation-triangle me-1' }),
                        'No Cached UTXOs'
                      ),
                      React.createElement('small', { className: 'text-muted' },
                        'Fetch UTXOs from the wallet dashboard to enable offline mode'
                      )
                    )
                ),
                React.createElement('div', { className: 'col-md-4 text-md-end' },
                  React.createElement('div', { className: 'form-check form-switch' },
                    React.createElement('input', {
                      className: 'form-check-input',
                      type: 'checkbox',
                      id: 'offlineMode',
                      checked: useOfflineMode,
                      onChange: (e) => setUseOfflineMode(e.target.checked),
                      disabled: !cachedUTXOs
                    }),
                    React.createElement('label', { className: 'form-check-label', htmlFor: 'offlineMode' },
                      'Offline Mode'
                    )
                  ),
                  useOfflineMode && React.createElement('small', { className: 'text-primary d-block mt-1' },
                    'Using cached UTXOs'
                  )
                )
              )
            ),

            // Transaction creation form
            React.createElement('form', { onSubmit: handleCreateTransaction },
              React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Recipient Address'),
                React.createElement('div', { className: 'input-group' },
                  React.createElement('input', {
                    type: 'text',
                    className: `form-control ${!toAddress && formErrors.some(e => e.includes('address')) ? 'is-invalid' : ''}`,
                    value: toAddress,
                    onChange: (e) => handleAddressChange(e.target.value),
                    placeholder: `kaspa:qqkqkzjvr7zwxxmjxjkmxxdwju9kjs6e9u82uh59z07vgaks6gg62v8707g73 or domain.kas`,
                    required: true,
                    disabled: isLookingUpDomain
                  }),
                  isLookingUpDomain && React.createElement('div', {
                    className: 'input-group-text'
                  },
                    React.createElement('span', { className: 'spinner-border spinner-border-sm' })
                  ),
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary',
                    type: 'button',
                    onClick: handleScanRecipientAddress,
                    title: 'Scan QR code with camera',
                    disabled: isScanning,
                    style: { borderLeft: 'none' }
                  },
                    isScanning ? 
                      React.createElement('span', { className: 'spinner-border spinner-border-sm' }) :
                      React.createElement('i', { className: 'bi bi-camera' })
                  ),
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary',
                    type: 'button',
                    onClick: handleUploadRecipientQR,
                    title: 'Upload QR code image',
                    style: { borderLeft: 'none' }
                  },
                    React.createElement('i', { className: 'bi bi-upload' })
                  )
                ),
                React.createElement('div', { className: 'form-text' },
                  `Enter a valid ${walletState.network} address or .kas domain. Current network: `,
                  React.createElement('span', { className: 'badge bg-primary' }, walletState.network)
                ),
                domainLookupStatus && React.createElement('div', {
                  className: `alert alert-${domainLookupStatus.type === 'success' ? 'success' : 'danger'} mt-2 mb-0 py-2`,
                  style: { fontSize: '0.875em' }
                }, domainLookupStatus.message),
                formErrors.some(e => e.includes('address')) && React.createElement('div', {
                  className: 'invalid-feedback'
                }, 'Please enter a valid Kaspa address for the selected network')
              ),
              
              React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Amount (KAS)'),
                React.createElement('input', {
                  type: 'number',
                  className: `form-control ${!amount && formErrors.some(e => e.includes('amount')) ? 'is-invalid' : ''}`,
                  value: amount,
                  onChange: (e) => setAmount(e.target.value),
                  placeholder: DEFAULT_TRANSACTION_AMOUNT.toString(),
                  step: '0.00000001',
                  min: MIN_TRANSACTION_AMOUNT.toString(),
                  max: MAX_TRANSACTION_AMOUNT.toString(),
                  required: true
                }),
                React.createElement('div', { className: 'form-text' },
                  `Minimum: ${MIN_TRANSACTION_AMOUNT} KAS | Maximum: ${MAX_TRANSACTION_AMOUNT} KAS`
                ),
                formErrors.some(e => e.includes('amount')) && React.createElement('div', {
                  className: 'invalid-feedback'
                }, 'Please enter a valid amount')
              ),
              
              React.createElement('div', { className: 'd-grid gap-2' },
                React.createElement('button', {
                  type: 'submit',
                  className: `btn btn-primary ${isCreating ? 'disabled' : ''}`,
                  disabled: isCreating || formErrors.length > 0
                },
                  isCreating ? 
                    React.createElement('span', null,
                      React.createElement('span', { 
                        className: 'spinner-border spinner-border-sm me-2' 
                      }),
                      'Creating Transaction...'
                    ) :
                    React.createElement('span', null,
                      React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
                      'Create Transaction'
                    )
                ),
                React.createElement('button', {
                  type: 'button',
                  className: 'btn btn-outline-secondary',
                  onClick: () => onNavigate('wallet-dashboard')
                }, 'Back to Dashboard')
              )
            )
          )
        ),
        
        // Sign Transaction section
        transactionData && !signedTransactionData && React.createElement('div', { className: 'card mt-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-pen me-2' }),
              'Sign Transaction'
            )
          ),
          React.createElement('div', { className: 'card-body text-center' },
            React.createElement('p', { className: 'text-muted mb-3' },
              'Transaction created successfully. Sign it to enable submission to the network.'
            ),
            React.createElement('button', {
              className: `btn btn-success ${isSigning ? 'disabled' : ''}`,
              onClick: handleSignTransaction,
              disabled: isSigning
            },
              isSigning ? 
                React.createElement('span', null,
                  React.createElement('span', { 
                    className: 'spinner-border spinner-border-sm me-2' 
                  }),
                  'Signing Transaction...'
                ) :
                React.createElement('span', null,
                  React.createElement('i', { className: 'bi bi-pen me-2' }),
                  'Sign Transaction'
                )
            )
          )
        ),

        // Submit Transaction section
        signedTransactionData && !submittedTransactionData && React.createElement('div', { className: 'card mt-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-broadcast me-2' }),
              'Submit Transaction'
            )
          ),
          React.createElement('div', { className: 'card-body text-center' },
            React.createElement('p', { className: 'text-muted mb-3' },
              'Transaction signed successfully. Submit it to the Kaspa network to complete the transfer.'
            ),
            React.createElement('button', {
              className: `btn btn-primary ${isSubmitting ? 'disabled' : ''}`,
              onClick: handleSubmitTransaction,
              disabled: isSubmitting
            },
              isSubmitting ? 
                React.createElement('span', null,
                  React.createElement('span', { 
                    className: 'spinner-border spinner-border-sm me-2' 
                  }),
                  'Submitting Transaction...'
                ) :
                React.createElement('span', null,
                  React.createElement('i', { className: 'bi bi-broadcast me-2' }),
                  'Submit to Network'
                )
            )
          )
        ),

        // Transaction Success section
        submittedTransactionData && React.createElement('div', { className: 'card mt-4 border-success' },
          React.createElement('div', { className: 'card-header bg-success text-white' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-check-circle me-2' }),
              'Transaction Submitted Successfully'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'alert alert-success mb-0' },
              React.createElement('h6', { className: 'alert-heading' },
                React.createElement('i', { className: 'bi bi-broadcast me-2' }),
                'Transaction Broadcast Complete'
              ),
              React.createElement('p', { className: 'mb-2' },
                'Your transaction has been successfully submitted to the Kaspa network and is now being processed.'
              ),
              submittedTransactionData.txId && React.createElement('p', { className: 'mb-0' },
                React.createElement('strong', null, 'Network Transaction ID: '),
                React.createElement('code', { className: 'text-success' }, submittedTransactionData.txId)
              )
            )
          )
        ),

        // QR Code and Download area
        (transactionData && qrCodeData) && React.createElement('div', { className: 'card mt-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-qr-code me-2' }),
              `${submittedTransactionData ? 'Submitted' : signedTransactionData ? 'Signed' : 'Unsigned'} Transaction QR Code`,
              qrCodeData.isMultiPart && React.createElement('span', { className: 'badge bg-info ms-2' },
                `Multi-Part (${qrCodeData.totalParts} parts)`
              )
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row' },
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('div', { className: 'text-center' },
                  React.createElement('img', {
                    src: qrCodeData.qrDataURL,
                    alt: 'Transaction QR Code',
                    className: 'img-fluid border rounded',
                    style: { maxWidth: '250px' }
                  }),
                  qrCodeData?.isMultiPart && React.createElement('div', { className: 'mt-2' },
                    React.createElement('div', { className: 'btn-group', role: 'group' },
                      React.createElement('button', {
                        className: 'btn btn-outline-primary btn-sm',
                        onClick: () => {
                          const newPart = Math.max(1, (qrCodeData?.currentPart || 1) - 1);
                          const newQRPart = qrCodeData?.qrParts?.[newPart - 1];
                          if (newQRPart) {
                            setQrCodeData({
                              ...qrCodeData,
                              currentPart: newPart,
                              qrDataURL: newQRPart.qrDataURL,
                              displayPart: newQRPart
                            });
                          }
                        },
                        disabled: (qrCodeData?.currentPart || 1) <= 1
                      },
                        React.createElement('i', { className: 'bi bi-chevron-left' })
                      ),
                      React.createElement('span', { className: 'btn btn-outline-secondary btn-sm disabled' },
                        `Part ${qrCodeData?.currentPart || 1} of ${qrCodeData?.totalParts || 1}`
                      ),
                      React.createElement('button', {
                        className: 'btn btn-outline-primary btn-sm',
                        onClick: () => {
                          const newPart = Math.min(qrCodeData?.totalParts || 1, (qrCodeData?.currentPart || 1) + 1);
                          const newQRPart = qrCodeData?.qrParts?.[newPart - 1];
                          if (newQRPart) {
                            setQrCodeData({
                              ...qrCodeData,
                              currentPart: newPart,
                              qrDataURL: newQRPart.qrDataURL,
                              displayPart: newQRPart
                            });
                          }
                        },
                        disabled: (qrCodeData?.currentPart || 1) >= (qrCodeData?.totalParts || 1)
                      },
                        React.createElement('i', { className: 'bi bi-chevron-right' })
                      )
                    )
                  ),
                  React.createElement('p', { className: 'text-muted mt-2 small' },
                    qrCodeData.isMultiPart 
                      ? `Scan all ${qrCodeData.totalParts} parts to import the complete transaction`
                      : 'Scan this QR code to import the transaction'
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label small' }, 'Transaction ID'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control form-control-sm',
                    value: transactionData?.transactionId || 'Unknown',
                    readOnly: true
                  })
                ),
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label small' }, 
                    qrCodeData?.isMultiPart ? 'Total QR Data Size' : 'QR Data Size'
                  ),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control form-control-sm',
                    value: `${qrCodeData?.size || 0} bytes`,
                    readOnly: true
                  })
                ),
                qrCodeData?.isMultiPart && React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label small' }, 'Current Part Size'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control form-control-sm',
                    value: `${qrCodeData?.displayPart?.size || qrCodeData?.qrParts?.[0]?.size || 0} bytes`,
                    readOnly: true
                  })
                ),
                React.createElement('div', { className: 'd-grid gap-2' },
                  React.createElement('button', {
                    className: 'btn btn-outline-primary btn-sm',
                    onClick: handleDownloadQR
                  },
                    React.createElement('i', { className: 'bi bi-download me-1' }),
                    qrCodeData.isMultiPart 
                      ? `Download Part ${qrCodeData.currentPart}` 
                      : `Download ${submittedTransactionData ? 'Submitted' : signedTransactionData ? 'Signed' : 'Unsigned'} QR`
                  ),
                  qrCodeData.isMultiPart && React.createElement('button', {
                    className: 'btn btn-outline-success btn-sm',
                    onClick: handleDownloadAllQRParts
                  },
                    React.createElement('i', { className: 'bi bi-download me-1' }),
                    'Download All Parts'
                  ),
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary btn-sm',
                    onClick: handleDownloadJSON
                  },
                    React.createElement('i', { className: 'bi bi-file-earmark-code me-1' }),
                    `Download ${submittedTransactionData ? 'Submitted' : signedTransactionData ? 'Signed' : 'Unsigned'} JSON`
                  )
                )
              )
            )
          )
        )
      ),
      
      // Transaction steps sidebar
      React.createElement('div', { className: 'col-lg-4' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' }, 'Transaction Steps')
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'list-group list-group-flush' },
              React.createElement('div', { className: 'list-group-item d-flex align-items-center' },
                React.createElement('span', { 
                  className: `badge ${transactionData ? 'bg-success' : 'bg-secondary'} me-2` 
                }, '1'),
                'Create Transaction',
                transactionData && React.createElement('i', { className: 'bi bi-check-lg text-success ms-auto' })
              ),
              React.createElement('div', { className: 'list-group-item d-flex align-items-center' },
                React.createElement('span', { 
                  className: `badge ${signedTransactionData ? 'bg-success' : transactionData ? 'bg-warning' : 'bg-secondary'} me-2` 
                }, '2'),
                'Sign Transaction',
                signedTransactionData ? 
                  React.createElement('i', { className: 'bi bi-check-lg text-success ms-auto' }) :
                  transactionData ? 
                    React.createElement('small', { className: 'text-warning ms-auto' }, 'Ready') :
                    React.createElement('small', { className: 'text-muted ms-auto' }, 'Pending')
              ),
              React.createElement('div', { className: 'list-group-item d-flex align-items-center' },
                React.createElement('span', { 
                  className: `badge ${submittedTransactionData ? 'bg-success' : signedTransactionData ? 'bg-warning' : 'bg-secondary'} me-2` 
                }, '3'),
                'Submit Transaction',
                submittedTransactionData ? 
                  React.createElement('i', { className: 'bi bi-check-lg text-success ms-auto' }) :
                  signedTransactionData ? 
                    React.createElement('small', { className: 'text-warning ms-auto' }, 'Ready') :
                    React.createElement('small', { className: 'text-muted ms-auto' }, 'Pending')
              )
            )
          )
        ),
        
        // Upload info card
        (uploadedFile || uploadedQrData) && React.createElement('div', { className: 'card mt-3' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-info-circle me-2' }),
              'Uploaded Transaction'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'small' },
              React.createElement('strong', null, 'Type: '),
              React.createElement('span', { 
                className: `badge bg-${(uploadedFile?.type === 'unsigned' || uploadedQrData?.type === 'unsigned') ? 'warning' : 
                                  (uploadedFile?.type === 'signed' || uploadedQrData?.type === 'signed') ? 'success' : 'info'}`
              }, uploadedFile?.type || uploadedQrData?.type || 'Unknown'),
              React.createElement('br'),
              React.createElement('strong', null, 'Source: '),
              uploadedFile ? 'JSON file' : 
              uploadedQrData?.source === 'camera' ? 'Camera scan' :
              uploadedQrData?.source === 'multipart-files' ? 'Multi-part upload' : 'QR code',
              React.createElement('br'),
              React.createElement('strong', null, 'File: '),
              React.createElement('span', { className: 'text-break' }, 
                (uploadedFile?.file?.name) || 
                (uploadedQrData?.file?.name) || 
                (uploadedQrData?.files ? `${uploadedQrData.files.length} files` : 
                 uploadedQrData?.source === 'camera' ? 'Camera scan' : 'Unknown')
              )
            )
          )
        )
      )
    )
  );
} 