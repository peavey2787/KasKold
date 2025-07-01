const { useState, useEffect } = React;

export function WalletCreation({ onNavigate, onWalletCreated, addNotification, network = 'mainnet' }) {
  const [currentStep, setCurrentStep] = useState(1);
  const [walletData, setWalletData] = useState(null);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [walletLabel, setWalletLabel] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [mnemonicRevealed, setMnemonicRevealed] = useState(false);
  const [mnemonicConfirmed, setMnemonicConfirmed] = useState(false);
  const [securityAcknowledged, setSecurityAcknowledged] = useState(false);
  const [selectedNetwork, setSelectedNetwork] = useState(network);



  // Sync selectedNetwork with prop when it changes
  useEffect(() => {
    setSelectedNetwork(network);
  }, [network]);

  const generateNewWallet = async () => {
    setIsGenerating(true);
    
    try {
      const { generateWallet } = await import('../../kaspa/js/wallet-generator.js');
      
      const generatedWallet = generateWallet(selectedNetwork);
      
      setWalletData({
        ...generatedWallet,
        address: generatedWallet.publicAddress,
        network: generatedWallet.networkType,
        xpub: generatedWallet.xpub,
        accountPath: generatedWallet.accountPath
      });
      
      setCurrentStep(2);
      addNotification('New wallet generated successfully', 'success');
      
    } catch (error) {
      console.error('Wallet generation failed:', error);
      addNotification('Failed to generate wallet: ' + error.message, 'error');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleSaveWallet = async (e) => {
    e.preventDefault();
    
    if (!validateForm()) {
      return;
    }

    setIsSaving(true);
    
    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      const walletToSave = {
        privateKey: walletData.privateKey,
        address: walletData.address,
        network: walletData.network,
        mnemonic: walletData.mnemonic,
        derivationPath: walletData.derivationPath,
        xpub: walletData.xpub,
        accountPath: walletData.accountPath
      };
      
      await walletStorage.saveWallet(walletToSave, password);
      
      // Update the wallet label if provided
      if (walletLabel.trim()) {
        const walletId = walletStorage.generateWalletId(walletData.address, walletData.network);
        await walletStorage.updateWalletLabel(walletId, walletLabel.trim());
      }
      
      // Create the wallet object for login
      const createdWallet = {
        address: walletData.address,
        network: walletData.network,
        privateKey: walletData.privateKey,
        mnemonic: walletData.mnemonic,
        derivationPath: walletData.derivationPath,
        xpub: walletData.xpub,
        accountPath: walletData.accountPath,
        label: walletLabel.trim() || `Wallet ${walletData.address.substring(0, 8)}...`
      };
      
      addNotification('Wallet created and saved successfully', 'success');
      onWalletCreated(createdWallet);
      
    } catch (error) {
      console.error('Wallet saving failed:', error);
      addNotification('Failed to save wallet: ' + error.message, 'error');
    } finally {
      setIsSaving(false);
    }
  };

  const isFormValid = () => {
    return password && 
           password.length >= 8 && 
           password === confirmPassword && 
           mnemonicConfirmed && 
           securityAcknowledged;
  };

  const validateForm = () => {
    if (!password || password.length < 8) {
      addNotification('Password must be at least 8 characters long', 'warning');
      return false;
    }
    
    if (password !== confirmPassword) {
      addNotification('Passwords do not match', 'warning');
      return false;
    }
    
    if (!mnemonicConfirmed) {
      addNotification('Please confirm you have backed up your mnemonic phrase', 'warning');
      return false;
    }
    
    if (!securityAcknowledged) {
      addNotification('Please acknowledge the security warnings', 'warning');
      return false;
    }
    
    return true;
  };

  const getPasswordStrength = (pwd) => {
    let strength = 0;
    if (pwd.length >= 8) strength++;
    if (pwd.length >= 12) strength++;
    if (/[a-z]/.test(pwd)) strength++;
    if (/[A-Z]/.test(pwd)) strength++;
    if (/[0-9]/.test(pwd)) strength++;
    if (/[^A-Za-z0-9]/.test(pwd)) strength++;
    
    if (strength <= 2) return { level: 'weak', text: 'Weak', color: 'danger' };
    if (strength <= 4) return { level: 'fair', text: 'Fair', color: 'warning' };
    if (strength <= 5) return { level: 'good', text: 'Good', color: 'info' };
    return { level: 'strong', text: 'Strong', color: 'success' };
  };

  const passwordStrength = getPasswordStrength(password);

  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-8' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
              'Create New Wallet'
            ),
            React.createElement('div', { className: 'progress mt-2', style: { height: '4px' } },
              React.createElement('div', { 
                className: 'progress-bar',
                style: { width: `${(currentStep / 2) * 100}%` }
              })
            )
          ),
          React.createElement('div', { className: 'card-body' },
            
            // Step 1: Security Warning and Generate
            currentStep === 1 && React.createElement('div', null,
              React.createElement('div', { className: 'text-center mb-4' },
                React.createElement('i', { className: 'bi bi-shield-exclamation display-4 text-warning mb-3' }),
                React.createElement('h4', null, 'Security Information'),
                React.createElement('p', { className: 'text-muted' }, 
                  'Before creating your wallet, please read and understand these important security considerations.'
                )
              ),

              // Network Selection (at the top, before security warnings)
              React.createElement('div', { className: 'mb-4' },
                React.createElement('label', { className: 'form-label fw-bold' }, 'Network'),
                React.createElement('select', {
                  className: 'form-select',
                  value: selectedNetwork,
                  onChange: (e) => setSelectedNetwork(e.target.value)
                },
                  React.createElement('option', { value: 'mainnet' }, 'Mainnet'),
                  React.createElement('option', { value: 'testnet-10' }, 'Testnet-10'),
                  React.createElement('option', { value: 'testnet-11' }, 'Testnet-11'),
                  React.createElement('option', { value: 'devnet' }, 'Devnet'),
                  React.createElement('option', { value: 'simnet' }, 'Simnet')
                ),
                React.createElement('div', { className: 'form-text' },
                  'Select the network for your new wallet. This cannot be changed after creation.'
                )
              ),
              
              React.createElement('div', { className: 'alert alert-warning' },
                React.createElement('h6', { className: 'alert-heading' },
                  React.createElement('i', { className: 'bi bi-exclamation-triangle me-2' }),
                  'Important Security Warnings'
                ),
                React.createElement('ul', { className: 'mb-0' },
                  React.createElement('li', null, 'Your mnemonic phrase is the ONLY way to recover your wallet'),
                  React.createElement('li', null, 'Write down your mnemonic phrase and store it safely offline'),
                  React.createElement('li', null, 'Never share your mnemonic phrase or private key with anyone'),
                  React.createElement('li', null, 'Lost mnemonic phrases cannot be recovered - your funds will be lost forever'),
                  React.createElement('li', null, 'Use a strong password to encrypt your wallet locally')
                )
              ),
              
              React.createElement('div', { className: 'form-check mb-4' },
                React.createElement('input', {
                  className: 'form-check-input',
                  type: 'checkbox',
                  id: 'securityAcknowledged',
                  checked: securityAcknowledged,
                  onChange: (e) => setSecurityAcknowledged(e.target.checked)
                }),
                React.createElement('label', { 
                  className: 'form-check-label',
                  htmlFor: 'securityAcknowledged',
                  style: { cursor: 'pointer' }
                },
                  'I understand and acknowledge the security warnings above'
                )
              ),
              
              React.createElement('div', { className: 'd-grid gap-2' },
                React.createElement('button', {
                  className: `btn btn-primary btn-lg ${!securityAcknowledged || isGenerating ? 'disabled' : ''}`,
                  onClick: generateNewWallet,
                  disabled: !securityAcknowledged || isGenerating
                },
                  isGenerating ? 
                    React.createElement('span', null,
                      React.createElement('span', { 
                        className: 'spinner-border spinner-border-sm me-2' 
                      }),
                      'Generating Wallet...'
                    ) :
                    React.createElement('span', null,
                      React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
                      'Generate New Wallet'
                    )
                ),
                React.createElement('button', {
                  className: 'btn btn-outline-secondary',
                  onClick: () => onNavigate('welcome')
                }, 'Back to Welcome')
              )
            ),
            
            // Step 2: Mnemonic & Password Setup
            currentStep === 2 && walletData && React.createElement('div', null,
              
              // Mnemonic Section
              React.createElement('div', { className: 'mb-4' },
                React.createElement('h6', { className: 'mb-3' },
                  React.createElement('i', { className: 'bi bi-key me-2' }),
                  'Your Recovery Phrase (Mnemonic)'
                ),
                React.createElement('div', { className: 'alert alert-info' },
                  React.createElement('p', { className: 'mb-2' },
                    React.createElement('strong', null, 'CRITICAL: '),
                    'Write down these 24 words in order and store them safely. This is the only way to recover your wallet.'
                  )
                ),
                
                !mnemonicRevealed ? 
                  React.createElement('div', { className: 'text-center py-4' },
                    React.createElement('button', {
                      className: 'btn btn-warning',
                      onClick: () => setMnemonicRevealed(true)
                    },
                      React.createElement('i', { className: 'bi bi-eye me-2' }),
                      'Reveal Recovery Phrase'
                    ),
                    React.createElement('p', { className: 'text-muted mt-2 small' },
                      'Make sure no one is looking at your screen'
                    )
                  ) :
                  React.createElement('div', null,
                    React.createElement('div', { 
                      className: 'mnemonic-display p-3 bg-light border rounded',
                      style: { fontFamily: 'monospace', fontSize: '16px', lineHeight: '1.6' }
                    }, walletData.mnemonic),
                    
                    React.createElement('div', { className: 'form-check mt-3' },
                      React.createElement('input', {
                        className: 'form-check-input',
                        type: 'checkbox',
                        id: 'mnemonicConfirmed',
                        checked: mnemonicConfirmed,
                        onChange: (e) => setMnemonicConfirmed(e.target.checked)
                      }),
                      React.createElement('label', { 
                        className: 'form-check-label',
                        htmlFor: 'mnemonicConfirmed',
                        style: { cursor: 'pointer' }
                      },
                        'I have safely written down my recovery phrase'
                      )
                    )
                  )
              ),
              
              // Password Section
              mnemonicRevealed && React.createElement('form', { onSubmit: handleSaveWallet },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Wallet Label (Optional)'),
                  React.createElement('input', {
                    type: 'text',
                    className: 'form-control',
                    value: walletLabel,
                    onChange: (e) => setWalletLabel(e.target.value),
                    placeholder: 'My Kaspa Wallet'
                  })
                ),
                
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Wallet Password'),
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('input', {
                      type: showPassword ? 'text' : 'password',
                      className: 'form-control',
                      value: password,
                      onChange: (e) => setPassword(e.target.value),
                      placeholder: 'Enter a strong password',
                      required: true
                    }),
                    React.createElement('button', {
                      className: 'btn btn-outline-secondary',
                      type: 'button',
                      onClick: () => setShowPassword(!showPassword)
                    },
                      React.createElement('i', { 
                        className: showPassword ? 'bi bi-eye-slash' : 'bi bi-eye' 
                      })
                    )
                  ),
                  password && React.createElement('div', { className: 'mt-2' },
                    React.createElement('div', { className: 'progress', style: { height: '4px' } },
                      React.createElement('div', { 
                        className: `progress-bar bg-${passwordStrength.color}`,
                        style: { width: `${(Object.keys({weak:1,fair:2,good:3,strong:4}).indexOf(passwordStrength.level) + 1) * 25}%` }
                      })
                    ),
                    React.createElement('small', { className: `text-${passwordStrength.color}` },
                      `Password strength: ${passwordStrength.text}`
                    )
                  )
                ),
                
                React.createElement('div', { className: 'mb-4' },
                  React.createElement('label', { className: 'form-label' }, 'Confirm Password'),
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('input', {
                      type: showConfirmPassword ? 'text' : 'password',
                      className: 'form-control',
                      value: confirmPassword,
                      onChange: (e) => setConfirmPassword(e.target.value),
                      placeholder: 'Confirm your password',
                      required: true
                    }),
                    React.createElement('button', {
                      className: 'btn btn-outline-secondary',
                      type: 'button',
                      onClick: () => setShowConfirmPassword(!showConfirmPassword)
                    },
                      React.createElement('i', { 
                        className: showConfirmPassword ? 'bi bi-eye-slash' : 'bi bi-eye' 
                      })
                    )
                  )
                ),
                
                React.createElement('div', { className: 'd-grid gap-2' },
                  React.createElement('button', {
                    type: 'submit',
                    className: `btn btn-success btn-lg ${isSaving || !isFormValid() ? 'disabled' : ''}`,
                    disabled: isSaving || !isFormValid()
                  },
                    isSaving ? 
                      React.createElement('span', null,
                        React.createElement('span', { 
                          className: 'spinner-border spinner-border-sm me-2' 
                        }),
                        'Creating Wallet...'
                      ) :
                      React.createElement('span', null,
                        React.createElement('i', { className: 'bi bi-check-circle me-2' }),
                        'Create & Save Wallet'
                      )
                  ),
                  React.createElement('button', {
                    type: 'button',
                    className: 'btn btn-outline-secondary',
                    onClick: () => setCurrentStep(1)
                  }, 'Back')
                )
              )
            ),
            
            // Wallet Info Display (for debugging)
            currentStep === 2 && walletData && React.createElement('div', { className: 'mt-4' },
              React.createElement('div', { className: 'card bg-light' },
                React.createElement('div', { className: 'card-header' },
                  React.createElement('h6', { className: 'mb-0' }, 'Wallet Information')
                ),
                React.createElement('div', { className: 'card-body' },
                  React.createElement('div', { className: 'row' },
                    React.createElement('div', { className: 'col-12 mb-2' },
                      React.createElement('strong', null, 'Address:'),
                      React.createElement('br'),
                      React.createElement('code', { className: 'text-break small' }, walletData.address)
                    ),
                    React.createElement('div', { className: 'col-6' },
                      React.createElement('strong', null, 'Network:'),
                      React.createElement('br'),
                      React.createElement('span', { 
                        className: `badge ${walletData.network === 'mainnet' ? 'bg-primary' : 'bg-warning text-dark'}`
                      }, walletData.network.toUpperCase())
                    ),
                    React.createElement('div', { className: 'col-6' },
                      React.createElement('strong', null, 'Derivation Path:'),
                      React.createElement('br'),
                      React.createElement('code', { className: 'small' }, walletData.derivationPath)
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