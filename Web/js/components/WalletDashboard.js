const { useState, useEffect } = React;

export function WalletDashboard({ walletState, onNavigate, addNotification, onGenerateNewAddress, onUpdateBalance, onMarkAddressUsed, cachedUTXOs, onCacheUTXOs, onClearCachedUTXOs }) {
  const [balance, setBalance] = useState(null);
  const [lastBalanceCheck, setLastBalanceCheck] = useState(null);
  const [addressQRCode, setAddressQRCode] = useState(null);
  const [showAddressQR, setShowAddressQR] = useState(false);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [xpubQRCode, setXpubQRCode] = useState(null);
  const [xpubString, setXpubString] = useState(null);
  const [qrMode, setQrMode] = useState('address');
  const [utxoQRCodes, setUtxoQRCodes] = useState(null);
  const [showUTXOQR, setShowUTXOQR] = useState(false);
  const [showUTXOImport, setShowUTXOImport] = useState(false);
  const [showCustomAddressPrompt, setShowCustomAddressPrompt] = useState(false);
  const [customAddress, setCustomAddress] = useState('');
  const [scanProgress, setScanProgress] = useState({ current: 0, total: 0, found: 0 });
  const [scanResults, setScanResults] = useState([]);
  const [isScanning, setIsScanning] = useState(false);
  const [scanStatus, setScanStatus] = useState('');
  const [scanStartIndex, setScanStartIndex] = useState(0);
  const [scanGapLimit, setScanGapLimit] = useState(20);
  const [showScanResults, setShowScanResults] = useState(false);
  const [isQRScannerOpen, setIsQRScannerOpen] = useState(false);
  const [qrScannerType, setQrScannerType] = useState('address');
  const [lastUTXOFetch, setLastUTXOFetch] = useState(null);
  const [isContinuousScanning, setIsContinuousScanning] = useState(false);
  const [continuousScanProgress, setContinuousScanProgress] = useState({ processed: 0, found: 0 });
  const [totalAddressesFound, setTotalAddressesFound] = useState(0);
  const [continuousScanTimeoutId, setContinuousScanTimeoutId] = useState(null);
  const [isCheckingBalance, setIsCheckingBalance] = useState(false);
  const [isLoadingUTXOs, setIsLoadingUTXOs] = useState(false);

  // Add missing state variables
  const [selectedNetwork, setSelectedNetwork] = useState(walletState.network || 'mainnet');
  const [startIndex, setStartIndex] = useState(0);
  const [gapLimit, setGapLimit] = useState(20);
  const [isProcessingExtendedKey, setIsProcessingExtendedKey] = useState(false);
  const [extendedKeyProgress, setExtendedKeyProgress] = useState({ current: 0, total: 0, status: '' });
  const [lastScanResult, setLastScanResult] = useState(null);
  const [currentScanBatch, setCurrentScanBatch] = useState(0);
  const [lastScannedXpubData, setLastScannedXpubData] = useState(null);

  // Function to check balance
  const checkBalance = async () => {
    if (isCheckingBalance) return;

    setIsCheckingBalance(true);
    try {
      if (walletState.isHDWallet && walletState.hdWallet) {
        // For HD wallets, use the HD wallet discovery which is more comprehensive
        await performHDWalletAddressDiscovery();
      } else {
        // For single address wallets - use unified wallet manager
        const { getSingleWallet } = await import('../../kaspa/js/wallet-manager.js');
        
        const unifiedWallet = getSingleWallet(walletState.address, walletState.network);
        await unifiedWallet.initialize();
        
        const balanceResult = await unifiedWallet.checkSingleAddressBalance(walletState.address);

        if (balanceResult.success) {
          setBalance(balanceResult.balance);
          setLastBalanceCheck(new Date());
          
          // Handle BigInt balance values
          let kasValue;
          if (typeof balanceResult.balance.kas === 'bigint') {
            kasValue = Number(balanceResult.balance.kas) / 100000000;
          } else if (typeof balanceResult.balance.kas === 'string') {
            kasValue = parseFloat(balanceResult.balance.kas);
          } else {
            kasValue = balanceResult.balance.kas;
          }
          
          addNotification(`Balance: ${kasValue.toFixed(8)} KAS`, 'success');
        } else {
          addNotification('Failed to check balance: ' + balanceResult.error, 'error');
        }
      }
    } catch (error) {
      console.error('Balance check failed:', error);
      addNotification('Failed to check balance: ' + error.message, 'error');
    } finally {
      setIsCheckingBalance(false);
    }
  };

  // HD wallet balance refresh using optimized Kaspa WASM built-in functions
  const performHDWalletAddressDiscovery = async () => {
    setIsDiscovering(true);
    addNotification('Refreshing HD wallet balance...', 'info');
    
    try {
      // Use the unified wallet manager with mnemonic-based scanning (NOT xpub)
      const { getHDWallet } = await import('../../kaspa/js/wallet-manager.js');
      
      const unifiedWallet = getHDWallet(
        walletState.hdWallet.mnemonic,
        walletState.network,
        walletState.hdWallet.derivationPath
      );
      
      await unifiedWallet.initialize();
      
      // Use optimized mnemonic-based balance checking
      const balanceResult = await unifiedWallet.checkHDWalletBalance();
      
      if (!balanceResult.success) {
        throw new Error(balanceResult.error);
      }
      
      // Update balance information for all discovered addresses
      for (const addressInfo of balanceResult.addressesWithBalance) {
        if (onUpdateBalance && onMarkAddressUsed) {
          onUpdateBalance(addressInfo.address, addressInfo.balanceSompi, addressInfo.utxos);
        onMarkAddressUsed(addressInfo.address);
      }
      }
      
      // Update UI with total balance
      setBalance({ 
        kas: balanceResult.totalBalance, 
        sompi: balanceResult.totalBalanceSompi 
      });
      setLastBalanceCheck(new Date());
      
      if (balanceResult.addressesFound > 0) {
        // Handle BigInt total balance values
        let totalKasValue;
        if (typeof balanceResult.totalBalance === 'bigint') {
          totalKasValue = Number(balanceResult.totalBalance) / 100000000;
        } else if (typeof balanceResult.totalBalance === 'string') {
          totalKasValue = parseFloat(balanceResult.totalBalance);
        } else {
          totalKasValue = balanceResult.totalBalance;
        }
        
        addNotification(
          `Found balance across ${balanceResult.addressesFound} addresses: ${totalKasValue.toFixed(8)} KAS`, 
          'success'
        );
      } else {
        addNotification('No addresses with balance found in HD wallet', 'info');
      }
      
    } catch (error) {
      console.error('HD wallet balance refresh failed:', error);
      addNotification('Failed to refresh HD wallet balance: ' + error.message, 'error');
    } finally {
      setIsDiscovering(false);
    }
  };



  // Auto-check balance when wallet loads (only if auto-discovery is enabled)
  useEffect(() => {
    if (walletState.address && !balance) {
      // Check if auto-discovery is enabled
      const checkAutoDiscovery = async () => {
        try {
          const { getAutoDiscoveryEnabled } = await import('../utils/settings-utils.js');
          const autoDiscoveryEnabled = getAutoDiscoveryEnabled();
          
          if (!autoDiscoveryEnabled) {
            console.log('Auto-discovery disabled, skipping balance check');
            return;
          }

          // For HD wallets, use the total balance from HD wallet manager if available
          if (walletState.isHDWallet && walletState.hdWallet) {
            try {
              const totalBalance = walletState.hdWallet.getTotalBalance();
              if (totalBalance > 0n) {
                const { sompiToKas } = await import('../../kaspa/js/currency-utils.js');
                const totalKAS = sompiToKas(totalBalance);
                setBalance({ kas: totalKAS, sompi: totalBalance });
              } else {
                // If no balance found in HD wallet, perform discovery
                performHDWalletAddressDiscovery();
              }
            } catch (error) {
              console.warn('Error getting HD wallet balance, falling back to discovery:', error);
              performHDWalletAddressDiscovery();
            }
          } else {
            checkBalance();
          }
        } catch (error) {
          console.error('Error checking auto-discovery setting:', error);
          // Default to checking balance if setting can't be loaded
          if (walletState.isHDWallet && walletState.hdWallet) {
            performHDWalletAddressDiscovery();
          } else {
            checkBalance();
          }
        }
      };

      checkAutoDiscovery();
    }
  }, [walletState.address, walletState.hdWallet]);

  // Sync balance with wallet state when it updates
  useEffect(() => {
    const updateBalance = async () => {
      if (walletState.balance !== null && walletState.balance !== undefined) {
        if (typeof walletState.balance === 'bigint') {
          const { sompiToKas } = await import('../../kaspa/js/currency-utils.js');
          const totalKAS = sompiToKas(walletState.balance);
          setBalance({ kas: parseFloat(totalKAS), sompi: walletState.balance });
        } else if (typeof walletState.balance === 'object') {
          setBalance(walletState.balance);
        }
      }
    };
    
    updateBalance();
  }, [walletState.balance]);

  // Sync selectedNetwork with wallet network
  useEffect(() => {
    if (walletState.network && walletState.network !== selectedNetwork) {
      console.log('🔍 Syncing selectedNetwork from', selectedNetwork, 'to', walletState.network);
      setSelectedNetwork(walletState.network);
    }
  }, [walletState.network, selectedNetwork]);

  // Debug showUTXOQR state changes
  useEffect(() => {
    console.log('🔍 showUTXOQR state changed:', showUTXOQR);
    if (showUTXOQR) {
      console.log('🔍 UTXO modal should be visible now!');
      console.log('🔍 All modal states:', {
        showUTXOQR,
        showAddressQR,
        showUTXOImport,
        showCustomAddressPrompt,
        utxoQRCodes: !!utxoQRCodes,
        cachedUTXOs: !!cachedUTXOs
      });
    }
  }, [showUTXOQR, showAddressQR, showUTXOImport, showCustomAddressPrompt, utxoQRCodes, cachedUTXOs]);

  // Display multi-part QR codes when loaded
  useEffect(() => {
    if (utxoQRCodes && utxoQRCodes.isMultiPart && showUTXOQR) {
      const container = document.getElementById('utxo-qr-display');
      if (container) {
        // Create multi-part QR display using the existing function
        import('../../kaspa/js/qr-manager.js').then(({ createMultiPartQRDisplay }) => {
          const displayElement = createMultiPartQRDisplay(utxoQRCodes, 'UTXO Data');
          container.innerHTML = '';
          container.appendChild(displayElement);
        });
      }
    }
  }, [utxoQRCodes, showUTXOQR]);

  const formatBalance = (balanceObj) => {
    if (!balanceObj) return '0.00';
    
    // Handle different types of balance values
    let kasValue;
    if (typeof balanceObj.kas === 'bigint') {
      // Convert BigInt to number (assuming it's in sompi, convert to KAS)
      kasValue = Number(balanceObj.kas) / 100000000; // Convert from sompi to KAS
    } else if (typeof balanceObj.kas === 'string') {
      // Convert string to number
      kasValue = parseFloat(balanceObj.kas);
    } else if (typeof balanceObj.kas === 'number') {
      kasValue = balanceObj.kas;
    } else {
      // Fallback
      kasValue = 0;
    }
    
    return kasValue.toFixed(8);
  };

  const formatLastCheck = (date) => {
    if (!date) return 'Never';
    return date.toLocaleTimeString();
  };

  // Copy address to clipboard
  const copyAddress = async () => {
    if (!walletState.address) {
      addNotification('No address to copy', 'error');
      return;
    }

    try {
      await navigator.clipboard.writeText(walletState.address);
      addNotification('Address copied to clipboard', 'success');
    } catch (error) {
      // Fallback for older browsers
      const textArea = document.createElement('textarea');
      textArea.value = walletState.address;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
      addNotification('Address copied to clipboard', 'success');
    }
  };

  // Generate and show QR code for address
  const showAddressQRCode = async () => {
    if (!walletState.address) {
      addNotification('No address available', 'error');
      return;
    }

    try {
      // Use cache-busting parameter to ensure fresh import
      const { generateQRCode } = await import(`../../kaspa/js/qr-manager.js?v=${Date.now()}`);
      
      const qrResult = await generateQRCode(walletState.address, {
        width: 300,
        height: 300,
        margin: 2
      });

      if (qrResult.success) {
        setAddressQRCode(qrResult.qrDataURL);
        setQrMode('address');
        setShowAddressQR(true);
      } else {
        addNotification('Failed to generate QR code: ' + qrResult.error, 'error');
      }
    } catch (error) {
      console.error('QR generation error:', error);
      addNotification('Failed to generate QR code: ' + error.message, 'error');
    }
  };

  // Generate and show QR code for extended public key
  const showXpubQRCode = async () => {
    if (!walletState.isHDWallet || !walletState.hdWallet) {
      addNotification('Extended public key only available for HD wallets', 'error');
      return;
    }

    try {
      // Get the extended public key from the HD wallet
      const xpub = await getExtendedPublicKey();
      
      if (!xpub) {
        addNotification('Failed to get extended public key', 'error');
        return;
      }

      // Get the proper account-level derivation path from HD wallet
      let derivationPath = walletState.hdWallet?.derivationPath;
      
      // Ensure we use account-level path (remove address-specific parts if present)
      if (derivationPath && derivationPath.includes('/0/0')) {
        // Convert address-level path to account-level path
        derivationPath = derivationPath.split('/0/0')[0];
      } else if (derivationPath && derivationPath.includes('/1/')) {
        // Handle change address paths as well
        derivationPath = derivationPath.split('/1/')[0];
      }
      
      // Fallback to standard Kaspa derivation path if not available
      if (!derivationPath) {
        // Kaspa uses the same derivation path for both mainnet and testnet
        derivationPath = "m/44'/111111'/0'";
      }
      
      // Create structured QR data with derivation path
      const xpubData = {
        type: 'kaspa-xpub',
        version: '1.0',
        xpub: xpub,
        derivationPath: derivationPath,
        network: walletState.network,
        timestamp: Date.now()
      };

      // Use cache-busting parameter to ensure fresh import
      const { generateQRCode } = await import(`../../kaspa/js/qr-manager.js?v=${Date.now()}`);
      
      const qrResult = await generateQRCode(JSON.stringify(xpubData), {
        width: 300,
        height: 300,
        margin: 2
      });

      if (qrResult.success) {
        setXpubQRCode(qrResult.qrDataURL);
        setXpubString(xpub);
        setQrMode('xpub');
        setShowAddressQR(true);
      } else {
        addNotification('Failed to generate extended public key QR code: ' + qrResult.error, 'error');
      }
    } catch (error) {
      console.error('Extended public key QR generation error:', error);
      addNotification('Failed to generate extended public key QR code: ' + error.message, 'error');
    }
  };

  // Get extended public key from HD wallet using the new address scanner
  const getExtendedPublicKey = async () => {
    if (!walletState.isHDWallet || !walletState.hdWallet) {
      return null;
    }

    try {
      const { addressScanner } = await import('../../kaspa/js/address-scanner.js');
      
      const xpub = await addressScanner.generateXpubFromHDWallet(walletState.hdWallet);
      
      return xpub;
      
    } catch (error) {
      console.error('Error getting extended public key:', error);
      addNotification('Error getting extended public key: ' + error.message, 'error');
      return null;
    }
  };

  // Switch between address and extended public key QR modes
  const switchQRMode = async (mode) => {
    if (mode === 'address') {
      await showAddressQRCode();
    } else if (mode === 'xpub') {
      await showXpubQRCode();
    }
  };

  const closeAddressQR = () => {
    setShowAddressQR(false);
    setAddressQRCode(null);
    setXpubQRCode(null);
    setXpubString(null);
    setQrMode('address');
  };

  const handleDownloadAddressQR = async () => {
    const currentQRCode = qrMode === 'xpub' ? xpubQRCode : addressQRCode;
    
    if (!currentQRCode) {
      addNotification('No QR code to download', 'error');
      return;
    }

    try {
      // Import the download function from QR manager
      const { downloadQRImage } = await import('../../kaspa/js/qr-manager.js');
      
      // Generate filename with current date
      const date = new Date().toISOString().split('T')[0]; // YYYY-MM-DD format
      const filename = qrMode === 'xpub' ? 
        `kaspa_xpub_qr_${date}.png` : 
        `kaspa_address_qr_${date}.png`;
      
      // Download the QR code
      downloadQRImage(currentQRCode, filename);
      
      addNotification('QR code downloaded successfully', 'success');
    } catch (error) {
      console.error('Error downloading QR code:', error);
      addNotification('Failed to download QR code: ' + error.message, 'error');
    }
  };

  // Fetch UTXOs and generate QR codes
  const fetchUTXOsForQR = async (customAddress = null) => {
    if (!walletState.address && !customAddress) {
      addNotification('No wallet address available', 'error');
      return;
    }

    // Close any open modals and show toast notification
    setShowCustomAddressPrompt(false);
    setShowUTXOImport(false);
    setIsDiscovering(true);
    addNotification('UTXO scanning started. This may take a moment...', 'info');
    
    try {
      const { fetchUTXOsForAddress, fetchUTXOsForAddresses } = await import('../../kaspa/js/address-scanner.js');
      
      let utxoResult;
      let allAddresses;
      
      if (customAddress) {
        // Use the custom address provided
        console.log('Fetching UTXOs for custom address:', customAddress);
        allAddresses = [customAddress];
        utxoResult = await fetchUTXOsForAddress(customAddress, walletState.network);
      } else if (walletState.isHDWallet && walletState.hdWallet) {
        // For HD wallets, fetch UTXOs from all addresses with balances
        const addressesWithBalance = walletState.hdWallet.getAllAddresses()
          .filter(addr => addr.balance > 0n)
          .map(addr => addr.address);
        
        console.log('HD Wallet addresses with balance:', addressesWithBalance);
        
        if (addressesWithBalance.length > 0) {
          allAddresses = addressesWithBalance;
          utxoResult = await fetchUTXOsForAddresses(allAddresses, walletState.network);
        } else {
          // Fallback to current address if no addresses with balance found
          console.log('No HD addresses with balance, using current address:', walletState.address);
          allAddresses = [walletState.address];
          utxoResult = await fetchUTXOsForAddress(walletState.address, walletState.network);
        }
      } else {
        // For single address wallets
        console.log('Single address wallet, using:', walletState.address);
        allAddresses = [walletState.address];
        utxoResult = await fetchUTXOsForAddress(walletState.address, walletState.network);
      }
      
      console.log('UTXO fetch result:', utxoResult);
      
      if (!utxoResult.success) {
        addNotification('Failed to fetch UTXOs: ' + utxoResult.error, 'error');
        return;
      }
      
      console.log('UTXOs found:', utxoResult.utxos?.length || 0);
      
      if (!utxoResult.utxos || utxoResult.utxos.length === 0) {
        console.log('🔍 No UTXOs found, but creating empty UTXO data structure for modal');
        
        // Still create UTXO data structure and show modal, even with 0 UTXOs
        const emptyUtxoData = {
          utxos: [],
          addresses: allAddresses,
          networkType: walletState.network,
          timestamp: Date.now(),
          count: 0
        };
        
        console.log('🔍 Caching empty UTXOs...');
        onCacheUTXOs(emptyUtxoData);
        setLastUTXOFetch(new Date());
        
        // Generate QR codes even for empty data (will show the structure)
        console.log('🔍 Generating QR codes for empty data...');
        await generateUTXOQRCodes(emptyUtxoData);
        
        // Show QR codes modal
        console.log('🔍 Setting showUTXOQR to true for empty data...');
        setShowUTXOQR(true);
        
        addNotification(`Address has balance but no available UTXOs found. UTXOs may have been spent or are not confirmed yet. Empty UTXO data generated for reference.`, 'info');
        return;
      }
      
      // Cache the UTXOs
      const utxoData = {
        utxos: utxoResult.utxos,
        addresses: allAddresses,
        networkType: walletState.network,
        timestamp: Date.now(),
        count: utxoResult.count || utxoResult.utxos.length
      };
      
      onCacheUTXOs(utxoData);
      setLastUTXOFetch(new Date());
      
      // Generate QR code(s) for the UTXO data
      await generateUTXOQRCodes(utxoData);
      
      // Show QR codes
      setShowUTXOQR(true);
      
      addNotification(`Successfully fetched ${utxoData.count} UTXOs`, 'success');
      
    } catch (error) {
      console.error('Error fetching UTXOs:', error);
      addNotification('Error fetching UTXOs: ' + error.message, 'error');
    } finally {
      setIsDiscovering(false);
    }
  };

  // Convert BigInt values to strings for JSON serialization (reuse existing pattern)
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

  // Convert string values back to BigInt for UTXO fields
  const convertStringToBigInt = (obj, bigIntFields = ['amount', 'fee', 'value', 'satoshis', 'balance']) => {
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

  // Generate QR codes for UTXO data
  const generateUTXOQRCodes = async (utxoData) => {
    console.log('🔍 generateUTXOQRCodes called with utxoData:', utxoData);
    
    try {
      const { generateMultiPartQR } = await import('../../kaspa/js/qr-manager.js');
      
      // Create UTXO QR data structure
      const qrData = {
        type: 'kaspa-utxo-data',
        version: '1.0',
        addresses: utxoData.addresses,
        utxos: utxoData.utxos,
        networkType: utxoData.networkType,
        timestamp: utxoData.timestamp,
        count: utxoData.count,
        dataId: `utxo_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
      };
      
      console.log('🔍 Created QR data structure:', qrData);
      
      // Convert BigInt values to strings for JSON serialization
      const serializedQrData = convertBigIntToString(qrData);
      console.log('🔍 Serialized QR data for JSON:', serializedQrData);
      
      // Generate multi-part QR codes (UTXOs can be large) - use 'utxo-data' as baseType
      console.log('🔍 Calling generateMultiPartQR...');
      const qrResult = await generateMultiPartQR(serializedQrData, 'utxo-data');
      console.log('🔍 QR generation result:', qrResult);
      
      if (qrResult.success) {
        console.log('🔍 Setting utxoQRCodes state...');
        setUtxoQRCodes(qrResult);
      } else {
        console.error('❌ Failed to generate UTXO QR codes:', qrResult.error);
        addNotification('Failed to generate UTXO QR codes: ' + qrResult.error, 'error');
      }
      
    } catch (error) {
      console.error('❌ Error generating UTXO QR codes:', error);
      addNotification('Error generating UTXO QR codes: ' + error.message, 'error');
    }
  };

  // Show UTXO import options modal
  const showUTXOImportOptions = () => {
    setShowUTXOImport(true);
  };

  // Show custom address prompt for UTXO fetching
  const promptForCustomAddress = () => {
    setCustomAddress(walletState.address || ''); // Pre-fill with current address
    setShowCustomAddressPrompt(true);
  };

  // Handle custom address UTXO fetch
  const handleCustomAddressFetch = async () => {
    const inputValue = customAddress.trim();
    
    // For HD wallets, allow empty input to fetch from all addresses
    if (!inputValue && walletState.isHDWallet) {
      setShowCustomAddressPrompt(false);
      await fetchUTXOsForQR(); // No input = use all wallet addresses
      return;
    }
    
    if (!inputValue) {
      addNotification('Please enter a valid address or extended public key', 'error');
      return;
    }

    try {
      // Check if it's an extended public key
      if (inputValue.startsWith('xpub') || inputValue.startsWith('tpub') || inputValue.startsWith('kpub') || inputValue.startsWith('ktpub')) {
        // Set processing state for progress bar
        setIsProcessingExtendedKey(true);
        setExtendedKeyProgress({ current: 0, total: 0, status: 'Starting extended public key scan...' });
        
        // Check if we have structured xpub data from QR scan that matches this input
        let derivationPath = null;
        if (lastScannedXpubData && lastScannedXpubData.xpub === inputValue && lastScannedXpubData.derivationPath) {
          derivationPath = lastScannedXpubData.derivationPath;
        }
        
        const addressObjects = await deriveAddressesFromXpub(inputValue, derivationPath, gapLimit, startIndex, (progress) => {
          // Update progress for the UI
          setExtendedKeyProgress(progress);
        });
        
        setIsProcessingExtendedKey(false);
        
        if (!addressObjects || addressObjects.length === 0) {
          setLastScanResult({
            success: false,
            message: `No addresses with balance found in range ${startIndex} to ${startIndex + gapLimit - 1}`,
            addressesFound: 0,
            rangeScanned: { start: startIndex, end: startIndex + gapLimit - 1 }
          });
          return;
        }
        
        // Success - show results and fetch UTXOs
        setLastScanResult({
          success: true,
          message: `Found ${addressObjects.length} addresses with balance!`,
          addressesFound: addressObjects.length,
          rangeScanned: { start: startIndex, end: startIndex + gapLimit - 1 },
          addressesWithBalance: addressObjects
        });
        
        // Extract just the addresses for UTXO fetching
        const addresses = addressObjects.map(addr => addr.address);
        console.log('🔍 handleCustomAddressFetch: Found addresses with balance, calling fetchUTXOsForMultipleAddresses with:', addresses);
        await fetchUTXOsForMultipleAddresses(addresses);
        addNotification(`Successfully fetched UTXOs from ${addressObjects.length} addresses`, 'success');
        return;
      }
      
      // Check if it looks like a regular Kaspa address
      if (inputValue.startsWith('kaspa:') || inputValue.startsWith('kaspatest:')) {
        setShowCustomAddressPrompt(false);
        await fetchUTXOsForQR(inputValue);
      } else {
        addNotification('Invalid address format. Please enter a valid Kaspa address (kaspa: or kaspatest:) or extended public key.', 'error');
        return;
      }
      
    } catch (error) {
      console.error('Error processing input:', error);
      setIsProcessingExtendedKey(false);
      setLastScanResult({
        success: false,
        message: 'Error: ' + error.message,
        addressesFound: 0
      });
      addNotification('Error processing input: ' + error.message, 'error');
    }
  };

  // Handle "Next" button - increment start index and search again
  const handleNextAddressRange = async () => {
    const newStartIndex = startIndex + gapLimit;
    setStartIndex(newStartIndex);
    setLastScanResult(null);
    
    // Wait a bit for state to update, then trigger search with explicit new start index
    setTimeout(async () => {
      const inputValue = customAddress.trim();
      
      if (inputValue.startsWith('xpub') || inputValue.startsWith('tpub') || inputValue.startsWith('kpub') || inputValue.startsWith('ktpub')) {
        // Set processing state for progress bar
        setIsProcessingExtendedKey(true);
        setExtendedKeyProgress({ current: 0, total: 0, status: 'Starting extended public key scan...' });
        
        try {
          // Check if we have structured xpub data from QR scan that matches this input
          let derivationPath = null;
          if (lastScannedXpubData && lastScannedXpubData.xpub === inputValue && lastScannedXpubData.derivationPath) {
            derivationPath = lastScannedXpubData.derivationPath;
          }
          
          const addressObjects = await deriveAddressesFromXpub(inputValue, derivationPath, gapLimit, newStartIndex, (progress) => {
            // Update progress for the UI
            setExtendedKeyProgress(progress);
          });
          
          setIsProcessingExtendedKey(false);
          
          if (!addressObjects || addressObjects.length === 0) {
            setLastScanResult({
              success: false,
              message: `No addresses with balance found in range ${newStartIndex} to ${newStartIndex + gapLimit - 1}`,
              addressesFound: 0,
              rangeScanned: { start: newStartIndex, end: newStartIndex + gapLimit - 1 }
            });
            return;
          }
          
          // Success - show results and fetch UTXOs
          setLastScanResult({
            success: true,
            message: `Found ${addressObjects.length} addresses with balance!`,
            addressesFound: addressObjects.length,
            rangeScanned: { start: newStartIndex, end: newStartIndex + gapLimit - 1 },
            addressesWithBalance: addressObjects
          });
          
          // Extract just the addresses for UTXO fetching
          const addresses = addressObjects.map(addr => addr.address);
          console.log('🔍 handleNextAddressRange: Found addresses with balance, calling fetchUTXOsForMultipleAddresses with:', addresses);
          await fetchUTXOsForMultipleAddresses(addresses);
          addNotification(`Successfully fetched UTXOs from ${addressObjects.length} addresses`, 'success');
        } catch (error) {
          console.error('Error processing next range:', error);
          setIsProcessingExtendedKey(false);
          setLastScanResult({
            success: false,
            message: 'Error: ' + error.message,
            addressesFound: 0
          });
          addNotification('Error processing next range: ' + error.message, 'error');
        }
      }
    }, 100);
  };

  // Close custom address prompt
  const closeCustomAddressPrompt = () => {
    setShowCustomAddressPrompt(false);
    setCustomAddress('');
    setStartIndex(0);
    setIsProcessingExtendedKey(false);
    setExtendedKeyProgress({ current: 0, total: 0, status: '' });
    setLastScanResult(null);
  };
  
  // Clear scan result when parameters change to allow re-scanning
  const clearScanResult = () => {
    setLastScanResult(null);
    setIsProcessingExtendedKey(false);
    setExtendedKeyProgress({ current: 0, total: 0, status: '' });
  };
  
  // Reset progress state when starting new scan
  const resetProgressState = () => {
    setIsScanning(true);
    setScanProgress({ current: 0, total: 0, found: 0 });
    setLastScanResult(null);
    setScanStatus('Starting extended public key scanning...');
  };

  // Open camera QR scanner for UTXO data
  const openCameraScanner = async () => {
    try {
      const { openCameraQRScanner } = await import('../../kaspa/js/qr-manager.js');
      
      // Close the options modal first
      setShowUTXOImport(false);
      
      // Open camera scanner with callback
      await openCameraQRScanner(handleUTXOImport);
      
    } catch (error) {
      console.error('Error opening UTXO scanner:', error);
      addNotification('Error opening QR scanner: ' + error.message, 'error');
    }
  };

  // Open camera QR scanner for address/xpub input
  const openAddressQRScanner = async () => {
    try {
      const { openCameraQRScanner } = await import('../../kaspa/js/qr-manager.js');
      
      // Create a wrapper callback that just populates the input field
      const handleAddressQRScanForInput = async (qrResult) => {
        try {
          if (!qrResult.success || !qrResult.qrData) {
            throw new Error('Invalid QR code data');
          }

          // Extract the address or xpub from QR data
          let addressValue = '';
          const qrData = qrResult.qrData;
          
          if (typeof qrData === 'string') {
            addressValue = qrData.trim();
          } else if (typeof qrData === 'object') {
            if (qrData.type === 'kaspa-address' && qrData.address) {
              addressValue = qrData.address;
            } else if (qrData.type === 'kaspa-xpub' && qrData.xpub) {
              addressValue = qrData.xpub;
            } else {
              throw new Error('QR code does not contain address or extended public key data');
            }
          }
          
          if (!addressValue) {
            throw new Error('Could not extract address or extended public key from QR code');
          }
          
          // Update the input field and reopen the prompt
          setCustomAddress(addressValue);
          setShowCustomAddressPrompt(true);
          
        } catch (error) {
          console.error('Error processing address QR:', error);
          addNotification('Error processing QR code: ' + error.message, 'error');
          // Reopen the prompt on error
          setShowCustomAddressPrompt(true);
        }
      };
      
      // Close the custom address prompt first
      setShowCustomAddressPrompt(false);
      
      // Open camera scanner with the wrapper callback
      await openCameraQRScanner(handleAddressQRScanForInput);
      
    } catch (error) {
      console.error('Error opening address QR scanner:', error);
      addNotification('Error opening QR scanner: ' + error.message, 'error');
    }
  };



  // Derive addresses from extended public key using the new address scanner
  const deriveAddressesFromXpub = async (xpub, derivationPath = null, customGapLimit = null, customStartIndex = null, progressCallback = null) => {
    console.log('Deriving addresses from xpub:', xpub);
    
    // Use proper derivation path from constants if not provided
    if (!derivationPath) {
      const { DEFAULT_ACCOUNT_PATH } = await import('../../kaspa/js/constants.js');
      derivationPath = DEFAULT_ACCOUNT_PATH;
    }
    
    console.log('Using derivation path:', derivationPath);
    console.log('Network type for balance checks:', walletState.network);

    try {
      // Import the new address scanner
      const { addressScanner } = await import('../../kaspa/js/address-scanner.js');

      // Try HDWallet approach first if available
      if (walletState.isHDWallet && walletState.hdWallet && !customGapLimit && !customStartIndex) {
        console.log('Trying HDWallet approach...');
        try {
          const hdWallet = walletState.hdWallet;
          console.log('HDWallet available, checking for addresses...');
          
          const addresses = hdWallet.getAllAddresses()
            .filter(addr => addr.balance > 0n)
            .map(addr => addr.address);
          
          if (addresses.length > 0) {
            console.log('Found addresses with balances in HDWallet:', addresses);
            return addresses;
          }
        } catch (hdError) {
          console.log('HDWallet approach failed:', hdError.message);
        }
      }

      const maxGap = customGapLimit || gapLimit;
      const startIdx = customStartIndex || startIndex;

      // If no custom parameters, try targeted scanning first (like working web app)
      if (!customGapLimit && !customStartIndex) {
        const targetedResult = await addressScanner.scanXpubWithSpecificIndices(
          xpub,
          walletState.network,
          [0, 1, 2, 5, 10, 20, 50, 100], // Same indices as working web app
          progressCallback
        );

        if (targetedResult.success && targetedResult.addressesFound > 0) {
          // Return full address objects with balance information
          return targetedResult.addressesWithBalance;
        }
      }

      // Fallback to range scanning
      const scanResult = await addressScanner.scanXpubRange(
        xpub,
        walletState.network,
        maxGap,
        progressCallback
      );

      if (!scanResult.success) {
        throw new Error(scanResult.error);
      }

      // Return full address objects with balance information
      return scanResult.addressesWithBalance;
    } catch (error) {
      console.error('Error deriving addresses from xpub:', error);
      throw new Error('Failed to derive addresses from extended public key: ' + error.message);
    }
  };

  // Fetch UTXOs for multiple addresses using the new address scanner
  const fetchUTXOsForMultipleAddresses = async (addresses) => {
    console.log('🔍 fetchUTXOsForMultipleAddresses called with addresses:', addresses);
    
    if (!addresses || addresses.length === 0) {
      throw new Error('No addresses provided');
    }

    setIsLoadingUTXOs(true);
    
    try {
      const { addressScanner } = await import('../../kaspa/js/address-scanner.js');
      
      console.log('🔍 Calling addressScanner.fetchUTXOsForAddresses...');
      const utxoResult = await addressScanner.fetchUTXOsForAddresses(addresses, walletState.network);
      console.log('🔍 UTXO result:', utxoResult);
      
      if (!utxoResult.success) {
        throw new Error(utxoResult.error);
      }
      
      if (!utxoResult.utxos || utxoResult.utxos.length === 0) {
        console.log('🔍 No UTXOs found, but creating empty UTXO data structure for modal');
        
        // Still create UTXO data structure and show modal, even with 0 UTXOs
        const emptyUtxoData = {
          utxos: [],
          addresses: addresses,
          networkType: walletState.network,
          timestamp: Date.now(),
          count: 0
        };
        
        console.log('🔍 Caching empty UTXOs...');
        onCacheUTXOs(emptyUtxoData);
        setLastUTXOFetch(new Date());
        
        // Generate QR codes even for empty data (will show the structure)
        console.log('🔍 Generating QR codes for empty data...');
        await generateUTXOQRCodes(emptyUtxoData);
        
        // Show QR codes modal
        console.log('🔍 Setting showUTXOQR to true for empty data...');
        setShowUTXOQR(true);
        
        addNotification(`Address has balance but no available UTXOs found. UTXOs may have been spent or are not confirmed yet. Empty UTXO data generated for reference.`, 'info');
        return;
      }
      
      console.log('🔍 UTXOs found:', utxoResult.utxos.length);
      
      // Cache the UTXOs
      const utxoData = {
        utxos: utxoResult.utxos,
        addresses: addresses,
        networkType: walletState.network,
        timestamp: Date.now(),
        count: utxoResult.count || utxoResult.utxos.length
      };
      
      console.log('🔍 Caching UTXOs...');
      onCacheUTXOs(utxoData);
      setLastUTXOFetch(new Date());
      
      // Generate QR code(s) for the UTXO data
      console.log('🔍 Generating QR codes...');
      await generateUTXOQRCodes(utxoData);
      
      // Show QR codes
      console.log('🔍 Setting showUTXOQR to true...');
      setShowUTXOQR(true);
      
      addNotification(`Successfully fetched ${utxoData.count} UTXOs from ${addresses.length} addresses`, 'success');
      
    } catch (error) {
      console.error('❌ Error fetching UTXOs for multiple addresses:', error);
      addNotification('Error fetching UTXOs: ' + error.message, 'error');
    } finally {
      setIsLoadingUTXOs(false);
    }
  };

  // Handle file upload for address/xpub QR images
  const handleAddressQRFileUpload = async (event) => {
    const files = Array.from(event.target.files);
    if (files.length === 0) return;

    try {
      const file = files[0];
      const { readQRFromImage } = await import('../../kaspa/js/qr-manager.js');
      
      const qrResult = await readQRFromImage(file);
      
      if (!qrResult.success) {
        throw new Error(qrResult.error || 'Failed to read QR code');
      }
      
      // Extract the address or xpub from QR data and populate the input field
      let addressValue = '';
      const qrData = qrResult.qrData;
      

      
      if (typeof qrData === 'string') {
        addressValue = qrData.trim();
      } else if (typeof qrData === 'object') {
        if (qrData.type === 'kaspa-address' && qrData.address) {
          addressValue = qrData.address;
        } else if (qrData.type === 'kaspa-xpub' && qrData.xpub) {
          // The xpub might be JSON-encoded, try to parse it
          let xpubString = qrData.xpub;
          if (typeof xpubString === 'string' && xpubString.startsWith('{')) {
            try {
              const xpubObj = JSON.parse(xpubString);
              if (xpubObj.xpub) {
                xpubString = xpubObj.xpub;
              }
            } catch (parseError) {
              // Silently continue with original string
            }
          }
          
          // For structured xpub QR codes, extract the xpub string for the input field
          // Store the full QR data for later use when user clicks Fetch UTXOs
          addressValue = xpubString;
          // Store the full structured data in a state variable for later use
          setLastScannedXpubData({
            ...qrData,
            xpub: xpubString // Use the cleaned xpub string
          });
        } else {
          throw new Error('QR code does not contain address or extended public key data');
        }
      }
      
      if (!addressValue) {
        throw new Error('Could not extract address or extended public key from QR code');
      }
      
      // Update the input field with the extracted value
      setCustomAddress(addressValue);
      
    } catch (error) {
      console.error('Error reading address QR file:', error);
      addNotification('Error reading QR file: ' + error.message, 'error');
    }
    
    // Clear the file input
    event.target.value = '';
  };

  // Handle file upload for QR images
  const handleUTXOFileUpload = async (event) => {
    const files = Array.from(event.target.files);
    if (files.length === 0) return;

    try {
      // Try multi-part QR first if multiple files
      if (files.length > 1) {
        const { readMultiPartQRFromImages } = await import('../../kaspa/js/qr-manager.js');
        
        const qrResult = await readMultiPartQRFromImages(files);
        
        if (qrResult.success && qrResult.qrData) {
          await handleUTXOImport(qrResult.qrData);
          setShowUTXOImport(false);
          return;
        }
      }
      
      // Single QR code processing
      const file = files[0];
      const { readQRFromImage } = await import('../../kaspa/js/qr-manager.js');
      
      const qrResult = await readQRFromImage(file);
      
      if (!qrResult.success) {
        throw new Error(qrResult.error || 'Failed to read QR code');
      }
      
      // Use the qrData from the result
      await handleUTXOImport(qrResult.qrData);
      setShowUTXOImport(false);
      
    } catch (error) {
      console.error('Error reading UTXO QR files:', error);
      addNotification('Error reading QR files: ' + error.message, 'error');
    }
    
    // Clear the file input
    event.target.value = '';
  };

    // Handle UTXO QR scan result
  const handleUTXOImport = async (qrData) => {
    try {
      // Validate the QR data
      if (!qrData || typeof qrData !== 'object') {
        throw new Error('Invalid QR data format');
      }
      
      if (qrData.type !== 'kaspa-utxo-data') {
        throw new Error('QR code is not UTXO data. Expected kaspa-utxo-data, got: ' + qrData.type);
      }
      
      if (!qrData.utxos || !Array.isArray(qrData.utxos)) {
        throw new Error('Invalid UTXO data structure');
      }
      
      // Convert string values back to BigInt where needed (UTXO-specific fields)
      const processedQrData = convertStringToBigInt(qrData, ['amount', 'fee', 'value', 'satoshis', 'balance']);
      
      // Cache the imported UTXOs
      const utxoData = {
        utxos: processedQrData.utxos,
        addresses: processedQrData.addresses || [walletState.address],
        networkType: processedQrData.networkType || walletState.network,
        timestamp: processedQrData.timestamp || Date.now(),
        count: processedQrData.count || processedQrData.utxos.length,
        imported: true
      };
      
      onCacheUTXOs(utxoData);
      setLastUTXOFetch(new Date(utxoData.timestamp));
      
      addNotification(`Successfully imported ${utxoData.count} UTXOs from QR code`, 'success');
      
    } catch (error) {
      console.error('Error importing UTXOs:', error);
      addNotification('Error importing UTXOs: ' + error.message, 'error');
    }
  };

  // Close modals
  const closeUTXOQR = () => {
    setShowUTXOQR(false);
    setUTXOQRCodes(null);
  };

  const closeUTXOImport = () => {
    setShowUTXOImport(false);
  };

  // Download UTXOs as JSON file
  const downloadUTXOsAsJSON = () => {
    if (!cachedUTXOs || !cachedUTXOs.utxos) {
      addNotification('No UTXOs available to download', 'error');
      return;
    }

    try {
      // Create downloadable data structure
      const downloadData = {
        type: 'kaspa-utxo-data',
        version: '1.0',
        addresses: cachedUTXOs.addresses,
        utxos: cachedUTXOs.utxos,
        networkType: cachedUTXOs.networkType,
        timestamp: cachedUTXOs.timestamp,
        count: cachedUTXOs.count,
        exported: true,
        exportTimestamp: Date.now()
      };

      // Convert BigInt values to strings for JSON serialization
      const serializedData = convertBigIntToString(downloadData);
      
      // Create JSON string
      const jsonString = JSON.stringify(serializedData, null, 2);
      
      // Create blob and download
      const blob = new Blob([jsonString], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      
      const a = document.createElement('a');
      a.href = url;
      a.download = `kaspa-utxos-${cachedUTXOs.networkType}-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      
      addNotification(`Downloaded ${cachedUTXOs.count} UTXOs as JSON file`, 'success');
      
    } catch (error) {
      console.error('Error downloading UTXOs as JSON:', error);
      addNotification('Error downloading UTXOs: ' + error.message, 'error');
    }
  };

  // Handle JSON file upload for UTXO import
  const handleUTXOJSONUpload = async (event) => {
    const files = Array.from(event.target.files);
    if (files.length === 0) return;

    try {
      const file = files[0];
      
      // Check if it's a JSON file
      if (!file.name.toLowerCase().endsWith('.json')) {
        throw new Error('Please select a JSON file');
      }
      
      // Read file content
      const text = await file.text();
      const jsonData = JSON.parse(text);
      
      // Validate the JSON structure
      if (!jsonData || typeof jsonData !== 'object') {
        throw new Error('Invalid JSON file format');
      }
      
      if (jsonData.type !== 'kaspa-utxo-data') {
        throw new Error('JSON file is not UTXO data. Expected kaspa-utxo-data, got: ' + jsonData.type);
      }
      
      if (!jsonData.utxos || !Array.isArray(jsonData.utxos)) {
        throw new Error('Invalid UTXO data structure in JSON file');
      }
      
      // Convert string values back to BigInt where needed
      const processedData = convertStringToBigInt(jsonData, ['amount', 'fee', 'value', 'satoshis', 'balance']);
      
      // Cache the imported UTXOs
      const utxoData = {
        utxos: processedData.utxos,
        addresses: processedData.addresses || [walletState.address],
        networkType: processedData.networkType || walletState.network,
        timestamp: processedData.timestamp || Date.now(),
        count: processedData.count || processedData.utxos.length,
        imported: true,
        importedFromJSON: true
      };
      
      onCacheUTXOs(utxoData);
      setLastUTXOFetch(new Date(utxoData.timestamp));
      setShowUTXOImport(false);
      
      addNotification(`Successfully imported ${utxoData.count} UTXOs from JSON file`, 'success');
      
    } catch (error) {
      console.error('Error importing UTXOs from JSON:', error);
      addNotification('Error importing JSON file: ' + error.message, 'error');
    }
    
    // Clear the file input
    event.target.value = '';
  };



  // Format UTXO fetch time
  const formatUTXOFetchTime = (date) => {
    if (!date) return 'Never';
    return date.toLocaleString();
  };

  // Navigate to wallet settings for UTXO management (DRY - centralized in settings)
  const handleCompoundUTXOs = () => {
    if (!cachedUTXOs || !cachedUTXOs.utxos || cachedUTXOs.utxos.length === 0) {
      addNotification('No UTXOs available to compound', 'error');
      return;
    }

    if (!walletState.isHDWallet) {
      addNotification('Compound UTXOs is only available for HD wallets', 'error');
      return;
    }

    // Navigate to wallet settings where the centralized compound button is located
    onNavigate('wallet-settings');
    addNotification('Navigate to Wallet Settings → UTXO Management to compound UTXOs', 'info');
  };



  const startContinuousScan = async () => {
    if (!lastScannedXpubData) {
      addNotification('No XPub data available. Please scan an XPub QR code first.', 'error');
      return;
    }

    setIsContinuousScanning(true);
    setCurrentScanBatch(0);
    setTotalAddressesFound(0);
    
    try {
      await continuousScanBatch();
    } catch (error) {
      console.error('Error during continuous scan:', error);
      addNotification(`Continuous scan error: ${error.message}`, 'error');
      setIsContinuousScanning(false);
    }
  };

  const stopContinuousScan = () => {
    setIsContinuousScanning(false);
    setCurrentScanBatch(0);
    setExtendedKeyProgress({ current: 0, total: 0, status: '' });
    
    // Clear any pending timeout
    if (continuousScanTimeoutId) {
      clearTimeout(continuousScanTimeoutId);
      setContinuousScanTimeoutId(null);
    }
    
    addNotification('Continuous scan stopped', 'info');
  };

  const continuousScanBatch = async () => {
    if (!isContinuousScanning || !lastScannedXpubData) {
      return;
    }

    const batchStartIndex = startIndex + (currentScanBatch * 50);
    
    setExtendedKeyProgress({
      current: 0,
      total: 50,
      status: `Scanning batch ${currentScanBatch + 1} (indices ${batchStartIndex}-${batchStartIndex + 49})`
    });

    try {
      // Call deriveAddressesFromXpub with custom parameters for this batch
      const result = await deriveAddressesFromXpub(
        lastScannedXpubData.xpub,
        lastScannedXpubData.derivationPath,
        50, // Gap limit for this batch
        batchStartIndex, // Start index for this batch
        (progress) => {
          if (isContinuousScanning) {
            setExtendedKeyProgress(progress);
          }
        }
      );

      // Check if scan was stopped during the async operation
      if (!isContinuousScanning) {
        return;
      }

      if (result && result.length > 0) {
        const foundAddresses = result.filter(addr => addr.balance && addr.balance > 0n);
        
        if (foundAddresses.length > 0) {
          setTotalAddressesFound(prev => prev + foundAddresses.length);
          
          // Update start index to the first found address index
          const firstFoundIndex = result.findIndex(addr => addr.balance && addr.balance > 0n);
          const newStartIndex = batchStartIndex + firstFoundIndex;
          setStartIndex(newStartIndex);
          
          addNotification(`Found ${foundAddresses.length} address(es) with balance! Start index updated to ${newStartIndex}`, 'success');
          
          // Always stop when addresses are found
          setIsContinuousScanning(false);
          setExtendedKeyProgress({ 
            current: 100, 
            total: 100, 
            status: `Scan complete - Found ${foundAddresses.length} addresses with balance` 
          });
          addNotification('Continuous scan stopped - addresses found', 'info');
          return;
        }
      }

      // Continue to next batch if scanning is still active
      if (isContinuousScanning) {
        setCurrentScanBatch(prev => prev + 1);
        // Add a small delay before next batch to prevent overwhelming the RPC
        const timeoutId = setTimeout(() => {
          if (isContinuousScanning) {
            continuousScanBatch();
          }
        }, 1000);
        setContinuousScanTimeoutId(timeoutId);
      }

    } catch (error) {
      console.error('Error in continuous scan batch:', error);
      addNotification(`Scan batch error: ${error.message}`, 'error');
      setIsContinuousScanning(false);
      setExtendedKeyProgress({ current: 0, total: 0, status: 'Scan stopped due to error' });
    }
  };

  // Update the continuous scan when scanning starts
  useEffect(() => {
    if (isContinuousScanning && currentScanBatch === 0) {
      // Start the first batch
      continuousScanBatch();
    }
  }, [isContinuousScanning]);

  // Cleanup timeouts on unmount
  useEffect(() => {
    return () => {
      if (continuousScanTimeoutId) {
        clearTimeout(continuousScanTimeoutId);
      }
    };
  }, [continuousScanTimeoutId]);



  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row' },
      React.createElement('div', { className: 'col-12' },
        // Wallet Info Card
        React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-wallet2 me-2' }),
              'Wallet Overview'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row' },
              React.createElement('div', { className: 'col-md-8' },
                React.createElement('h6', { className: 'mb-2' }, 'Wallet Address:'),
                React.createElement('div', { className: 'address-display mb-3' },
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('code', { 
                      className: 'form-control text-break text-primary fw-bold',
                      style: { 
                        fontSize: '0.9em',
                        backgroundColor: 'var(--bg-secondary)',
                        color: 'var(--brand-primary)',
                        border: '1px solid var(--border-color)',
                        fontFamily: 'monospace'
                      }
                    }, 
                      walletState.address || 'Loading...'
                    ),
                    React.createElement('button', {
                      className: 'btn btn-outline-primary',
                      type: 'button',
                      onClick: copyAddress,
                      title: 'Copy address to clipboard',
                      style: { minWidth: '80px', padding: '8px 12px', fontSize: '0.9em' }
                    },
                      React.createElement('i', { className: 'bi bi-copy me-1', style: { fontSize: '1.1em' } }),
                      'Copy'
                    ),
                    React.createElement('button', {
                      className: 'btn btn-outline-secondary',
                      type: 'button',
                      onClick: showAddressQRCode,
                      title: 'Show address as QR code',
                      style: { minWidth: '80px', padding: '8px 12px', fontSize: '0.9em' }
                    },
                      React.createElement('i', { className: 'bi bi-qr-code me-1', style: { fontSize: '1.1em' } }),
                      'QR'
                    ),
                    // Generate new address button (only for HD wallets)
                    walletState.isHDWallet && React.createElement('button', {
                      className: 'btn btn-outline-success',
                      type: 'button',
                      onClick: onGenerateNewAddress,
                      title: 'Generate new receive address',
                      style: { minWidth: '80px', padding: '8px 12px', fontSize: '0.9em' }
                    },
                      React.createElement('i', { className: 'bi bi-plus-circle me-1', style: { fontSize: '1.1em' } }),
                      'New'
                    )
                  )
                ),
                // UTXO Management buttons
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('div', { className: 'row g-2' },
                    React.createElement('div', { className: 'col-sm-6' },
                      React.createElement('button', {
                        className: `btn btn-outline-info btn-sm w-100 ${isLoadingUTXOs ? 'disabled' : ''}`,
                        onClick: promptForCustomAddress,
                        disabled: isLoadingUTXOs,
                        title: 'Fetch UTXOs for a specific address or all wallet addresses'
                      },
                        isLoadingUTXOs ? 
                          React.createElement('span', null,
                            React.createElement('span', { 
                              className: 'spinner-border spinner-border-sm me-2' 
                            }),
                            'Fetching...'
                          ) :
                          React.createElement('span', null,
                            React.createElement('i', { className: 'bi bi-download me-2' }),
                            'Fetch UTXOs'
                          )
                      )
                    ),
                    React.createElement('div', { className: 'col-sm-6' },
                      React.createElement('button', {
                        className: 'btn btn-outline-warning btn-sm w-100',
                        onClick: showUTXOImportOptions,
                        title: 'Import UTXOs from QR codes for offline transaction creation'
                      },
                        React.createElement('i', { className: 'bi bi-upload me-2' }),
                        'Import UTXOs'
                      )
                    )
                  ),
                                     // Show cached UTXO info if available
                   cachedUTXOs && React.createElement('div', { className: 'mt-2' },
                     React.createElement('small', { className: 'text-muted' },
                       React.createElement('i', { className: 'bi bi-check-circle text-success me-1' }),
                       `Cached UTXOs: ${cachedUTXOs.count} | Fetched: ${formatUTXOFetchTime(lastUTXOFetch)}`,
                       cachedUTXOs.imported && React.createElement('span', { className: 'badge bg-info ms-2' }, 'Imported')
                     )
                   )
                 ),
                React.createElement('div', { className: 'row' },
                  React.createElement('div', { className: 'col-sm-6' },
                    React.createElement('h6', { className: 'mb-1' }, 'Network:'),
                    React.createElement('span', { className: 'badge bg-primary' }, 
                      walletState.network || 'mainnet'
                    )
                  ),
                  React.createElement('div', { className: 'col-sm-6' },
                    React.createElement('h6', { className: 'mb-1' }, 'Last Balance Check:'),
                    React.createElement('small', { className: 'text-muted' }, 
                      formatLastCheck(lastBalanceCheck)
                    )
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-4 text-md-end' },
                React.createElement('h6', { className: 'mb-2' }, 'Balance:'),
                React.createElement('div', { className: 'balance-amount mb-3' }, 
                  formatBalance(balance), ' KAS'
                ),
                React.createElement('div', { className: 'd-flex gap-2 justify-content-end' },
                  React.createElement('button', {
                    className: `btn btn-outline-primary btn-sm ${isCheckingBalance ? 'disabled' : ''}`,
                    onClick: checkBalance,
                    disabled: isCheckingBalance,
                    title: walletState.isHDWallet ? 'Refresh balance and discover HD addresses' : 'Refresh balance'
                  },
                    isCheckingBalance ? 
                      React.createElement('span', null,
                        React.createElement('span', { 
                          className: 'spinner-border spinner-border-sm me-2' 
                        }),
                        'Checking...'
                      ) :
                      React.createElement('span', null,
                        React.createElement('i', { className: 'bi bi-arrow-clockwise me-2' }),
                        'Refresh Balance'
                      )
                  ),
                  
                  // HD discovery is now integrated into Refresh Balance button
                )
              )
            )
          )
        ),

        // Quick Actions Card
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-lightning me-2' }),
              'Quick Actions'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row g-3' },
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('button', {
                  className: 'btn btn-primary quick-action-btn w-100',
                  onClick: () => onNavigate('transaction')
                },
                  React.createElement('i', { className: 'bi bi-arrow-up-right' }),
                  React.createElement('div', null, 'Send Transaction')
                )
              ),
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('button', {
                  className: 'btn btn-success quick-action-btn w-100',
                  onClick: () => onNavigate('message-signing')
                },
                  React.createElement('i', { className: 'bi bi-pen' }),
                  React.createElement('div', null, 'Sign Message')
                )
              ),
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('button', {
                  className: 'btn btn-info quick-action-btn w-100',
                  onClick: () => onNavigate('wallet-settings')
                },
                  React.createElement('i', { className: 'bi bi-gear' }),
                  React.createElement('div', null, 'Wallet Settings')
                )
              )
            )
          )
        )
      )
    ),

    // Address QR Code Modal
    showAddressQR && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.5)' },
      onClick: closeAddressQR
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered',
        onClick: (e) => e.stopPropagation()
      },
        React.createElement('div', { className: 'modal-content' },
          React.createElement('div', { className: 'modal-header' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-qr-code me-2' }),
              qrMode === 'xpub' ? 'Extended Public Key QR Code' : 'Wallet Address QR Code'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeAddressQR
            })
          ),
          React.createElement('div', { className: 'modal-body text-center' },
            // Mode switcher buttons (only show for HD wallets)
            walletState.isHDWallet && React.createElement('div', { className: 'btn-group mb-3', role: 'group' },
              React.createElement('button', {
                type: 'button',
                className: `btn btn-sm ${qrMode === 'address' ? 'btn-primary' : 'btn-outline-primary'}`,
                onClick: () => switchQRMode('address')
              },
                React.createElement('i', { className: 'bi bi-house me-2' }),
                'Address'
              ),
              React.createElement('button', {
                type: 'button',
                className: `btn btn-sm ${qrMode === 'xpub' ? 'btn-primary' : 'btn-outline-primary'}`,
                onClick: () => switchQRMode('xpub')
              },
                React.createElement('i', { className: 'bi bi-key me-2' }),
                'Extended Public Key'
              )
            ),
            
            // QR Code display
            (qrMode === 'address' ? addressQRCode : xpubQRCode) && React.createElement('div', { className: 'mb-3' },
              React.createElement('img', {
                src: qrMode === 'address' ? addressQRCode : xpubQRCode,
                alt: qrMode === 'address' ? 'Wallet Address QR Code' : 'Extended Public Key QR Code',
                className: 'img-fluid border rounded',
                style: { maxWidth: '300px' }
              })
            ),
            
            // Information display
            React.createElement('div', { className: 'mt-3' },
              React.createElement('small', { className: 'text-muted d-block mb-2' }, 
                qrMode === 'address' ? 'Wallet Address:' : 'Extended Public Key:'
              ),
              React.createElement('code', { 
                className: 'text-primary fw-bold d-block text-break',
                style: { fontSize: '0.8em' }
              }, 
                qrMode === 'address' ? walletState.address : (xpubString || 'Loading...')
              )
            )
          ),
          React.createElement('div', { className: 'modal-footer' },
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-outline-success',
              onClick: handleDownloadAddressQR
            },
              React.createElement('i', { className: 'bi bi-download me-2' }),
              'Download QR'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-outline-primary',
              onClick: copyAddress
            },
              React.createElement('i', { className: 'bi bi-copy me-2' }),
              'Copy Address'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-secondary',
              onClick: closeAddressQR
            }, 'Close')
          )
        )
      )
    ),

    // UTXO QR Code Modal
    showUTXOQR && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.5)', zIndex: 9999 },
      onClick: closeUTXOQR,
      ref: (el) => {
        if (el) {
          console.log('🔍 UTXO modal DOM element created:', el);
        }
      }
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered',
        onClick: (e) => e.stopPropagation()
      },
        React.createElement('div', { className: 'modal-content' },
          React.createElement('div', { className: 'modal-header' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-qr-code me-2' }),
              'UTXO QR Codes'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeUTXOQR
            })
          ),
                     React.createElement('div', { className: 'modal-body text-center' },
            // Show helpful message when no UTXOs
            cachedUTXOs && cachedUTXOs.count === 0 && React.createElement('div', { className: 'mb-3' },
              React.createElement('div', { className: 'alert alert-info' },
                React.createElement('h6', { className: 'alert-heading' },
                  React.createElement('i', { className: 'bi bi-info-circle me-2' }),
                  'No UTXOs Available'
                ),
                React.createElement('p', { className: 'mb-2' },
                  'The address has a balance but no available UTXOs were found. This typically means:'
                ),
                React.createElement('ul', { className: 'text-start mb-2' },
                  React.createElement('li', null, 'UTXOs may have been spent recently'),
                  React.createElement('li', null, 'Transactions are still pending confirmation'),
                  React.createElement('li', null, 'The wallet needs to be refreshed')
                ),
                React.createElement('p', { className: 'mb-0 text-muted' },
                  'Empty UTXO data structure has been generated for reference.'
                )
              )
            ),
            utxoQRCodes && React.createElement('div', { className: 'mb-3' },
              utxoQRCodes.isMultiPart ? 
                // Multi-part QR codes
                React.createElement('div', null,
                  React.createElement('div', { className: 'mb-2' },
                    React.createElement('span', { className: 'badge bg-info' },
                      `Multi-part QR: ${utxoQRCodes.totalParts} parts`
                    )
                  ),
                  React.createElement('div', { 
                    id: 'utxo-qr-display',
                    style: { minHeight: '300px' }
                  })
                ) :
                // Single QR code
                React.createElement('img', {
                  src: utxoQRCodes.qrDataURL,
                  alt: 'UTXO QR Code',
                  className: 'img-fluid border rounded',
                  style: { maxWidth: '300px' }
                })
            ),
            React.createElement('div', { className: 'mt-3' },
              React.createElement('small', { className: 'text-muted d-block mb-2' }, 
                `UTXO Data (${cachedUTXOs ? cachedUTXOs.count : 0} UTXOs)`
              ),
              React.createElement('code', { 
                className: 'text-primary fw-bold d-block text-break',
                style: { fontSize: '0.8em' }
              }, 
                formatUTXOFetchTime(lastUTXOFetch)
              )
            )
          ),
                     React.createElement('div', { className: 'modal-footer justify-content-center' },
             React.createElement('div', { className: 'btn-group', role: 'group' },
               // Compound UTXOs button (only show if we have multiple UTXOs)
               cachedUTXOs && cachedUTXOs.utxos && cachedUTXOs.utxos.length > 1 && React.createElement('button', {
                 type: 'button',
                 className: 'btn btn-warning',
                 onClick: () => {
                   closeUTXOQR();
                   handleCompoundUTXOs();
                 },
                 title: `Consolidate ${cachedUTXOs.utxos.length} UTXOs into 1 UTXO to reduce future transaction fees`
               },
                 React.createElement('i', { className: 'bi bi-arrow-down-up me-2' }),
                 'Compound UTXOs'
               ),
               React.createElement('button', {
                 type: 'button',
                 className: 'btn btn-outline-success',
                 onClick: () => {
                   if (utxoQRCodes && utxoQRCodes.qrDataURL) {
                     const link = document.createElement('a');
                     link.href = utxoQRCodes.qrDataURL;
                     link.download = `utxos_${Date.now()}.png`;
                     link.click();
                   }
                 }
               },
                 React.createElement('i', { className: 'bi bi-download me-2' }),
                 'Download QR'
               ),
               React.createElement('button', {
                 type: 'button',
                 className: 'btn btn-outline-info',
                 onClick: downloadUTXOsAsJSON
               },
                 React.createElement('i', { className: 'bi bi-file-earmark-code me-2' }),
                 'Download JSON'
               ),
               React.createElement('button', {
                 type: 'button',
                 className: 'btn btn-outline-primary',
                 onClick: fetchUTXOsForQR
               },
                 React.createElement('i', { className: 'bi bi-arrow-clockwise me-2' }),
                 'Refresh UTXOs'
               ),
               React.createElement('button', {
                 type: 'button',
                 className: 'btn btn-secondary',
                 onClick: closeUTXOQR
               }, 'Close')
             )
           )
        )
      )
    ),



    // UTXO Import Options Modal
    showUTXOImport && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.5)' },
      onClick: closeUTXOImport
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered',
        onClick: (e) => e.stopPropagation()
      },
        React.createElement('div', { className: 'modal-content' },
          React.createElement('div', { className: 'modal-header' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-upload me-2' }),
              'Import UTXO Data'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeUTXOImport
            })
          ),
          React.createElement('div', { className: 'modal-body' },
            React.createElement('p', { className: 'text-muted mb-4' },
              'Choose how to import your UTXO data:'
            ),
            React.createElement('div', { className: 'row g-3' },
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('div', { className: 'card h-100 border-primary' },
                  React.createElement('div', { className: 'card-body text-center' },
                    React.createElement('i', { 
                      className: 'bi bi-camera text-primary mb-3',
                      style: { fontSize: '2rem' }
                    }),
                    React.createElement('h6', { className: 'card-title' }, 'Scan QR Code'),
                    React.createElement('p', { className: 'card-text small text-muted' },
                      'Use your device camera to scan QR codes containing UTXO data'
                    ),
                    React.createElement('button', {
                      className: 'btn btn-primary btn-sm',
                      onClick: openCameraScanner
                    },
                      React.createElement('i', { className: 'bi bi-camera me-2' }),
                      'Open Camera'
                    )
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('div', { className: 'card h-100 border-secondary' },
                  React.createElement('div', { className: 'card-body text-center' },
                    React.createElement('i', { 
                      className: 'bi bi-file-earmark-image text-secondary mb-3',
                      style: { fontSize: '2rem' }
                    }),
                    React.createElement('h6', { className: 'card-title' }, 'Upload QR Images'),
                    React.createElement('p', { className: 'card-text small text-muted' },
                      'Upload saved QR code images from your device'
                    ),
                    React.createElement('label', {
                      className: 'btn btn-secondary btn-sm',
                      htmlFor: 'utxo-file-input'
                    },
                      React.createElement('i', { className: 'bi bi-upload me-2' }),
                      'Choose Files'
                    ),
                    React.createElement('input', {
                      id: 'utxo-file-input',
                      type: 'file',
                      accept: 'image/*',
                      multiple: true,
                      style: { display: 'none' },
                      onChange: handleUTXOFileUpload
                    })
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-4' },
                React.createElement('div', { className: 'card h-100 border-success' },
                  React.createElement('div', { className: 'card-body text-center' },
                    React.createElement('i', { 
                      className: 'bi bi-file-earmark-code text-success mb-3',
                      style: { fontSize: '2rem' }
                    }),
                    React.createElement('h6', { className: 'card-title' }, 'Import JSON File'),
                    React.createElement('p', { className: 'card-text small text-muted' },
                      'Upload a JSON file containing UTXO data'
                    ),
                    React.createElement('label', {
                      className: 'btn btn-success btn-sm',
                      htmlFor: 'utxo-json-input'
                    },
                      React.createElement('i', { className: 'bi bi-file-earmark-code me-2' }),
                      'Choose JSON'
                    ),
                    React.createElement('input', {
                      id: 'utxo-json-input',
                      type: 'file',
                      accept: '.json,application/json',
                      style: { display: 'none' },
                      onChange: handleUTXOJSONUpload
                    })
                  )
                )
              )
            )
          ),
          React.createElement('div', { className: 'modal-footer' },
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-secondary',
              onClick: closeUTXOImport
            }, 'Cancel')
                    )
        )
      )
    ),

    // Custom Address Prompt Modal
    showCustomAddressPrompt && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.5)' },
      onClick: closeCustomAddressPrompt
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered',
        onClick: (e) => e.stopPropagation()
      },
        React.createElement('div', { className: 'modal-content' },
          React.createElement('div', { className: 'modal-header' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-wallet2 me-2' }),
              'Fetch UTXOs for Address'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeCustomAddressPrompt
            })
          ),
          React.createElement('div', { className: 'modal-body' },
            // Network Selection
            React.createElement('div', { className: 'mb-3' },
              React.createElement('label', { 
                htmlFor: 'network-select',
                className: 'form-label' 
              }, 'Network:'),
              React.createElement('select', {
                id: 'network-select',
                className: 'form-select',
                value: selectedNetwork,
                onChange: (e) => {
                  setSelectedNetwork(e.target.value);
                  clearScanResult();
                }
              },
                React.createElement('option', { value: 'mainnet' }, 'Mainnet'),
                React.createElement('option', { value: 'testnet-10' }, 'Testnet-10'),
                React.createElement('option', { value: 'testnet-11' }, 'Testnet-11'),
                React.createElement('option', { value: 'simnet' }, 'Simnet'),
                React.createElement('option', { value: 'devnet' }, 'Devnet')
              ),
              React.createElement('div', { className: 'form-text small' },
                `Currently selected: ${selectedNetwork}. `,
                selectedNetwork !== walletState.network && 
                  React.createElement('span', { className: 'text-warning' },
                    `⚠️ Different from wallet network (${walletState.network})`
                  )
              )
            ),
            React.createElement('div', { className: 'mb-3' },
              React.createElement('label', { 
                htmlFor: 'custom-address-input',
                className: 'form-label' 
              }, 'Kaspa Address or Extended Public Key:'),
              React.createElement('div', { className: 'input-group' },
                React.createElement('input', {
                  id: 'custom-address-input',
                  type: 'text',
                  className: 'form-control',
                  value: customAddress,
                  onChange: (e) => setCustomAddress(e.target.value),
                  placeholder: 'Enter Kaspa address or extended public key...',
                  autoFocus: true
                }),
                React.createElement('button', {
                  className: 'btn btn-outline-primary',
                  type: 'button',
                  onClick: openAddressQRScanner,
                  title: 'Scan QR code for address or extended public key'
                },
                  React.createElement('i', { className: 'bi bi-camera' })
                ),
                React.createElement('button', {
                  className: 'btn btn-outline-secondary',
                  type: 'button',
                  onClick: () => document.getElementById('address-qr-file-input').click(),
                  title: 'Upload QR code image'
                },
                  React.createElement('i', { className: 'bi bi-upload me-1' }),
                  React.createElement('span', { className: 'small' }, '📤')
                ),
                React.createElement('input', {
                  id: 'address-qr-file-input',
                  type: 'file',
                  accept: 'image/*',
                  style: { display: 'none' },
                  onChange: handleAddressQRFileUpload
                })
              )
            ),
            React.createElement('div', { className: 'row' },
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { 
                    htmlFor: 'start-index-input',
                    className: 'form-label small' 
                  }, 'Start Index:'),
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('input', {
                      id: 'start-index-input',
                      type: 'number',
                      className: 'form-control form-control-sm',
                      value: startIndex,
                      onChange: (e) => {
                        setStartIndex(parseInt(e.target.value) || 0);
                        clearScanResult();
                      },
                      min: 0,
                      max: 1000000,
                      placeholder: '0'
                    }),
                    React.createElement('span', { className: 'input-group-text small' }, 'index')
                  ),
                  React.createElement('div', { className: 'form-text small' },
                    'Address index to start scanning from (useful for continuing from where you left off)'
                  )
                )
              ),
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { 
                    htmlFor: 'gap-limit-input',
                    className: 'form-label small' 
                  }, 'Gap Limit:'),
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('input', {
                      id: 'gap-limit-input',
                      type: 'number',
                      className: 'form-control form-control-sm',
                      value: gapLimit,
                      onChange: (e) => {
                        setGapLimit(parseInt(e.target.value) || 20);
                        clearScanResult();
                      },
                      min: 1,
                      max: 1000000,
                      placeholder: '20'
                    }),
                    React.createElement('span', { className: 'input-group-text small' }, 'addresses')
                  ),
                  React.createElement('div', { className: 'form-text small' },
                    'Number of consecutive empty addresses before stopping scan'
                  )
                )
              )
            ),
            React.createElement('div', { className: 'mb-3' },
              React.createElement('div', { className: 'alert alert-info py-2' },
                React.createElement('small', null,
                  React.createElement('strong', null, 'Extended Public Key Scanning: '),
                  'Set start index to continue from a specific address number (e.g., if you already scanned 0-99, set start index to 100). ',
                  'Gap limit determines how many consecutive empty addresses to check before stopping. ',
                  React.createElement('strong', null, 'Tip: '), 
                  'For fastest results when scanning your own wallet, use the "Discover HD" button instead.'
                )
              )
            ),

            // Continuous Scanning Section
            React.createElement('div', { className: 'mb-3' },
              React.createElement('div', { className: 'card border-secondary' },
                React.createElement('div', { className: 'card-header py-2' },
                  React.createElement('h6', { className: 'card-title mb-0' },
                    React.createElement('i', { className: 'bi bi-arrow-repeat me-2' }),
                    'Continuous Scanning',
                    lastScannedXpubData && React.createElement('span', { className: 'badge bg-success ms-2' }, 'XPub Ready')
                  )
                ),
                React.createElement('div', { className: 'card-body py-3' },
                  // Progress bar for continuous scanning
                  isContinuousScanning && React.createElement('div', { className: 'mb-3' },
                    React.createElement('div', { className: 'progress mb-2', style: { height: '10px' } },
                      React.createElement('div', { 
                        className: 'progress-bar progress-bar-striped progress-bar-animated bg-info',
                        style: { 
                          width: extendedKeyProgress.total > 0 ? 
                            `${Math.round((extendedKeyProgress.current / extendedKeyProgress.total) * 100)}%` : 
                            '100%'
                        } 
                      })
                    ),
                    React.createElement('small', { className: 'text-muted d-block' }, 
                      extendedKeyProgress.status || 'Scanning...'
                    )
                  ),
                  React.createElement('div', { className: 'row align-items-center' },
                    React.createElement('div', { className: 'col-md-8' },
                                             React.createElement('button', {
                         type: 'button',
                         className: `btn ${isContinuousScanning ? 'btn-danger' : 'btn-success'} me-2`,
                         onClick: isContinuousScanning ? stopContinuousScan : startContinuousScan,
                         disabled: !lastScannedXpubData && !isContinuousScanning
                       },
                        React.createElement('i', { 
                          className: `bi bi-${isContinuousScanning ? 'stop-circle' : 'play-circle'} me-2` 
                        }),
                        isContinuousScanning ? 'Stop Scan' : 'Start Continuous Scan'
                      ),
                      isContinuousScanning && React.createElement('span', { className: 'text-muted small' },
                        `Batch ${currentScanBatch + 1} • Found ${totalAddressesFound} addresses`
                      )
                    ),
                    React.createElement('div', { className: 'col-md-4 text-end' },
                      !lastScannedXpubData && React.createElement('small', { className: 'text-warning' },
                        React.createElement('i', { className: 'bi bi-exclamation-triangle me-1' }),
                        'Scan XPub first'
                      )
                    )
                  ),
                  React.createElement('div', { className: 'mt-2' },
                    React.createElement('small', { className: 'text-muted' },
                      React.createElement('strong', null, 'How it works: '),
                      'Automatically stops when addresses with balance are found. Start Index will be updated to the first found address. Scans in batches of 50 addresses with live progress updates.'
                    )
                  )
                )
              )
            ),
            
            // Progress Bar (show when processing extended key)
            isProcessingExtendedKey && React.createElement('div', { className: 'mb-3' },
              React.createElement('div', { className: 'card border-info' },
                React.createElement('div', { className: 'card-body py-3' },
                  React.createElement('h6', { className: 'card-title text-info mb-2' },
                    React.createElement('i', { className: 'bi bi-hourglass-split me-2' }),
                    'Processing Extended Public Key...'
                  ),
                  React.createElement('div', { className: 'progress mb-2', style: { height: '8px' } },
                    React.createElement('div', { 
                      className: 'progress-bar progress-bar-striped progress-bar-animated',
                      style: { 
                        width: extendedKeyProgress.total > 0 ? 
                          `${Math.round((extendedKeyProgress.current / extendedKeyProgress.total) * 100)}%` : 
                          '100%'
                      } 
                    })
                  ),
                  React.createElement('small', { className: 'text-muted' }, 
                    extendedKeyProgress.status || 'Processing...'
                  )
                )
              )
            ),
            
            // Results Display (show when scan is complete)
            lastScanResult && React.createElement('div', { className: 'mb-3' },
              React.createElement('div', { 
                className: `card border-${lastScanResult.success ? 'success' : 'warning'}` 
              },
                React.createElement('div', { className: 'card-body py-3' },
                  React.createElement('h6', { 
                    className: `card-title text-${lastScanResult.success ? 'success' : 'warning'} mb-2` 
                  },
                    React.createElement('i', { 
                      className: `bi bi-${lastScanResult.success ? 'check-circle' : 'exclamation-triangle'} me-2` 
                    }),
                    lastScanResult.success ? 'Scan Complete!' : 'No Addresses Found'
                  ),
                  React.createElement('p', { className: 'card-text mb-2' }, lastScanResult.message),
                  lastScanResult.rangeScanned && React.createElement('small', { className: 'text-muted' },
                    `Scanned range: ${lastScanResult.rangeScanned.start} - ${lastScanResult.rangeScanned.end}`
                  ),
                  
                  // Show "Next" button if no addresses found
                  !lastScanResult.success && lastScanResult.rangeScanned && React.createElement('div', { className: 'mt-3' },
                    React.createElement('button', {
                      type: 'button',
                      className: 'btn btn-outline-primary btn-sm me-2',
                      onClick: handleNextAddressRange,
                      disabled: isProcessingExtendedKey
                    },
                      React.createElement('i', { className: 'bi bi-arrow-right me-1' }),
                      `Next Range (${startIndex + gapLimit} - ${startIndex + gapLimit * 2 - 1})`
                    ),
                    React.createElement('button', {
                      type: 'button',
                      className: 'btn btn-outline-secondary btn-sm',
                      onClick: closeCustomAddressPrompt
                    },
                      'Close'
                    )
                  )
                )
              )
            ),
            
            React.createElement('div', { className: 'text-muted small' },
              walletState.isHDWallet ? 
                'Leave empty to fetch UTXOs from all wallet addresses with balances, enter a specific address, or scan/upload a QR code containing an address or extended public key.' :
                'Enter the address to fetch UTXOs for, or scan/upload a QR code. Your current address is pre-filled.'
            )
          ),
          React.createElement('div', { className: 'modal-footer' },
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-secondary',
              onClick: closeCustomAddressPrompt
            }, 'Cancel'),
            
            // Always show main fetch button
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-primary',
              onClick: handleCustomAddressFetch,
              disabled: isProcessingExtendedKey || (!customAddress.trim() && !walletState.isHDWallet)
            },
              isProcessingExtendedKey ? 
                React.createElement('span', null,
                  React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                  'Processing...'
                ) :
                React.createElement('span', null,
                  React.createElement('i', { className: 'bi bi-download me-2' }),
                  customAddress.trim() ? 'Fetch UTXOs' : (walletState.isHDWallet ? 'Fetch All UTXOs' : 'Fetch UTXOs')
                )
            ),
            
            // Show "Compound UTXOs" and "Close" buttons when results are displayed and successful
            lastScanResult && lastScanResult.success && React.createElement('div', { className: 'btn-group' },
              // Compound UTXOs button (only show if we have cached UTXOs with multiple inputs)
              cachedUTXOs && cachedUTXOs.utxos && cachedUTXOs.utxos.length > 1 && React.createElement('button', {
                type: 'button',
                className: 'btn btn-warning',
                onClick: handleCompoundUTXOs,
                title: `Consolidate ${cachedUTXOs.utxos.length} UTXOs into 1 UTXO to reduce future transaction fees`
              },
                React.createElement('i', { className: 'bi bi-arrow-down-up me-2' }),
                `Compound ${cachedUTXOs.utxos.length} UTXOs`
              ),
              // Close button
              React.createElement('button', {
                type: 'button',
                className: 'btn btn-success',
                onClick: closeCustomAddressPrompt
              },
                React.createElement('i', { className: 'bi bi-check-circle me-2' }),
                'Close'
              )
            )
          )
        )
      )
    )
  );
} 