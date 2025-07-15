const { useState, useEffect } = React;

export function WalletLogin({ onNavigate, onWalletLogin, addNotification }) {
  const [wallets, setWallets] = useState([]);
  const [selectedWallet, setSelectedWallet] = useState('');
  const [password, setPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  useEffect(() => {
    loadWallets();
  }, []);

  const loadWallets = async () => {
    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();    
      const savedWallets = await walletStorage.getAllWallets();

      setWallets(savedWallets || []);
      
      if (savedWallets && savedWallets.length > 0) {
        setSelectedWallet(savedWallets[0].id);
      } 
    } catch (error) {
      console.error('🔍 WALLET LOGIN: Failed to load wallets:', error);
      addNotification('Failed to load wallets', 'error');
    }
  };

  const handleLogin = async (e) => {
    e.preventDefault();
    
    if (!selectedWallet || !password) {
      addNotification('Please select a wallet and enter password', 'warning');
      return;
    }

    setIsLoading(true);
    
    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      
      const wallet = wallets.find(w => w.id === selectedWallet);
      if (!wallet) {
        throw new Error('Wallet not found');
      }

      // Attempt to decrypt and login
      const decryptedWallet = await walletStorage.decryptWallet(selectedWallet, password);

      if (decryptedWallet) {
        onWalletLogin(decryptedWallet);
      } else {
        throw new Error('Invalid password');
      }
      
    } catch (error) {
      console.error('Login failed:', error);
      addNotification('Login failed: ' + error.message, 'error');
    } finally {
      setIsLoading(false);
    }
  };

  const handleBackToWelcome = () => {
    onNavigate('welcome');
  };

  const handleDeleteWallet = async (walletId, walletLabel) => {
    if (!confirm(`Are you sure you want to delete wallet "${walletLabel}"? This action cannot be undone.`)) {
      return;
    }

    try {
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      const walletStorage = new WalletStorage();
      await walletStorage.deleteWallet(walletId);
      await loadWallets();
      addNotification('Wallet deleted successfully', 'success');
      
      if (selectedWallet === walletId) {
        const remainingWallets = wallets.filter(w => w.id !== walletId);
        setSelectedWallet(remainingWallets.length > 0 ? remainingWallets[0].id : '');
      }
    } catch (error) {
      console.error('Failed to delete wallet:', error);
      addNotification('Failed to delete wallet', 'error');
    }
  };



  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-6' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-key me-2' }),
              'Login to Wallet'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            wallets.length === 0 ? 
              React.createElement('div', { className: 'text-center py-4' },
                React.createElement('i', { className: 'bi bi-wallet2 display-4 text-muted mb-3' }),
                React.createElement('h6', { className: 'text-muted' }, 'No wallets found'),
                React.createElement('p', { className: 'text-muted mb-3' }, 'Create a new wallet or restore an existing one to get started.'),
                React.createElement('div', { className: 'd-grid gap-2' },
                  React.createElement('button', {
                    className: 'btn btn-primary',
                    onClick: () => onNavigate('wallet-creation')
                  }, 'Create New Wallet'),
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary',
                    onClick: () => onNavigate('wallet-restore')
                  }, 'Restore Wallet')
                )
              ) :
              React.createElement('form', { onSubmit: handleLogin },
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Select Wallet'),
                  React.createElement('select', {
                    className: 'form-select',
                    value: selectedWallet,
                    onChange: (e) => setSelectedWallet(e.target.value),
                    required: true
                  },
                    React.createElement('option', { value: '' }, 'Choose a wallet...'),
                    ...wallets.map(wallet => {
                      const displayName = wallet.label || 
                                        `Wallet ${wallet.address ? wallet.address.substring(6, 14) : wallet.id.substring(0, 8)}...`;
                      return React.createElement('option', { 
                        key: wallet.id, 
                        value: wallet.id 
                      }, displayName);
                    })
                  )
                ),
                
                React.createElement('div', { className: 'mb-3' },
                  React.createElement('label', { className: 'form-label' }, 'Password'),
                  React.createElement('div', { className: 'input-group' },
                    React.createElement('input', {
                      type: showPassword ? 'text' : 'password',
                      className: 'form-control',
                      value: password,
                      onChange: (e) => setPassword(e.target.value),
                      placeholder: 'Enter wallet password',
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
                  )
                ),

                React.createElement('div', { className: 'd-grid gap-2' },
                  React.createElement('button', {
                    type: 'submit',
                    className: `btn btn-primary ${isLoading ? 'disabled' : ''}`,
                    disabled: isLoading
                  },
                    isLoading ? 
                      React.createElement('span', null,
                        React.createElement('span', { 
                          className: 'spinner-border spinner-border-sm me-2' 
                        }),
                        'Logging in...'
                      ) :
                      'Login'
                  ),
                  React.createElement('button', {
                    type: 'button',
                    className: 'btn btn-outline-secondary',
                    onClick: handleBackToWelcome
                  }, 'Back to Welcome')
                )
              )
          )
        ),

        // Wallet Management Section
        wallets.length > 0 && React.createElement('div', { className: 'card mt-3' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-gear me-2' }),
              'Wallet Management'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'list-group list-group-flush' },
              ...wallets.map(wallet =>
                React.createElement('div', { 
                  key: wallet.id,
                  className: 'list-group-item d-flex justify-content-between align-items-center px-0'
                },
                  React.createElement('div', null,
                    React.createElement('strong', null, 
                      wallet.label || `Wallet ${wallet.address ? wallet.address.substring(6, 14) : wallet.id.substring(0, 8)}...`
                    ),
                    React.createElement('br'),
                    React.createElement('small', { className: 'text-muted' }, 
                      `Created: ${wallet.createdAt ? new Date(wallet.createdAt).toLocaleDateString() : 'Unknown'}`
                    ),
                    React.createElement('br'),
                    React.createElement('small', { className: 'text-muted text-break' }, 
                      `${wallet.network}: ${wallet.address?.substring(0, 25)}...`
                    )
                  ),
                                      React.createElement('button', {
                      className: 'btn btn-outline-danger btn-sm',
                      onClick: () => handleDeleteWallet(wallet.id, wallet.label || wallet.name || `Wallet ${wallet.address?.substring(0, 8)}...`)
                    },
                    React.createElement('i', { className: 'bi bi-trash' })
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