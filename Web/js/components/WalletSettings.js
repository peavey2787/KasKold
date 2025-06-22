const { useState, useEffect } = React;



export function WalletSettings({ walletState, onNavigate, addNotification, onGenerateNewAddress, sessionManager }) {
  const [walletLabel, setWalletLabel] = useState(walletState.currentWallet?.name || '');
  const [originalWalletLabel, setOriginalWalletLabel] = useState('');
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmNewPassword, setConfirmNewPassword] = useState('');
  const [sessionTimeout, setSessionTimeout] = useState(30);
  const [autoLogoutEnabled, setAutoLogoutEnabled] = useState(true);
  const [persistSession, setPersistSession] = useState(true);
  const [isLoadingMnemonic, setIsLoadingMnemonic] = useState(false);
  const [showMnemonic, setShowMnemonic] = useState(false);
  const [mnemonic, setMnemonic] = useState('');
  const [mnemonicAvailable, setMnemonicAvailable] = useState(false);
  const [isScanningAddresses, setIsScanningAddresses] = useState(false);
  const [addressScanResults, setAddressScanResults] = useState(null);
  const [isUpdatingLabel, setIsUpdatingLabel] = useState(false);
  const [isChangingPassword, setIsChangingPassword] = useState(false);
  const [isGeneratingAddress, setIsGeneratingAddress] = useState(false);
  const [scannedAddresses, setScannedAddresses] = useState([]);
  const [generatedAddresses, setGeneratedAddresses] = useState([]);
  const [scanRange, setScanRange] = useState({ start: 0, end: 10 });
  const [addressType, setAddressType] = useState('receive'); // 'receive' or 'change'
  const [validationErrors, setValidationErrors] = useState({});
  const [mnemonicPassword, setMnemonicPassword] = useState('');
  const [isRevealingMnemonic, setIsRevealingMnemonic] = useState(false);
  const [revealedMnemonic, setRevealedMnemonic] = useState('');
  const [walletHasMnemonic, setWalletHasMnemonic] = useState(null); // null = checking, true/false = result
  const [sessionSettings, setSessionSettings] = useState({ timeoutMinutes: 0, autoSave: true });
  const [isUpdatingSession, setIsUpdatingSession] = useState(false);

  // Password strength calculation
  const calculatePasswordStrength = (password) => {
    let score = 0;
    if (password.length >= 8) score++;
    if (password.length >= 12) score++;
    if (/[a-z]/.test(password)) score++;
    if (/[A-Z]/.test(password)) score++;
    if (/[0-9]/.test(password)) score++;
    if (/[^A-Za-z0-9]/.test(password)) score++;
    
    if (score < 3) return { strength: 'weak', color: 'danger' };
    if (score < 5) return { strength: 'fair', color: 'warning' };
    if (score < 6) return { strength: 'good', color: 'info' };
    return { strength: 'strong', color: 'success' };
  };

  const passwordStrength = calculatePasswordStrength(newPassword);

  // Check if wallet has mnemonic on component mount
  useEffect(() => {
    const checkMnemonicAvailability = async () => {
      const mnemonic = await getMnemonic();
      setWalletHasMnemonic(mnemonic && mnemonic !== 'NEEDS_RELOGIN');
    };
    
    if (walletState.currentWallet) {
      checkMnemonicAvailability();
    }
  }, [walletState.currentWallet]);

  // Load session settings on component mount
  useEffect(() => {
    const loadSessionSettings = () => {
      try {
        if (sessionManager) {
          const settings = sessionManager.getSettings();
          setSessionSettings(settings);
        }
      } catch (error) {
        console.error('Failed to load session settings:', error);
      }
    };
    
    loadSessionSettings();
  }, [sessionManager]);

  // Update wallet label
  const handleUpdateLabel = async (e) => {
    e.preventDefault();
    
    if (!walletLabel.trim()) {
      setValidationErrors({ walletLabel: 'Wallet label cannot be empty' });
      return;
    }

    setIsUpdatingLabel(true);
    setValidationErrors({});

    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      // Generate wallet ID from address and network to ensure consistency
      const walletId = walletStorage.generateWalletId(walletState.address, walletState.network);
      
      const success = await walletStorage.updateWalletLabel(walletId, walletLabel.trim());
      
      if (success) {
        addNotification('Wallet label updated successfully', 'success');
        // Update the current wallet state if possible
        if (walletState.currentWallet) {
          walletState.currentWallet.name = walletLabel.trim();
        }
      } else {
        throw new Error('Failed to update wallet label');
      }
    } catch (error) {
      console.error('Label update error:', error);
      addNotification('Failed to update wallet label: ' + error.message, 'error');
    } finally {
      setIsUpdatingLabel(false);
    }
  };

  // Change wallet password
  const handleChangePassword = async (e) => {
    e.preventDefault();
    
    const errors = {};
    if (!currentPassword) errors.currentPassword = 'Current password is required';
    if (!newPassword) errors.newPassword = 'New password is required';
    if (newPassword.length < 8) errors.newPassword = 'Password must be at least 8 characters';
    if (newPassword !== confirmNewPassword) errors.confirmNewPassword = 'Passwords do not match';

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    setIsChangingPassword(true);
    setValidationErrors({});

    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      // Generate wallet ID from address and network to ensure consistency
      const walletId = walletStorage.generateWalletId(walletState.address, walletState.network);
      
      // First verify current password by trying to decrypt
      await walletStorage.decryptWallet(walletId, currentPassword);
      
      // Get current wallet data
      const currentWalletData = await walletStorage.decryptWallet(walletId, currentPassword);
      
      // Delete old wallet
      await walletStorage.deleteWallet(walletId);
      
      // Save with new password
      const walletData = {
        privateKey: currentWalletData.privateKey,
        address: currentWalletData.address,
        network: currentWalletData.network,
        mnemonic: currentWalletData.mnemonic,
        derivationPath: currentWalletData.derivationPath
      };
      
      const newWalletId = await walletStorage.saveWallet(walletData, newPassword);
      
      // Update label if it exists
      if (currentWalletData.label) {
        await walletStorage.updateWalletLabel(newWalletId, currentWalletData.label);
      }
      
      // Update the current wallet state with new ID (should be the same since we use address+network)
      if (walletState.currentWallet) {
        walletState.currentWallet.id = newWalletId;
      }
      
      addNotification('Password changed successfully', 'success');
      setCurrentPassword('');
      setNewPassword('');
      setConfirmNewPassword('');
      
    } catch (error) {
      console.error('Password change error:', error);
      if (error.message.includes('decrypt')) {
        setValidationErrors({ currentPassword: 'Current password is incorrect' });
      } else {
        addNotification('Failed to change password: ' + error.message, 'error');
      }
    } finally {
      setIsChangingPassword(false);
    }
  };

  // Get mnemonic from wallet (decrypt if needed)
  const getMnemonic = async () => {
    // First try to get from current wallet state
    if (walletState.currentWallet?.mnemonic) {
      return walletState.currentWallet.mnemonic;
    }

    // If not available in state, try to decrypt and check if wallet has mnemonic
    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      // Generate wallet ID from address and network to ensure consistency
      const walletId = walletStorage.generateWalletId(walletState.address, walletState.network);
      
      // Get wallet info to check if it has mnemonic
      const wallet = await walletStorage.getWallet(walletId);
      if (!wallet || !wallet.encryptedMnemonic) {
        return null; // Wallet doesn't have mnemonic (created from private key)
      }
      
      // Wallet has mnemonic but not in current state - need re-login
      return 'NEEDS_RELOGIN';
    } catch (error) {
      console.error('Error checking mnemonic availability:', error);
      return null;
    }
  };

  // Scan for addresses in a range
  const handleScanAddresses = async () => {
    const mnemonic = await getMnemonic();
    if (!mnemonic) {
      addNotification('This wallet was created from a private key and does not have a mnemonic phrase. Only HD wallets created from mnemonic phrases support address scanning.', 'warning');
      return;
    }
    if (mnemonic === 'NEEDS_RELOGIN') {
      addNotification('Mnemonic not available in current session. Please log out and log back in to access HD wallet features.', 'warning');
      return;
    }

    setIsScanningAddresses(true);

    try {
      const { getKaspa } = await import('../../kaspa/js/init.js');
      const kaspa = getKaspa();
      const { Mnemonic, XPrv, NewAddressKind } = kaspa;

      // Recreate wallet from mnemonic
      const mnemonicObj = new Mnemonic(mnemonic);
      const seed = mnemonicObj.toSeed();
      const xPrv = new XPrv(seed);

      const addresses = [];
      const addressKind = addressType === 'receive' ? NewAddressKind.Receive : NewAddressKind.Change;

      for (let i = scanRange.start; i <= scanRange.end; i++) {
        try {
          // Use the appropriate derivation path for the address type
          const basePath = walletState.currentWallet.derivationPath || "m/44'/111111'/0'";
          const fullPath = `${basePath}/${addressType === 'receive' ? '0' : '1'}/${i}`;
          
          const derivedXPrv = xPrv.derivePath(fullPath);
          const privateKey = derivedXPrv.toPrivateKey();
          const address = privateKey.toPublicKey().toAddress(walletState.network).toString();
          
          addresses.push({
            index: i,
            address: address,
            type: addressType,
            derivationPath: fullPath
          });
        } catch (error) {
          console.error(`Error generating address at index ${i}:`, error);
        }
      }

      setScannedAddresses(addresses);
      addNotification(`Scanned ${addresses.length} ${addressType} addresses`, 'success');

    } catch (error) {
      console.error('Address scanning error:', error);
      addNotification('Failed to scan addresses: ' + error.message, 'error');
    } finally {
      setIsScanningAddresses(false);
    }
  };

  // Generate a new address using the HD wallet manager
  const handleGenerateNewAddress = async () => {
    if (!walletState.isHDWallet) {
      addNotification('This wallet was created from a private key and does not have a mnemonic phrase. Only HD wallets created from mnemonic phrases support address generation.', 'warning');
      return;
    }

    setIsGeneratingAddress(true);

    try {
      const newAddress = await onGenerateNewAddress();
      
      if (newAddress) {
        const addressEntry = {
          index: newAddress.index,
          address: newAddress.address,
          type: newAddress.type,
          derivationPath: newAddress.derivationPath,
          generated: new Date().toISOString()
        };

        setGeneratedAddresses(prev => [...prev, addressEntry]);
      }

    } catch (error) {
      console.error('Address generation error:', error);
      addNotification('Failed to generate new address: ' + error.message, 'error');
    } finally {
      setIsGeneratingAddress(false);
    }
  };

  // Show mnemonic with password verification
  const handleShowMnemonic = async (e) => {
    e.preventDefault();
    
    if (!mnemonicPassword) {
      setValidationErrors({ mnemonicPassword: 'Password is required to reveal mnemonic' });
      return;
    }

    setIsRevealingMnemonic(true);
    setValidationErrors({});

    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      // Generate wallet ID from address and network to ensure consistency
      const walletId = walletStorage.generateWalletId(walletState.address, walletState.network);
      
      // Decrypt wallet to get mnemonic
      const decryptedWallet = await walletStorage.decryptWallet(walletId, mnemonicPassword);
      
      if (decryptedWallet && decryptedWallet.mnemonic) {
        setRevealedMnemonic(decryptedWallet.mnemonic);
        setShowMnemonic(true);
        addNotification('Mnemonic revealed successfully', 'success');
      } else {
        addNotification('This wallet was created from a private key and does not have a mnemonic phrase. Only HD wallets created from mnemonic phrases support address scanning and generation.', 'warning');
      }
      
    } catch (error) {
      console.error('Mnemonic reveal error:', error);
      if (error.message.includes('decrypt')) {
        setValidationErrors({ mnemonicPassword: 'Password is incorrect' });
      } else {
        addNotification('Failed to reveal mnemonic: ' + error.message, 'error');
      }
    } finally {
      setIsRevealingMnemonic(false);
    }
  };

  const hideMnemonic = () => {
    setShowMnemonic(false);
    setRevealedMnemonic('');
    setMnemonicPassword('');
  };

  // Copy address to clipboard
  const copyToClipboard = async (text, label) => {
    try {
      await navigator.clipboard.writeText(text);
      addNotification(`${label} copied to clipboard`, 'success');
    } catch (error) {
      addNotification('Failed to copy to clipboard', 'error');
    }
  };

  // Update session settings
  const handleUpdateSessionSettings = async (e) => {
    e.preventDefault();
    setIsUpdatingSession(true);

    try {
      if (!sessionManager) {
        throw new Error('Session manager not available');
      }
      
      // Save new settings
      sessionManager.saveSettings(sessionSettings);
      
      // If timeout was changed, restart the session with new timeout
      if (sessionSettings.timeoutMinutes > 0) {
        sessionManager.startTimeout(sessionSettings.timeoutMinutes);
        addNotification(`Session timeout updated to ${sessionSettings.timeoutMinutes} minutes`, 'success');
        
        // Force save current session if user is logged in
        if (walletState.isLoggedIn) {
          sessionManager.saveSession(walletState);
        }
      } else {
        sessionManager.stopTimeout();
        sessionManager.clearSession(); // Clear existing session if timeout disabled
        addNotification('Session timeout disabled - you will be logged out on page refresh', 'info');
      }
      
    } catch (error) {
      console.error('Session settings update error:', error);
      addNotification('Failed to update session settings: ' + error.message, 'error');
    } finally {
      setIsUpdatingSession(false);
    }
  };

  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-10' },
        
        // Header
        React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('div', { className: 'd-flex justify-content-between align-items-center' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-gear me-2' }),
              'Wallet Settings'
              ),
              React.createElement('button', {
                className: 'btn btn-outline-secondary btn-sm',
                onClick: () => onNavigate('wallet-dashboard')
              },
                React.createElement('i', { className: 'bi bi-arrow-left me-1' }),
                'Back to Dashboard'
              )
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row' },
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('p', { className: 'mb-1' },
                  React.createElement('strong', null, 'Current Wallet: '),
                  walletState.currentWallet?.name || 'Unnamed Wallet'
                ),
                React.createElement('p', { className: 'mb-1' },
                  React.createElement('strong', null, 'Address: '),
                  React.createElement('code', { className: 'text-primary' }, 
                    walletState.address?.substring(0, 20) + '...'
                  )
                ),
                React.createElement('p', { className: 'mb-0' },
                  React.createElement('strong', null, 'Network: '),
                  React.createElement('span', { className: 'badge bg-primary' }, walletState.network)
                )
              ),
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('p', { className: 'mb-1' },
                  React.createElement('strong', null, 'Wallet Type: '),
                  walletState.isHDWallet ? 
                    React.createElement('span', { className: 'badge bg-success' }, 'HD Wallet (Mnemonic)') :
                    React.createElement('span', { className: 'badge bg-warning' }, 'Private Key Wallet')
                ),
                !walletState.isHDWallet && React.createElement('p', { className: 'mb-0' },
                  React.createElement('small', { className: 'text-muted' },
                    'HD features (address scanning/generation) not available'
                  )
                )
              )
            )
          )
        ),

        React.createElement('div', { className: 'row' },
          // Left Column - Settings
          React.createElement('div', { className: 'col-lg-6' },
            
            // Update Wallet Label
            React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-tag me-2' }),
                  'Update Wallet Label'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('form', { onSubmit: handleUpdateLabel },
                  React.createElement('div', { className: 'mb-3' },
                    React.createElement('label', { className: 'form-label' }, 'Wallet Label'),
                    React.createElement('input', {
                      type: 'text',
                      className: `form-control ${validationErrors.walletLabel ? 'is-invalid' : ''}`,
                      value: walletLabel,
                      onChange: (e) => setWalletLabel(e.target.value),
                      placeholder: 'Enter wallet label...'
                    }),
                    validationErrors.walletLabel && React.createElement('div', {
                      className: 'invalid-feedback'
                    }, validationErrors.walletLabel)
                  ),
                  React.createElement('button', {
                    type: 'submit',
                    className: `btn btn-primary ${isUpdatingLabel ? 'disabled' : ''}`,
                    disabled: isUpdatingLabel
                  },
                    isUpdatingLabel ? React.createElement('span', null,
                      React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                      'Updating...'
                    ) : React.createElement('span', null,
                      React.createElement('i', { className: 'bi bi-check me-2' }),
                      'Update Label'
                    )
                  )
                )
              )
            ),

            // Change Password
            React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-shield-lock me-2' }),
                  'Change Password'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('form', { onSubmit: handleChangePassword },
                  React.createElement('div', { className: 'mb-3' },
                    React.createElement('label', { className: 'form-label' }, 'Current Password'),
                    React.createElement('input', {
                      type: 'password',
                      className: `form-control ${validationErrors.currentPassword ? 'is-invalid' : ''}`,
                      value: currentPassword,
                      onChange: (e) => setCurrentPassword(e.target.value),
                      placeholder: 'Enter current password...'
                    }),
                    validationErrors.currentPassword && React.createElement('div', {
                      className: 'invalid-feedback'
                    }, validationErrors.currentPassword)
                  ),
                  React.createElement('div', { className: 'mb-3' },
                    React.createElement('label', { className: 'form-label' }, 'New Password'),
                    React.createElement('input', {
                      type: 'password',
                      className: `form-control ${validationErrors.newPassword ? 'is-invalid' : ''}`,
                      value: newPassword,
                      onChange: (e) => setNewPassword(e.target.value),
                      placeholder: 'Enter new password...'
                    }),
                    validationErrors.newPassword && React.createElement('div', {
                      className: 'invalid-feedback'
                    }, validationErrors.newPassword),
                    newPassword && React.createElement('div', { className: 'mt-2' },
                      React.createElement('div', { className: 'progress', style: { height: '4px' } },
                        React.createElement('div', {
                          className: `progress-bar bg-${passwordStrength.color}`,
                          style: { width: `${(passwordStrength.strength === 'weak' ? 25 : passwordStrength.strength === 'fair' ? 50 : passwordStrength.strength === 'good' ? 75 : 100)}%` }
                        })
                      ),
                      React.createElement('small', { className: `text-${passwordStrength.color}` },
                        `Password strength: ${passwordStrength.strength}`
                      )
                    )
                  ),
                  React.createElement('div', { className: 'mb-3' },
                    React.createElement('label', { className: 'form-label' }, 'Confirm New Password'),
                    React.createElement('input', {
                      type: 'password',
                      className: `form-control ${validationErrors.confirmNewPassword ? 'is-invalid' : ''}`,
                      value: confirmNewPassword,
                      onChange: (e) => setConfirmNewPassword(e.target.value),
                      placeholder: 'Confirm new password...'
                    }),
                    validationErrors.confirmNewPassword && React.createElement('div', {
                      className: 'invalid-feedback'
                    }, validationErrors.confirmNewPassword)
                  ),
                  React.createElement('button', {
                    type: 'submit',
                    className: `btn btn-warning ${isChangingPassword ? 'disabled' : ''}`,
                    disabled: isChangingPassword
                  },
                    isChangingPassword ? React.createElement('span', null,
                      React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                      'Changing...'
                    ) : React.createElement('span', null,
                      React.createElement('i', { className: 'bi bi-shield-check me-2' }),
                      'Change Password'
                    )
                  )
                )
              )
            ),

            // Session Settings
            React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-clock me-2' }),
                  'Session Settings'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('form', { onSubmit: handleUpdateSessionSettings },
                  React.createElement('div', { className: 'mb-3' },
                    React.createElement('label', { className: 'form-label' }, 'Session Timeout (minutes)'),
                    React.createElement('input', {
                      type: 'number',
                      className: 'form-control',
                      value: sessionSettings.timeoutMinutes,
                      onChange: (e) => setSessionSettings(prev => ({
                        ...prev,
                        timeoutMinutes: parseInt(e.target.value) || 0
                      })),
                      min: '0',
                      max: '1440',
                      placeholder: '0 = No timeout (current behavior)'
                    }),
                    React.createElement('div', { className: 'form-text' },
                      sessionSettings.timeoutMinutes === 0 ? 
                        'You will be logged out when you refresh the page (current behavior)' :
                        `You will stay logged in for ${sessionSettings.timeoutMinutes} minutes after page refresh`
                    )
                  ),
                  React.createElement('div', { className: 'mb-3 form-check' },
                    React.createElement('input', {
                      type: 'checkbox',
                      className: 'form-check-input',
                      id: 'autoSave',
                      checked: sessionSettings.autoSave,
                      onChange: (e) => setSessionSettings(prev => ({
                        ...prev,
                        autoSave: e.target.checked
                      }))
                    }),
                    React.createElement('label', { className: 'form-check-label', htmlFor: 'autoSave' },
                      'Enable automatic session saving'
                    ),
                    React.createElement('div', { className: 'form-text' },
                      'When enabled, your login session will be saved for the specified timeout period'
                    )
                  ),
                  React.createElement('button', {
                    type: 'submit',
                    className: `btn btn-info ${isUpdatingSession ? 'disabled' : ''}`,
                    disabled: isUpdatingSession
                  },
                    isUpdatingSession ? React.createElement('span', null,
                      React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                      'Updating...'
                    ) : React.createElement('span', null,
                      React.createElement('i', { className: 'bi bi-check me-2' }),
                      'Update Session Settings'
                    )
                  )
                )
              )
            ),

            // Show Mnemonic (only for HD wallets)
            walletState.isHDWallet && React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-eye me-2' }),
                  'Show Recovery Phrase'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                !showMnemonic ? 
                  React.createElement('form', { onSubmit: handleShowMnemonic },
                    React.createElement('div', { className: 'alert alert-warning mb-3' },
                      React.createElement('i', { className: 'bi bi-exclamation-triangle me-2' }),
                      React.createElement('strong', null, 'Security Warning: '),
                      'Never share your recovery phrase with anyone. Anyone with access to your recovery phrase can control your wallet.'
                    ),
                    React.createElement('div', { className: 'mb-3' },
                      React.createElement('label', { className: 'form-label' }, 'Wallet Password'),
                      React.createElement('input', {
                        type: 'password',
                        className: `form-control ${validationErrors.mnemonicPassword ? 'is-invalid' : ''}`,
                        value: mnemonicPassword,
                        onChange: (e) => setMnemonicPassword(e.target.value),
                        placeholder: 'Enter wallet password to reveal mnemonic...'
                      }),
                      validationErrors.mnemonicPassword && React.createElement('div', {
                        className: 'invalid-feedback'
                      }, validationErrors.mnemonicPassword)
                    ),
                    React.createElement('button', {
                      type: 'submit',
                      className: `btn btn-warning ${isRevealingMnemonic ? 'disabled' : ''}`,
                      disabled: isRevealingMnemonic
                    },
                      isRevealingMnemonic ? React.createElement('span', null,
                        React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                        'Revealing...'
                      ) : React.createElement('span', null,
                        React.createElement('i', { className: 'bi bi-eye me-2' }),
                        'Show Recovery Phrase'
                      )
                    )
                  ) :
                  React.createElement('div', null,
                    React.createElement('div', { className: 'alert alert-success mb-3' },
                      React.createElement('i', { className: 'bi bi-shield-check me-2' }),
                      React.createElement('strong', null, 'Recovery Phrase Revealed')
                    ),
                    React.createElement('div', { className: 'card bg-dark text-light mb-3' },
                      React.createElement('div', { className: 'card-body' },
                        React.createElement('div', { className: 'row' },
                          ...revealedMnemonic.split(' ').map((word, index) =>
                            React.createElement('div', { key: index, className: 'col-md-3 col-sm-4 col-6 mb-2' },
                              React.createElement('div', { className: 'p-2 bg-secondary rounded text-center' },
                                React.createElement('small', { className: 'text-muted' }, index + 1),
                                React.createElement('div', { className: 'fw-bold' }, word)
                              )
                            )
                          )
                        )
                      )
                    ),
                    React.createElement('div', { className: 'd-grid gap-2 d-md-flex' },
                      React.createElement('button', {
                        className: 'btn btn-outline-primary',
                        onClick: () => copyToClipboard(revealedMnemonic, 'Recovery phrase')
                      },
                        React.createElement('i', { className: 'bi bi-copy me-2' }),
                        'Copy to Clipboard'
                      ),
                      React.createElement('button', {
                        className: 'btn btn-secondary',
                        onClick: hideMnemonic
                      },
                        React.createElement('i', { className: 'bi bi-eye-slash me-2' }),
                        'Hide Recovery Phrase'
                      )
                    )
                  )
              )
            ),

            // Info for private key wallets
            !walletState.isHDWallet && React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-info-circle me-2' }),
                  'Private Key Wallet'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('div', { className: 'alert alert-info' },
                  React.createElement('h6', { className: 'alert-heading' },
                    React.createElement('i', { className: 'bi bi-key me-2' }),
                    'Limited Functionality'
                  ),
                  React.createElement('p', { className: 'mb-0' },
                    'This wallet was created from a private key and does not support HD (Hierarchical Deterministic) features like address scanning and generation. To access these features, create a new wallet from a mnemonic phrase.'
                  )
                )
              )
            )
          ),

          // Right Column - Address Management
          React.createElement('div', { className: 'col-lg-6' },
            
            // Scan for Addresses
            React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-search me-2' }),
                  'Scan for Addresses'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('div', { className: 'row mb-3' },
                  React.createElement('div', { className: 'col-md-6' },
                    React.createElement('label', { className: 'form-label' }, 'Address Type'),
                    React.createElement('select', {
                      className: 'form-select',
                      value: addressType,
                      onChange: (e) => setAddressType(e.target.value)
                    },
                      React.createElement('option', { value: 'receive' }, 'Receive Addresses'),
                      React.createElement('option', { value: 'change' }, 'Change Addresses')
                    )
                  ),
                  React.createElement('div', { className: 'col-md-3' },
                    React.createElement('label', { className: 'form-label' }, 'Start Index'),
                    React.createElement('input', {
                      type: 'number',
                      className: 'form-control',
                      value: scanRange.start,
                      onChange: (e) => setScanRange(prev => ({ ...prev, start: parseInt(e.target.value) || 0 })),
                      min: 0
                    })
                  ),
                  React.createElement('div', { className: 'col-md-3' },
                    React.createElement('label', { className: 'form-label' }, 'End Index'),
                    React.createElement('input', {
                      type: 'number',
                      className: 'form-control',
                      value: scanRange.end,
                      onChange: (e) => setScanRange(prev => ({ ...prev, end: parseInt(e.target.value) || 10 })),
                      min: scanRange.start
                    })
                  )
                ),
                React.createElement('button', {
                  type: 'button',
                  className: `btn btn-info ${isScanningAddresses || !walletState.isHDWallet ? 'disabled' : ''}`,
                  onClick: handleScanAddresses,
                  disabled: isScanningAddresses || !walletState.isHDWallet
                },
                  isScanningAddresses ? React.createElement('span', null,
                    React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                    'Scanning...'
                  ) : React.createElement('span', null,
                    React.createElement('i', { className: 'bi bi-search me-2' }),
                    'Scan Addresses'
                  )
                )
              )
            ),

            // Generate New Address
            React.createElement('div', { className: 'card mb-4' },
              React.createElement('div', { className: 'card-header' },
                React.createElement('h6', { className: 'card-title mb-0' },
                  React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
                  'Generate New Address'
                )
              ),
              React.createElement('div', { className: 'card-body' },
                React.createElement('p', { className: 'text-muted mb-3' },
                  'Generate a new receive address for this HD wallet.'
                ),
                React.createElement('button', {
                  type: 'button',
                  className: `btn btn-success ${isGeneratingAddress || !walletState.isHDWallet ? 'disabled' : ''}`,
                  onClick: handleGenerateNewAddress,
                  disabled: isGeneratingAddress || !walletState.isHDWallet
                },
                  isGeneratingAddress ? React.createElement('span', null,
                    React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                    'Generating...'
                  ) : React.createElement('span', null,
                    React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
                    'Generate New Address'
                  )
                )
              )
            )
          )
        ),

        // Scanned Addresses Results
        scannedAddresses.length > 0 && React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-list me-2' }),
              `Scanned ${addressType.charAt(0).toUpperCase() + addressType.slice(1)} Addresses (${scannedAddresses.length})`
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'table-responsive' },
              React.createElement('table', { className: 'table table-sm' },
                React.createElement('thead', null,
                  React.createElement('tr', null,
                    React.createElement('th', null, 'Index'),
                    React.createElement('th', null, 'Address'),
                    React.createElement('th', null, 'Actions')
                  )
                ),
                React.createElement('tbody', null,
                  scannedAddresses.map(addr =>
                    React.createElement('tr', { key: addr.index },
                      React.createElement('td', null, addr.index),
                      React.createElement('td', null,
                        React.createElement('code', { 
                          className: 'text-primary',
                          style: { fontSize: '0.8em' }
                        }, 
                          addr.address.substring(0, 30) + '...'
                        )
                      ),
                      React.createElement('td', null,
                        React.createElement('button', {
                          className: 'btn btn-outline-primary btn-sm',
                          onClick: () => copyToClipboard(addr.address, 'Address')
                        },
                          React.createElement('i', { className: 'bi bi-copy' })
                        )
                      )
                    )
                  )
                )
              )
            )
          )
        ),

        // Generated Addresses
        generatedAddresses.length > 0 && React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-stars me-2' }),
              `Generated Addresses (${generatedAddresses.length})`
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'table-responsive' },
              React.createElement('table', { className: 'table table-sm' },
                React.createElement('thead', null,
                  React.createElement('tr', null,
                    React.createElement('th', null, 'Index'),
                    React.createElement('th', null, 'Address'),
                    React.createElement('th', null, 'Generated'),
                    React.createElement('th', null, 'Actions')
                  )
                ),
                React.createElement('tbody', null,
                  generatedAddresses.map((addr, idx) =>
                    React.createElement('tr', { key: idx },
                      React.createElement('td', null, addr.index),
                      React.createElement('td', null,
                        React.createElement('code', { 
                          className: 'text-success',
                          style: { fontSize: '0.8em' }
                        }, 
                          addr.address.substring(0, 30) + '...'
                        )
                      ),
                      React.createElement('td', null,
                        React.createElement('small', { className: 'text-muted' },
                          new Date(addr.generated).toLocaleTimeString()
                        )
                      ),
                      React.createElement('td', null,
            React.createElement('button', {
                          className: 'btn btn-outline-success btn-sm',
                          onClick: () => copyToClipboard(addr.address, 'Generated Address')
                        },
                          React.createElement('i', { className: 'bi bi-copy' })
                        )
                      )
                    )
                  )
                )
              )
            )
          )
        )
      )
    )
  );
} 
