const { useState, useEffect } = React;

export function WalletRestore({ onNavigate, onWalletRestored, addNotification, network: propNetwork = 'mainnet' }) {
  const [restoreMethod, setRestoreMethod] = useState('mnemonic'); // 'mnemonic' or 'privateKey'
  const [mnemonicPhrase, setMnemonicPhrase] = useState('');
  const [privateKey, setPrivateKey] = useState('');
  const [passphrase, setPassphrase] = useState(''); // Optional BIP39 passphrase
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [network, setNetwork] = useState(propNetwork);
  const [derivationPath, setDerivationPath] = useState("m/44'/111111'/0'/0/0");
  const [walletLabel, setWalletLabel] = useState('');
  const [isRestoring, setIsRestoring] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [validationErrors, setValidationErrors] = useState({});
  const [restoredWalletData, setRestoredWalletData] = useState(null);
  const [isSaving, setIsSaving] = useState(false);

  // Sync network state with prop
  useEffect(() => {
    setNetwork(propNetwork);
  }, [propNetwork]);

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

  const passwordStrength = calculatePasswordStrength(password);

  // Validate form
  const validateForm = () => {
    const errors = {};

    // Validate restore method specific fields
    if (restoreMethod === 'mnemonic') {
      if (!mnemonicPhrase.trim()) {
        errors.mnemonicPhrase = 'Mnemonic phrase is required';
      } else {
        const words = mnemonicPhrase.trim().split(/\s+/);
        const validWordCounts = [12, 15, 18, 21, 24];
        if (!validWordCounts.includes(words.length)) {
          errors.mnemonicPhrase = `Invalid word count. Expected 12, 15, 18, 21, or 24 words, got ${words.length}.`;
        }
      }
    } else {
      if (!privateKey.trim()) {
        errors.privateKey = 'Private key is required';
      }
    }

    // Validate password
    if (!password) {
      errors.password = 'Password is required';
    } else if (password.length < 8) {
      errors.password = 'Password must be at least 8 characters long';
    }

    if (password !== confirmPassword) {
      errors.confirmPassword = 'Passwords do not match';
    }

    if (!walletLabel.trim()) {
      errors.walletLabel = 'Wallet label is required';
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  // Handle wallet restoration (first step - just restore, don't save)
  const handleRestoreWallet = async () => {
    // Basic validation for restoration inputs
    const errors = {};
    if (restoreMethod === 'mnemonic') {
      if (!mnemonicPhrase.trim()) {
        errors.mnemonicPhrase = 'Mnemonic phrase is required';
      }
    } else {
      if (!privateKey.trim()) {
        errors.privateKey = 'Private key is required';
      }
    }

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      addNotification('Please fix the validation errors', 'error');
      return;
    }

    setIsRestoring(true);

    try {
      let restorationResult;

      if (restoreMethod === 'mnemonic') {
        // Import restoration functions
        const { restoreWalletFromMnemonic, validateMnemonic } = await import('../../kaspa/js/wallet-restore.js');
        
        // Validate mnemonic first
        const mnemonicValidation = validateMnemonic(mnemonicPhrase);
        if (!mnemonicValidation.isValid) {
          throw new Error(mnemonicValidation.error);
        }

        // Restore from mnemonic
        restorationResult = restoreWalletFromMnemonic(
          mnemonicPhrase,
          network,
          derivationPath,
          passphrase || null
        );
      } else {
        // Import restoration functions
        const { restoreWalletFromPrivateKey, validatePrivateKey } = await import('../../kaspa/js/wallet-restore.js');
        
        // Validate private key first
        const privateKeyValidation = validatePrivateKey(privateKey);
        if (!privateKeyValidation.isValid) {
          throw new Error(privateKeyValidation.error);
        }

        // Restore from private key
        restorationResult = restoreWalletFromPrivateKey(privateKey, network);
      }

      if (!restorationResult.success) {
        throw new Error(restorationResult.error);
      }

      // Store restoration result for saving later
      setRestoredWalletData({
        privateKey: restorationResult.privateKey,
        address: restorationResult.publicAddress,
        network: restorationResult.networkType,
        mnemonic: restorationResult.mnemonic,
        derivationPath: restorationResult.mnemonic ? "m/44'/111111'/0'" : restorationResult.derivationPath, // ✅ Use account-level path for mnemonic, keep original for private key
        restoredFrom: restoreMethod
      });

      addNotification(`Wallet restored successfully! Address: ${restorationResult.publicAddress.substring(0, 20)}...`, 'success');

    } catch (error) {
      console.error('Wallet restoration failed:', error);
      addNotification('Failed to restore wallet: ' + error.message, 'error');
    } finally {
      setIsRestoring(false);
    }
  };

  // Handle wallet saving (second step)
  const handleSaveWallet = async (e) => {
    e.preventDefault();

    if (!restoredWalletData) {
      addNotification('No wallet data to save. Please restore a wallet first.', 'error');
      return;
    }

    if (!validateForm()) {
      addNotification('Please fix the validation errors', 'error');
      return;
    }

    setIsSaving(true);

    try {
      // Save the restored wallet
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();

      const walletId = await walletStorage.saveWallet(restoredWalletData, password);

      // Update wallet label if provided
      if (walletLabel.trim()) {
        await walletStorage.updateWalletLabel(walletId, walletLabel.trim());
      }

      // Prepare wallet object for login
      const savedWallet = {
        id: walletId,
        address: restoredWalletData.address,
        network: restoredWalletData.network,
        privateKey: restoredWalletData.privateKey,
        mnemonic: restoredWalletData.mnemonic,
        derivationPath: restoredWalletData.derivationPath,
        name: walletLabel.trim(),
        createdAt: new Date()
      };

      addNotification(`Wallet saved successfully`, 'success');
      
      // Auto-login to the saved wallet
      onWalletRestored(savedWallet);

    } catch (error) {
      console.error('Wallet saving failed:', error);
      addNotification('Failed to save wallet: ' + error.message, 'error');
    } finally {
      setIsSaving(false);
    }
  };

  // Handle skip saving with confirmation
  const handleSkipSaving = () => {
    if (window.confirm('Are you sure you want to skip saving? You will need to enter your mnemonic/private key again next time.')) {
      // Prepare temporary wallet object for login
      const tempWallet = {
        id: 'temp_' + Date.now(),
        address: restoredWalletData.address,
        network: restoredWalletData.network,
        privateKey: restoredWalletData.privateKey,
        mnemonic: restoredWalletData.mnemonic,
        derivationPath: restoredWalletData.derivationPath,
        name: 'Temporary Wallet',
        createdAt: new Date(),
        isTemporary: true
      };

      addNotification('Using wallet temporarily - not saved to storage', 'warning');
      onWalletRestored(tempWallet);
    }
  };

  // Handle mnemonic word validation on blur
  const handleMnemonicBlur = async () => {
    if (!mnemonicPhrase.trim()) return;

    try {
      const { validateMnemonic } = await import('../../kaspa/js/wallet-restore.js');
      const validation = validateMnemonic(mnemonicPhrase);
      
      if (!validation.isValid) {
        setValidationErrors(prev => ({
          ...prev,
          mnemonicPhrase: validation.error
        }));
      } else {
        setValidationErrors(prev => {
          const newErrors = { ...prev };
          delete newErrors.mnemonicPhrase;
          return newErrors;
        });
      }
    } catch (error) {
      console.error('Mnemonic validation error:', error);
    }
  };

  // Handle private key validation on blur
  const handlePrivateKeyBlur = async () => {
    if (!privateKey.trim()) return;

    try {
      const { validatePrivateKey } = await import('../../kaspa/js/wallet-restore.js');
      const validation = validatePrivateKey(privateKey);
      
      if (!validation.isValid) {
        setValidationErrors(prev => ({
          ...prev,
          privateKey: validation.error
        }));
      } else {
        setValidationErrors(prev => {
          const newErrors = { ...prev };
          delete newErrors.privateKey;
          return newErrors;
        });
      }
    } catch (error) {
      console.error('Private key validation error:', error);
    }
  };

  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-8' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-arrow-clockwise me-2' }),
              'Restore Wallet'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('form', { onSubmit: handleSaveWallet },
              
              // Network Selection (moved to top)
              React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Network'),
                React.createElement('select', {
                  className: 'form-select',
                  value: network,
                  onChange: (e) => setNetwork(e.target.value)
                },
                  React.createElement('option', { value: 'mainnet' }, 'Mainnet'),
                  React.createElement('option', { value: 'testnet-10' }, 'Testnet-10'),
                  React.createElement('option', { value: 'testnet-11' }, 'Testnet-11'),
                  React.createElement('option', { value: 'devnet' }, 'Devnet'),
                  React.createElement('option', { value: 'simnet' }, 'Simnet')
                )
              ),

              // Restoration Method Selection
            React.createElement('div', { className: 'mb-4' },
                React.createElement('label', { className: 'form-label' }, 'Restoration Method'),
                React.createElement('div', { className: 'btn-group w-100', role: 'group' },
                  React.createElement('input', {
                    type: 'radio',
                    className: 'btn-check',
                    name: 'restoreMethod',
                    id: 'mnemonic',
                    checked: restoreMethod === 'mnemonic',
                    onChange: () => setRestoreMethod('mnemonic')
                  }),
                  React.createElement('label', {
                    className: 'btn btn-outline-primary',
                    htmlFor: 'mnemonic'
                  },
                    React.createElement('i', { className: 'bi bi-list-ul me-2' }),
                    'Mnemonic Phrase'
                  ),
                  React.createElement('input', {
                    type: 'radio',
                    className: 'btn-check',
                    name: 'restoreMethod',
                    id: 'privateKey',
                    checked: restoreMethod === 'privateKey',
                    onChange: () => setRestoreMethod('privateKey')
                  }),
                  React.createElement('label', {
                    className: 'btn btn-outline-primary',
                    htmlFor: 'privateKey'
                  },
                    React.createElement('i', { className: 'bi bi-key me-2' }),
                    'Private Key'
                  )
                )
              ),

              // Security Notice (moved above inputs)
              React.createElement('div', { className: 'alert alert-warning mb-4' },
                React.createElement('h6', { className: 'alert-heading' },
                  React.createElement('i', { className: 'bi bi-exclamation-triangle me-2' }),
                  'Security Notice'
                ),
                React.createElement('p', { className: 'mb-0' },
                  'Make sure you are in a secure environment when entering your mnemonic phrase or private key.'
                )
              ),

              // Mnemonic Phrase Input
              restoreMethod === 'mnemonic' && React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Mnemonic Phrase'),
                React.createElement('textarea', {
                  className: `form-control ${validationErrors.mnemonicPhrase ? 'is-invalid' : ''}`,
                  rows: 3,
                  value: mnemonicPhrase,
                  onChange: (e) => setMnemonicPhrase(e.target.value),
                  onBlur: handleMnemonicBlur,
                  placeholder: 'Enter your 24 word mnemonic phrase...',
                  required: restoreMethod === 'mnemonic'
                }),
                validationErrors.mnemonicPhrase && React.createElement('div', {
                  className: 'invalid-feedback'
                }, validationErrors.mnemonicPhrase),
                React.createElement('div', { className: 'form-text' },
                  'Enter the mnemonic phrase (seed words) from your wallet backup'
                )
              ),

              // Private Key Input
              restoreMethod === 'privateKey' && React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Private Key'),
                React.createElement('textarea', {
                  className: `form-control ${validationErrors.privateKey ? 'is-invalid' : ''}`,
                  rows: 2,
                  value: privateKey,
                  onChange: (e) => setPrivateKey(e.target.value),
                  onBlur: handlePrivateKeyBlur,
                  placeholder: 'Enter your private key...',
                  required: restoreMethod === 'privateKey'
                }),
                validationErrors.privateKey && React.createElement('div', {
                  className: 'invalid-feedback'
                }, validationErrors.privateKey),
                React.createElement('div', { className: 'form-text' },
                  'Enter the private key in hexadecimal format'
                )
              ),

              // Restore Wallet Button (after inputs)
              !restoredWalletData && React.createElement('div', { className: 'text-center mb-4' },
                React.createElement('button', {
                  type: 'button',
                  className: `btn btn-primary ${isRestoring ? 'disabled' : ''}`,
                  onClick: handleRestoreWallet,
                  disabled: isRestoring
                },
                  isRestoring ? React.createElement('span', null,
                    React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                    'Restoring Wallet...'
                  ) : React.createElement('span', null,
                    React.createElement('i', { className: 'bi bi-arrow-clockwise me-2' }),
                    'Restore Wallet'
                  )
                )
              ),

              // Wallet Restored Success Message
              restoredWalletData && React.createElement('div', { className: 'alert alert-success mb-4' },
                React.createElement('h6', { className: 'alert-heading' },
                  React.createElement('i', { className: 'bi bi-check-circle me-2' }),
                  'Wallet Restored Successfully!'
                ),
                React.createElement('p', { className: 'mb-2' },
                  React.createElement('strong', null, 'Address: '),
                  React.createElement('code', { className: 'text-success' }, restoredWalletData.address)
                ),
                React.createElement('p', { className: 'mb-2' },
                  React.createElement('strong', null, 'Network: '),
                  restoredWalletData.network
                ),
                restoredWalletData.derivationPath && React.createElement('p', { className: 'mb-0' },
                  React.createElement('strong', null, 'Derivation Path: '),
                  React.createElement('code', null, restoredWalletData.derivationPath)
                )
              ),

              // Info Notice about encryption (only show after restoration)
              restoredWalletData && React.createElement('div', { className: 'alert alert-info mb-4' },
                React.createElement('h6', { className: 'alert-heading' },
                  React.createElement('i', { className: 'bi bi-info-circle me-2' }),
                  'Wallet Encryption'
                ),
                React.createElement('p', { className: 'mb-0' },
                  'Your wallet will be encrypted with the password you provide.'
                )
              ),

              // Advanced Options Toggle (only show before restoration)
              !restoredWalletData && React.createElement('div', { className: 'mb-3' },
                React.createElement('button', {
                  type: 'button',
                  className: 'btn btn-link p-0 text-decoration-none',
                  onClick: () => setShowAdvanced(!showAdvanced)
                },
                  React.createElement('i', { 
                    className: `bi bi-chevron-${showAdvanced ? 'up' : 'down'} me-1` 
                  }),
                  'Advanced Options'
                )
              ),

              // Advanced Options (only show before restoration)
              !restoredWalletData && showAdvanced && React.createElement('div', { className: 'border rounded p-3 mb-3 bg-light' },
                // BIP39 Passphrase (only for mnemonic)
                restoreMethod === 'mnemonic' && React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'BIP39 Passphrase (Optional)'),
                  React.createElement('input', {
                    type: 'password',
                    className: 'form-control',
                    value: passphrase,
                    onChange: (e) => setPassphrase(e.target.value),
                    placeholder: 'Enter BIP39 passphrase if used...'
                  }),
                  React.createElement('div', { className: 'form-text' },
                    'Only enter if you used a passphrase when creating the wallet'
                  )
                ),

                // Derivation Path (only for mnemonic)
                restoreMethod === 'mnemonic' && React.createElement('div', { className: 'mb-0' },
                  React.createElement('label', { className: 'form-label' }, 'Derivation Path'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control',
                    value: derivationPath,
                    onChange: (e) => setDerivationPath(e.target.value),
                    placeholder: "m/44'/111111'/0'/0/0"
                  }),
                  React.createElement('div', { className: 'form-text' },
                    'Standard Kaspa derivation path. Only change if you know what you\'re doing.'
                  )
                )
              ),

              // Wallet Label (only show after restoration)
              restoredWalletData && React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Wallet Label'),
                React.createElement('input', {
                  type: 'text',
                  className: `form-control ${validationErrors.walletLabel ? 'is-invalid' : ''}`,
                  value: walletLabel,
                  onChange: (e) => setWalletLabel(e.target.value),
                  placeholder: 'Enter a name for this wallet...',
                  required: true
                }),
                validationErrors.walletLabel && React.createElement('div', {
                  className: 'invalid-feedback'
                }, validationErrors.walletLabel)
              ),

              // Password Section (only show after restoration)
              restoredWalletData && React.createElement('div', { className: 'row' },
                React.createElement('div', { className: 'col-md-6 mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Wallet Password'),
                  React.createElement('input', {
                    type: 'password',
                    className: `form-control ${validationErrors.password ? 'is-invalid' : ''}`,
                    value: password,
                    onChange: (e) => setPassword(e.target.value),
                    placeholder: 'Enter password to encrypt wallet...',
                    required: true
                  }),
                  validationErrors.password && React.createElement('div', {
                    className: 'invalid-feedback'
                  }, validationErrors.password),
                  password && React.createElement('div', { className: 'mt-2' },
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
                React.createElement('div', { className: 'col-md-6 mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Confirm Password'),
                  React.createElement('input', {
                    type: 'password',
                    className: `form-control ${validationErrors.confirmPassword ? 'is-invalid' : ''}`,
                    value: confirmPassword,
                    onChange: (e) => setConfirmPassword(e.target.value),
                    placeholder: 'Confirm your password...',
                    required: true
                  }),
                  validationErrors.confirmPassword && React.createElement('div', {
                    className: 'invalid-feedback'
                  }, validationErrors.confirmPassword)
                )
              ),

              // Action Buttons
              React.createElement('div', { className: 'd-flex justify-content-between align-items-center' },
            React.createElement('button', {
                  type: 'button',
              className: 'btn btn-outline-secondary',
              onClick: () => onNavigate('welcome')
                },
                  React.createElement('i', { className: 'bi bi-arrow-left me-2' }),
                  'Back'
                ),
                
                // Show different buttons based on restoration state
                !restoredWalletData ? 
                  // Before restoration: just back button (restore button is above inputs)
                  null :
                  // After restoration: save and skip buttons
                  React.createElement('div', { className: 'd-flex gap-2' },
                    React.createElement('button', {
                      type: 'button',
                      className: 'btn btn-outline-warning',
                      onClick: handleSkipSaving
                    },
                      React.createElement('i', { className: 'bi bi-skip-forward me-2' }),
                      'Skip Saving'
                    ),
                    React.createElement('button', {
                      type: 'submit',
                      className: `btn btn-success ${isSaving ? 'disabled' : ''}`,
                      disabled: isSaving
                    },
                      isSaving ? React.createElement('span', null,
                        React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                        'Saving Wallet...'
                      ) : React.createElement('span', null,
                        React.createElement('i', { className: 'bi bi-save me-2' }),
                        'Save Wallet'
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