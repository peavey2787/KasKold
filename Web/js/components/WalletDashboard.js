const { useState, useEffect } = React;

// Utility function to resolve kaspa module paths
const getKaspaModulePath = (modulePath) => {
  // Use new URL constructor to resolve path relative to the document base
  const baseUrl = new URL(document.baseURI || window.location.href);
  return new URL(`kaspa/js/${modulePath}`, baseUrl).href;
};

export function WalletDashboard({ walletState, onNavigate, addNotification, onGenerateNewAddress, onUpdateBalance, onMarkAddressUsed }) {
  const [balance, setBalance] = useState(null);
  const [isLoadingBalance, setIsLoadingBalance] = useState(false);
  const [lastBalanceCheck, setLastBalanceCheck] = useState(null);
  const [showAddressQR, setShowAddressQR] = useState(false);
  const [addressQRCode, setAddressQRCode] = useState(null);

  // Function to check balance
  const checkBalance = async () => {
    if (!walletState.address) {
      addNotification('No wallet address available', 'error');
      return;
    }

    setIsLoadingBalance(true);
    
    try {
      if (walletState.isHDWallet && walletState.hdWallet) {
    
        
        // For HD wallets, check balance for all addresses
        const allAddresses = walletState.hdWallet.getAllAddresses();
        const { checkAddressBalance } = await import(getKaspaModulePath('wallet-balance.js'));
        
        let totalBalance = 0n;
        let hasBalanceUpdates = false;
        
        for (const addressInfo of allAddresses) {
          const balanceResult = await checkAddressBalance(addressInfo.address, walletState.network);
          
          if (balanceResult.success) {
            const balanceInSompi = BigInt(Math.round(balanceResult.balance.kas * 100000000));
            
            // Update address balance in HD wallet
            onUpdateBalance(addressInfo.address, balanceInSompi, balanceResult.utxos || []);
            
            // Mark address as used if it has received funds
            if (balanceInSompi > 0n) {
              onMarkAddressUsed(addressInfo.address);
              hasBalanceUpdates = true;
            }
            
            totalBalance += balanceInSompi;
          }
        }
        
        // Convert back to KAS for display
        const totalKAS = Number(totalBalance) / 100000000;
        setBalance({ kas: totalKAS, sompi: totalBalance });
        setLastBalanceCheck(new Date());
        
      } else {
        // For single address wallets
        const { checkAddressBalance } = await import(getKaspaModulePath('wallet-balance.js'));
        
        const balanceResult = await checkAddressBalance(walletState.address, walletState.network);

        if (balanceResult.success) {
          setBalance(balanceResult.balance);
          setLastBalanceCheck(new Date());
          
          // Update balance in wallet state for single address wallets
          const balanceInSompi = BigInt(Math.round(balanceResult.balance.kas * 100000000));
          onUpdateBalance(walletState.address, balanceInSompi, balanceResult.utxos || []);
        } else {
          console.error('Balance check failed:', balanceResult.error);
          addNotification('Failed to check balance: ' + balanceResult.error, 'error');
          setBalance(null);
        }
      }
    } catch (error) {
      console.error('Error checking balance:', error);
      addNotification('Error checking balance: ' + error.message, 'error');
      setBalance(null);
    } finally {
      setIsLoadingBalance(false);
    }
  };

  // Auto-check balance when wallet loads and sync with wallet state
  useEffect(() => {
    if (walletState.address && !balance) {
      // For HD wallets, use the total balance from HD wallet manager if available
      if (walletState.isHDWallet && walletState.hdWallet) {
        const totalBalance = walletState.hdWallet.getTotalBalance();
        if (totalBalance > 0n) {
          const totalKAS = Number(totalBalance) / 100000000;
          setBalance({ kas: totalKAS, sompi: totalBalance });
        } else {
          checkBalance();
        }
      } else {
        checkBalance();
      }
    }
  }, [walletState.address, walletState.hdWallet]);

  // Sync balance with wallet state when it updates
  useEffect(() => {
    if (walletState.balance !== null && walletState.balance !== undefined) {
      if (typeof walletState.balance === 'bigint') {
        const totalKAS = Number(walletState.balance) / 100000000;
        setBalance({ kas: totalKAS, sompi: walletState.balance });
      } else if (typeof walletState.balance === 'object') {
        setBalance(walletState.balance);
      }
    }
  }, [walletState.balance]);

  const formatBalance = (balanceObj) => {
    if (!balanceObj) return '0.00';
    return balanceObj.kas.toFixed(8);
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
      const { generateQRCode } = await import(`${getKaspaModulePath('qr-manager.js')}?v=${Date.now()}`);
      
      const qrResult = await generateQRCode(walletState.address, {
        width: 300,
        height: 300,
        margin: 2
      });

      if (qrResult.success) {
        setAddressQRCode(qrResult.qrDataURL);
        setShowAddressQR(true);
      } else {
        addNotification('Failed to generate QR code: ' + qrResult.error, 'error');
      }
    } catch (error) {
      console.error('QR generation error:', error);
      addNotification('Failed to generate QR code: ' + error.message, 'error');
    }
  };

  const closeAddressQR = () => {
    setShowAddressQR(false);
    setAddressQRCode(null);
  };

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
                React.createElement('button', {
                  className: `btn btn-outline-primary btn-sm ${isLoadingBalance ? 'disabled' : ''}`,
                  onClick: checkBalance,
                  disabled: isLoadingBalance
                },
                  isLoadingBalance ? 
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
                  React.createElement('i', { className: 'bi bi-send' }),
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
              'Wallet Address QR Code'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeAddressQR
            })
          ),
          React.createElement('div', { className: 'modal-body text-center' },
            addressQRCode && React.createElement('div', { className: 'mb-3' },
              React.createElement('img', {
                src: addressQRCode,
                alt: 'Wallet Address QR Code',
                className: 'img-fluid border rounded',
                style: { maxWidth: '300px' }
              })
            ),
            React.createElement('div', { className: 'mt-3' },
              React.createElement('small', { className: 'text-muted d-block mb-2' }, 'Wallet Address:'),
              React.createElement('code', { 
                className: 'text-primary fw-bold d-block text-break',
                style: { fontSize: '0.8em' }
              }, 
                walletState.address
              )
            )
          ),
          React.createElement('div', { className: 'modal-footer' },
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
    )
  );
} 