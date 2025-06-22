const { useState, useEffect, useRef } = React;

export function TransactionManager({ walletState, onNavigate, addNotification, onGenerateChangeAddress, onMarkAddressUsed }) {
  const [amount, setAmount] = useState('');
  const [toAddress, setToAddress] = useState('');
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
  const transactionHandlersSetup = useRef(false);
  const fileInputRef = useRef();
  const qrInputRef = useRef();

  useEffect(() => {
    // Set up transaction event handlers from init.js when component mounts
    if (!transactionHandlersSetup.current) {
      setupTransactionHandlers();
      transactionHandlersSetup.current = true;
    }
  }, []);

  const setupTransactionHandlers = async () => {
    try {
      const { setupTransactionEventHandlers } = await import('../../kaspa/js/init.js');
      setupTransactionEventHandlers();
    } catch (error) {
      console.error('Failed to set up transaction handlers:', error);
      addNotification('Failed to initialize transaction system', 'error');
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

    if (!walletState.currentWallet) {
      addNotification('No wallet selected', 'error');
      return;
    }

    // Validate address for current network (use network from header dropdown)
    const currentNetwork = walletState.network;
    const addressValidation = await validateAddressForNetwork(toAddress.trim(), currentNetwork);
    if (!addressValidation.isValid) {
      addNotification(addressValidation.error, 'error');
      return;
    }

    setIsCreating(true);
    
    try {
      let changeAddress = null;
      
      // For HD wallets, generate a new change address
      if (walletState.isHDWallet && onGenerateChangeAddress) {
        const changeAddressInfo = await onGenerateChangeAddress();
        changeAddress = changeAddressInfo?.address;
      }

      // Use the existing transaction creation logic from transaction-create.js
      const { createTransaction } = await import('../../kaspa/js/transaction-create.js');
      
      const transaction = await createTransaction(
        walletState.address,
        addressValidation.address,
        parseFloat(amount),
        currentNetwork,
        { 
          changeAddress: changeAddress,
          hdWallet: walletState.hdWallet
        }
      );

      if (transaction && transaction.success) {
        // The transaction data is in the transaction object itself, not nested under 'data'
        setTransactionData(transaction);
        setSignedTransactionData(null); // Clear any previous signed transaction
        await generateQRCode(transaction, 'unsigned');
        addNotification('Transaction created successfully', 'success');
      } else if (transaction && transaction.error) {
        throw new Error(transaction.error);
      } else if (transaction && transaction.transactionId) {
        // Handle case where transaction is returned directly without success wrapper
        setTransactionData(transaction);
        setSignedTransactionData(null); // Clear any previous signed transaction
        await generateQRCode(transaction, 'unsigned');
        addNotification('Transaction created successfully', 'success');
      } else {
        throw new Error('Transaction creation failed - no valid response received');
      }
      
    } catch (error) {
      console.error('Transaction creation failed:', error);
      addNotification('Transaction creation failed: ' + error.message, 'error');
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
      const { signTransaction } = await import('../../kaspa/js/transaction-sign.js');
      
      let result;
      
      if (walletState.isHDWallet && walletState.hdWallet) {
        // For HD wallets, we need to get private keys for all input addresses
        const inputAddresses = transactionData.inputs?.map(input => input.address) || [];
        const privateKeys = {};
        
        for (const address of inputAddresses) {
          try {
            privateKeys[address] = walletState.hdWallet.getPrivateKeyForAddress(address);
          } catch (error) {
            console.warn('Could not get private key for address:', error.message);
          }
        }
        
        result = await signTransaction(transactionData, privateKeys);
      } else {
        // For single address wallets
        if (!walletState.currentWallet || !walletState.currentWallet.privateKey) {
          addNotification('No private key available for signing', 'error');
          return;
        }
        
        result = await signTransaction(transactionData, walletState.currentWallet.privateKey);
      }

      if (result && result.success) {
        // Create signed transaction data structure
        const signedTxData = {
          ...transactionData, // Include original transaction data
          ...result, // Include signing result
          status: 'signed',
          signedAt: new Date().toISOString()
        };

        setSignedTransactionData(signedTxData);
        await generateQRCode(signedTxData, 'signed');
        addNotification('Transaction signed successfully', 'success');
      } else {
        throw new Error(result?.error || 'Transaction signing failed');
      }

    } catch (error) {
      console.error('Transaction signing failed:', error);
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
        await generateQRCode(submittedTxData, 'submitted');
        
        // Mark addresses as used for HD wallets
        if (walletState.isHDWallet && onMarkAddressUsed) {
          // Mark input addresses as used
          if (submittedTxData.inputs) {
            submittedTxData.inputs.forEach(input => {
              if (input.address) {
                onMarkAddressUsed(input.address);
              }
            });
          }
          
          // Mark change address as used if it exists
          if (submittedTxData.outputs) {
            submittedTxData.outputs.forEach(output => {
              if (output.address && output.address !== toAddress) {
                // This is likely the change address
                onMarkAddressUsed(output.address);
              }
            });
          }
        }
        
        addNotification('Transaction submitted successfully to the network', 'success');
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
        setQrCodeData({ ...qrResult, type });
      } else {
        console.error('QR generation failed:', qrResult.error);
        addNotification('Failed to generate QR code: ' + qrResult.error, 'warning');
      }
    } catch (error) {
      console.error('QR generation error:', error);
      addNotification('Failed to generate QR code', 'warning');
    }
  };

  const handleDownloadQR = () => {
    if (!qrCodeData) return;
    
    try {
      const transactionType = submittedTransactionData ? 'submitted' : signedTransactionData ? 'signed' : 'unsigned';
      const txId = (submittedTransactionData || signedTransactionData || transactionData)?.transactionId;
      const filename = `kaspa-${transactionType}-transaction-${txId}.png`;
      
      // Create download link
      const link = document.createElement('a');
      link.href = qrCodeData.qrDataURL;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      
      addNotification(`${transactionType} transaction QR code downloaded`, 'success');
    } catch (error) {
      console.error('Download failed:', error);
      addNotification('Failed to download QR code', 'error');
    }
  };

  const convertBigIntToString = (obj) => {
    if (obj === null || obj === undefined) return obj;
    
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
  };

  const convertStringToBigInt = (obj, bigIntFields = ['amount', 'fee', 'value', 'satoshis']) => {
    if (obj === null || obj === undefined) return obj;
    
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
  };

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

          // Detect transaction type
          let transactionType = 'unknown';
          if (qrData.type === 'kaspa-unsigned-transaction-qr') {
            transactionType = 'unsigned';
          } else if (qrData.type === 'kaspa-signed-transaction-qr') {
            transactionType = 'signed';
          } else if (qrData.type === 'kaspa-submitted-transaction-qr') {
            transactionType = 'submitted';
          }

          if (transactionType === 'unknown') {
            throw new Error(`Unrecognized QR transaction format. Found type: ${qrData.type || 'undefined'}`);
          }

          // Convert string values back to BigInt where needed
          const processedQrData = convertStringToBigInt(qrData);

          // Reset transaction progress states
          if (transactionType === 'unsigned') {
            setTransactionData(processedQrData);
            setSignedTransactionData(null);
            setSubmittedTransactionData(null);
            await generateQRCode(processedQrData, 'unsigned');
          } else if (transactionType === 'signed') {
            setTransactionData(null);
            setSignedTransactionData(processedQrData);
            setSubmittedTransactionData(null);
            await generateQRCode(processedQrData, 'signed');
          } else if (transactionType === 'submitted') {
            setTransactionData(null);
            setSignedTransactionData(null);
            setSubmittedTransactionData(processedQrData);
            await generateQRCode(processedQrData, 'submitted');
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
      
      // Detect QR transaction type
      let transactionType = 'unknown';
      if (qrData.type === 'kaspa-unsigned-transaction-qr') {
        transactionType = 'unsigned';
      } else if (qrData.type === 'kaspa-signed-transaction-qr') {
        transactionType = 'signed';
      } else if (qrData.type === 'kaspa-submitted-transaction-qr') {
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
        setTransactionData(processedQrData);
        setSignedTransactionData(null);
        setSubmittedTransactionData(null);
        setQrCodeData({ qrDataURL: URL.createObjectURL(file), size: file.size, type: 'unsigned' });
      } else if (transactionType === 'signed') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData(processedQrData);
        setSubmittedTransactionData(null);
        setQrCodeData({ qrDataURL: URL.createObjectURL(file), size: file.size, type: 'signed' });
      } else if (transactionType === 'submitted') {
        setTransactionData(null); // Clear unsigned data
        setSignedTransactionData(null); // Clear signed data
        setSubmittedTransactionData(processedQrData);
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

          // Detect transaction type
          let transactionType = 'unknown';
          if (qrData.type === 'kaspa-unsigned-transaction-qr') {
            transactionType = 'unsigned';
          } else if (qrData.type === 'kaspa-signed-transaction-qr') {
            transactionType = 'signed';
          } else if (qrData.type === 'kaspa-submitted-transaction-qr') {
            transactionType = 'submitted';
          }

          if (transactionType === 'unknown') {
            throw new Error(`Unrecognized QR transaction format. Found type: ${qrData.type || 'undefined'}`);
          }

          // Convert string values back to BigInt where needed
          const processedQrData = convertStringToBigInt(qrData);

          // Reset transaction progress states based on scanned QR transaction type
          if (transactionType === 'unsigned') {
            setTransactionData(processedQrData);
            setSignedTransactionData(null);
            setSubmittedTransactionData(null);
            await generateQRCode(processedQrData, 'unsigned');
          } else if (transactionType === 'signed') {
            setTransactionData(null);
            setSignedTransactionData(processedQrData);
            setSubmittedTransactionData(null);
            await generateQRCode(processedQrData, 'signed');
          } else if (transactionType === 'submitted') {
            setTransactionData(null);
            setSignedTransactionData(null);
            setSubmittedTransactionData(processedQrData);
            await generateQRCode(processedQrData, 'submitted');
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
            
            // Transaction creation form
            React.createElement('form', { onSubmit: handleCreateTransaction },
              React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Recipient Address'),
                React.createElement('input', {
                  type: 'text',
                  className: `form-control ${!toAddress && formErrors.some(e => e.includes('address')) ? 'is-invalid' : ''}`,
                  value: toAddress,
                  onChange: (e) => setToAddress(e.target.value),
                  placeholder: `kaspa:qqkqkzjvr7zwxxmjxjkmxxdwju9kjs6e9u82uh59z07vgaks6gg62v8707g73`,
                  required: true
                }),
                React.createElement('div', { className: 'form-text' },
                  `Enter a valid ${walletState.network} address (must start with "${walletState.network === 'mainnet' ? 'kaspa:' : 'kaspatest:'}"). Current network: `,
                  React.createElement('span', { className: 'badge bg-primary' }, walletState.network)
                ),
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
                  placeholder: '0.00',
                  step: '0.00000001',
                  min: '0',
                  required: true
                }),
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
              `${submittedTransactionData ? 'Submitted' : signedTransactionData ? 'Signed' : 'Unsigned'} Transaction QR Code`
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
                  React.createElement('p', { className: 'text-muted mt-2 small' },
                    'Scan this QR code to import the transaction'
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label small' }, 'Transaction ID'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control form-control-sm',
                    value: transactionData.transactionId,
                    readOnly: true
                  })
                ),
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label small' }, 'QR Data Size'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control form-control-sm',
                    value: `${qrCodeData.size} bytes`,
                    readOnly: true
                  })
                ),
                React.createElement('div', { className: 'd-grid gap-2' },
                  React.createElement('button', {
                    className: 'btn btn-outline-primary btn-sm',
                    onClick: handleDownloadQR
                  },
                    React.createElement('i', { className: 'bi bi-download me-1' }),
                    `Download ${submittedTransactionData ? 'Submitted' : signedTransactionData ? 'Signed' : 'Unsigned'} QR`
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
                className: `badge bg-${uploadedFile?.type === 'unsigned' || uploadedQrData?.type === 'unsigned' ? 'warning' : 
                                  uploadedFile?.type === 'signed' || uploadedQrData?.type === 'signed' ? 'success' : 'info'}`
              }, uploadedFile?.type || uploadedQrData?.type),
              React.createElement('br'),
              React.createElement('strong', null, 'Source: '),
              uploadedFile ? 'JSON file' : 'QR code',
              React.createElement('br'),
              React.createElement('strong', null, 'File: '),
              React.createElement('span', { className: 'text-break' }, 
                uploadedFile?.file.name || uploadedQrData?.file.name
              )
            )
          )
        )
      )
    )
  );
} 