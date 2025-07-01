// Import other components
import { WalletHeader } from './WalletHeader.js';
import { WelcomeScreen } from './WelcomeScreen.js';
import { WalletCreation } from './WalletCreation.js';
import { WalletLogin } from './WalletLogin.js';
import { WalletRestore } from './WalletRestore.js';
import { WalletDashboard } from './WalletDashboard.js';
import { TransactionManager } from './TransactionManager.js';
import { MessageSigning } from './MessageSigning.js';
import { ScriptBuilder } from './ScriptBuilder.js';
import { WalletSettings } from './WalletSettings.js';
import { ToastContainer } from './ToastContainer.js';
import { DEFAULT_ACCOUNT_PATH } from '../../kaspa/js/constants.js';

const { useState, useEffect, useRef } = React;

// Main Wallet Application Component
export function WalletApp() {
  const [currentView, setCurrentView] = useState('loading'); // Start with loading state
  const [walletState, setWalletState] = useState({
    isLoggedIn: false,
    currentWallet: null,
    balance: null,
    address: null, // Current active receive address
    network: 'mainnet',
    hdWallet: null, // HD wallet manager instance
    allAddresses: [], // All generated addresses
    isHDWallet: false, // Whether this is an HD wallet
    mnemonic: null,
    privateKey: null,
    derivationPath: null
  });
  const [cachedUTXOs, setCachedUTXOs] = useState(null); // Cached UTXO data for offline transactions
  const [theme, setTheme] = useState('dark');
  const [notifications, setNotifications] = useState([]); // For persistent notification history
  const [toastNotifications, setToastNotifications] = useState([]); // For temporary toast notifications
  const [unreadNotificationCount, setUnreadNotificationCount] = useState(0); // For badge count
  const [isCheckingSession, setIsCheckingSession] = useState(true); // Session check state
  const [navigationData, setNavigationData] = useState(null); // Data to pass between views
  const [showSessionWarning, setShowSessionWarning] = useState(false);
  const [sessionWarningMinutes, setSessionWarningMinutes] = useState(0);
  const [warningCountdown, setWarningCountdown] = useState(0);
  const kaspaInitialized = useRef(false);
  const initializationInProgress = useRef(false);
  const sessionManager = useRef(null);

  // Define addNotification first so it's available in useEffect
  const addNotification = (message, type = 'info') => {
    const notification = {
      id: Date.now() + Math.random(), // Ensure unique ID
      message,
      type,
      timestamp: new Date(),
      isRead: false // Track read status
    };
    // Add to persistent notifications (for history)
    setNotifications(prev => [notification, ...prev]);
    
    // Add to toast notifications (for temporary display)
    setToastNotifications(prev => [notification, ...prev]);
    
    // Increment unread count for badge (will be decremented if manually dismissed)
    setUnreadNotificationCount(prev => prev + 1);
  };

  // Restore wallet session from saved data
  const restoreWalletSession = async (savedSession) => {
    try {
      console.log('💾 SESSION: Restoring session...', { 
        isHDWallet: savedSession.isHDWallet,
        hasMnemonic: !!savedSession.mnemonic,
        currentWallet: !!savedSession.currentWallet
      });

      // Check if this is an HD wallet and restore HD wallet manager
      if (savedSession.isHDWallet && (savedSession.mnemonic || savedSession.currentWallet?.mnemonic)) {
        const { getHDWallet } = await import('../../kaspa/js/wallet-manager.js');
        const mnemonic = savedSession.mnemonic || savedSession.currentWallet.mnemonic;
        let derivationPath = savedSession.derivationPath || savedSession.currentWallet?.derivationPath;
        
        // Ensure we use account-level path for HD wallet manager
        if (derivationPath && derivationPath.includes('/0/0')) {
          // Convert address-level path to account-level path
          derivationPath = derivationPath.split('/0/0')[0];
          console.log('🔧 FIXED: Converted address-level path to account-level:', derivationPath);
        } else if (!derivationPath) {
          // Default to Kaspa account-level path
          derivationPath = DEFAULT_ACCOUNT_PATH;
          console.log('🔧 FIXED: Using default account-level path:', derivationPath);
        }
        
        const hdWallet = getHDWallet(mnemonic, savedSession.network, derivationPath);
        await hdWallet.initialize();
        
        // Restore addresses if available
        if (savedSession.allAddresses && savedSession.allAddresses.length > 0) {
          // Restore the address state to HD wallet
          for (const addr of savedSession.allAddresses) {
            if (addr.type === 'receive' && addr.index !== undefined) {
              // Generate addresses up to the saved index
              while (hdWallet.receiveAddressIndex < addr.index) {
                await hdWallet.generateNextReceiveAddress();
              }
            }
            if (addr.balance && addr.balance > 0) {
              hdWallet.updateAddressBalance(addr.address, addr.balance, []);
            }
            if (addr.used) {
              hdWallet.markAddressAsUsed(addr.address);
            }
          }
        } else {
          // Generate initial address if no addresses were saved
          await hdWallet.generateNextReceiveAddress();
        }
        
        // Restore the wallet state with HD wallet manager
        // Always use the HD wallet's current receive address as the main address (with UTXO check)
        const currentReceiveAddress = await hdWallet.getCurrentReceiveAddress();
        setWalletState({
          ...savedSession,
          hdWallet: hdWallet,
          allAddresses: hdWallet.getAllAddresses(),
          address: currentReceiveAddress,
          balance: null // Reset balance to null - it will be fetched fresh
        });
        
        // Log address synchronization for debugging
        if (savedSession.address && savedSession.address !== currentReceiveAddress) {
          console.log('🔧 SYNC: Updated main address from saved session:', {
            savedAddress: savedSession.address,
            newAddress: currentReceiveAddress
          });
        }
      } else {
        // Single address wallet - reset balance to null for fresh fetch
        setWalletState({
          ...savedSession,
          balance: null // Reset balance to null - it will be fetched fresh
        });
      }
      
      setCurrentView('wallet-dashboard');
      addNotification('Session restored successfully', 'success');
      console.log('💾 SESSION: Session restored successfully');
    } catch (error) {
      console.error('💾 SESSION: Failed to restore session:', error);
      addNotification('Failed to restore session: ' + error.message, 'error');
      // Clear invalid session
      if (sessionManager.current) {
        sessionManager.current.clearSession();
      }
      setCurrentView('welcome');
    }
  };

  // Quick session check before full initialization
  const checkForExistingSession = async () => {
    try {
      // Load saved theme immediately
      const savedTheme = localStorage.getItem('kaspa_theme') || 'dark';
      setTheme(savedTheme);
      
      // Initialize session manager for quick check
      if (!sessionManager.current) {
        const { SessionManager } = await import('../session-manager.js');
        sessionManager.current = new SessionManager();
        
        // Set up session expiration callback
        sessionManager.current.setSessionExpiredCallback(() => {
          addNotification('Session expired. Please log in again.', 'warning');
          handleWalletLogout();
        });
        
        // Set session warning callback
        sessionManager.current.setSessionWarningCallback((minutesRemaining) => {
          setSessionWarningMinutes(minutesRemaining);
          setWarningCountdown(minutesRemaining * 60); // Convert to seconds
          setShowSessionWarning(true);
          
          // Start countdown timer
          const countdownInterval = setInterval(() => {
            setWarningCountdown(prev => {
              if (prev <= 1) {
                clearInterval(countdownInterval);
                setShowSessionWarning(false);
                return 0;
              }
              return prev - 1;
            });
          }, 1000);
        });
      }
      
      // Quick session check
      const savedSession = sessionManager.current.loadSession();
      console.log('💾 SESSION: Quick session check result:', { 
        hasSavedSession: !!savedSession,
        isLoggedIn: savedSession?.isLoggedIn,
        isHDWallet: savedSession?.isHDWallet,
        address: savedSession?.address
      });
      
      if (savedSession && savedSession.isLoggedIn) {
        // Don't restore session yet, just continue to full initialization
        return savedSession;
      } else {
        console.log('💾 SESSION: No valid session found, going to welcome screen');
        setIsCheckingSession(false);
        setCurrentView('welcome');
        return null;
      }
    } catch (error) {
      console.error('React WalletApp: Error checking session:', error);
      setIsCheckingSession(false);
      setCurrentView('welcome');
      return null;
    }
  };

  const initializeKaspaWallet = async (existingSession = null) => {
    // Prevent multiple simultaneous initializations
    if (initializationInProgress.current) {
      return;
    }
    
    initializationInProgress.current = true;
    
    try {
      // Import and initialize Kaspa modules
      const { initKaspa } = await import('../../kaspa/js/init.js');
      const { WalletStorage } = await import('../../kaspa/js/wallet-storage.js');
      
      // Initialize Kaspa WASM (check if already initialized globally)
      if (!kaspaInitialized.current && !window.kaspaInitialized) {
        await initKaspa();
        kaspaInitialized.current = true;
        window.kaspaInitialized = true;
      } else {
        kaspaInitialized.current = true;
      }

      // Initialize wallet storage
      const walletStorage = new WalletStorage();
      const savedWallets = await walletStorage.getAllWallets();

      // If we have an existing session, restore it now
      if (existingSession) {
        await restoreWalletSession(existingSession);
      }
      
      // Finish loading state
      setIsCheckingSession(false);
      if (!existingSession) {
        setCurrentView('welcome');
      }

    } catch (error) {
      console.error('React WalletApp: Failed to initialize Kaspa wallet:', error);
      console.error('React WalletApp: Error stack:', error.stack);
      addNotification('Failed to initialize wallet system: ' + error.message, 'error');
      setIsCheckingSession(false);
      setCurrentView('welcome');
    } finally {
      initializationInProgress.current = false;
    }
  };

  // Initialize app with session check first
  useEffect(() => {
    const initializeApp = async () => {
      const existingSession = await checkForExistingSession();
      if (existingSession) {
        // If session exists, continue with full initialization
        await initializeKaspaWallet(existingSession);
      } else {
        // If no session, still do minimal initialization for wallet storage
        await initializeKaspaWallet();
      }
    };
    
    initializeApp();
  }, []);

  // Theme management
  useEffect(() => {
    document.body.className = theme === 'dark' ? 'dark-theme' : 'light-theme';
    document.documentElement.setAttribute('data-bs-theme', theme);
    localStorage.setItem('kaspa_theme', theme);
  }, [theme]);

  // Save session when wallet state changes
  useEffect(() => {
    if (sessionManager.current && walletState.isLoggedIn) {
      // Create a serializable copy of wallet state without hdWallet instance
      const sessionState = {
        ...walletState,
        hdWallet: null // Don't save the hdWallet instance - it will be recreated on restore
      };
      sessionManager.current.saveSession(sessionState);
    }
  }, [walletState.isLoggedIn, walletState.currentWallet, walletState.address]);

  const toggleTheme = () => {
    setTheme(prev => prev === 'dark' ? 'light' : 'dark');
  };

  const handleWalletLogin = async (wallet) => {
    
    try {
      // Initialize HD Wallet if needed
      let hdWallet = null;
      if (wallet.mnemonic) {
        const { getHDWallet } = await import('../../kaspa/js/wallet-manager.js');
        
        // Convert address-level path to account-level path for HD wallet manager
        let derivationPath = wallet.derivationPath;
        if (derivationPath && derivationPath.includes(DEFAULT_ACCOUNT_PATH + '/')) {
          // Convert from address-level to account-level
          derivationPath = DEFAULT_ACCOUNT_PATH;
          console.log('🔧 FIXED: Converted address-level path to account-level for HD wallet:', derivationPath);
        } else if (!derivationPath) {
          // Default to Kaspa account-level path
          derivationPath = DEFAULT_ACCOUNT_PATH;
          console.log('🔧 FIXED: Using default account-level path:', derivationPath);
        }
        
        hdWallet = getHDWallet(wallet.mnemonic, wallet.network, derivationPath);
        
        // Initialize the HD wallet
        await hdWallet.initialize();
        
        // If importing existing wallet state
        if (wallet.hdWalletState) {
          await hdWallet.importState(wallet.hdWalletState);
        } else {
          // Generate initial address
          await hdWallet.generateNextReceiveAddress();
        }
        
        // PRIVACY ENHANCEMENT: Ensure we show a fresh address
        // Check if current address has UTXOs and generate new one if needed
        await ensureFreshReceiveAddress(hdWallet);
      }

      setWalletState({
        isLoggedIn: true,
        currentWallet: wallet,
        address: hdWallet ? await hdWallet.getCurrentReceiveAddress() : wallet.address,
        balance: 0n,
        network: wallet.network,
        mnemonic: wallet.mnemonic || null,
        privateKey: wallet.privateKey || null,
        derivationPath: wallet.derivationPath || null,
        isHDWallet: !!wallet.mnemonic,
        hdWallet: hdWallet,
        allAddresses: hdWallet ? hdWallet.getAllAddresses() : []
      });

      // Note: Session saving is handled by the useEffect hook that watches walletState changes
      // No need to save session here manually as it will be saved automatically

      navigateToView('wallet-dashboard');
      addNotification('Wallet loaded successfully', 'success');
    } catch (error) {
      console.error('Wallet login error:', error);
      addNotification('Failed to load wallet: ' + error.message, 'error');
    }
  };

  // PRIVACY ENHANCEMENT: Ensure fresh receive address
  const ensureFreshReceiveAddress = async (hdWallet) => {
    try {
      
      const currentAddress = hdWallet.getCurrentReceiveAddress();
      
      // Check if current address has been used or has balance/UTXOs
      let needsNewAddress = hdWallet.shouldGenerateNewReceiveAddress();
      
      // Double-check by actually querying the blockchain for UTXOs
      if (!needsNewAddress) {
        try {
          const { getSingleWallet } = await import('../../kaspa/js/wallet-manager.js');
          const singleWallet = getSingleWallet(currentAddress, hdWallet.network);
          await singleWallet.initialize();
          const balanceResult = await singleWallet.checkSingleAddressBalance(currentAddress);
          
          if (balanceResult.success && (balanceResult.balance.kas > 0 || balanceResult.utxoCount > 0)) {
            needsNewAddress = true;
            
            // Update HD wallet with the found balance/UTXOs using safe conversion
            const { kasToSompi } = await import('../../kaspa/js/currency-utils.js');
            const balanceInSompi = kasToSompi(balanceResult.balance.kas.toString());
            hdWallet.updateAddressBalance(currentAddress, balanceInSompi, balanceResult.utxos || []);
            hdWallet.markAddressAsUsed(currentAddress);
          }
        } catch (balanceError) {
          console.warn('🔒 PRIVACY: Could not check address balance, proceeding with used flag only:', balanceError);
        }
      }
      
      // Generate new address if needed
      if (needsNewAddress) {
        const newAddress = await hdWallet.generateNextReceiveAddress();
        addNotification('Generated fresh receive address for privacy', 'info');
        return newAddress;
      } else {
        return null;
      }
    } catch (error) {
      console.error('🔒 PRIVACY: Error ensuring fresh address:', error);
      // Non-critical error, don't interrupt wallet login
      return null;
    }
  };

  // Handle session warning extension
  const extendSession = () => {
    console.log('🔄 UI: Extend session button clicked');

    if (sessionManager.current) {
      console.log('🔄 UI: Session manager available, calling extendSession');
      const extended = sessionManager.current.extendSession();
      console.log('🔄 UI: ExtendSession result:', extended);

      if (extended) {
        setShowSessionWarning(false);
        addNotification('Session extended successfully', 'success');
        console.log('🔄 UI: Session extended successfully');
      } else {
        addNotification('Failed to extend session', 'error');
        console.log('🔄 UI: Session extension failed');
      }
    } else {
      console.error('🔄 UI: Session manager not available');
      addNotification('Session manager not available', 'error');
    }
  };

  // Close session warning (user chose not to extend)
  const closeSessionWarning = () => {
    setShowSessionWarning(false);
  };

  const handleWalletLogout = () => {
    // Clear session data
    if (sessionManager.current) {
      sessionManager.current.clearSession();
    }
    
    setWalletState({
      isLoggedIn: false,
      currentWallet: null,
      balance: null,
      address: null,
      network: walletState.network,
      hdWallet: null,
      allAddresses: [],
      isHDWallet: false,
      mnemonic: null,
      privateKey: null,
      derivationPath: null
    });
    setCurrentView('welcome');
    addNotification('Wallet logged out', 'info');
  };

  const navigateToView = (view, data = null) => {
    setCurrentView(view);
    setNavigationData(data);
  };

  const clearNotificationBadge = () => {
    // Clear the toast notifications and reset unread count
    setToastNotifications([]);
    setUnreadNotificationCount(0);
  };

  const handleToastDismiss = (id) => {
    // Find the notification being dismissed
    const dismissedNotification = toastNotifications.find(n => n.id === id);
    
    // Remove from toast notifications
    setToastNotifications(prev => prev.filter(n => n.id !== id));
    
    // If notification was manually dismissed and not yet read, decrement badge count
    if (dismissedNotification && !dismissedNotification.isRead) {
      setUnreadNotificationCount(prev => Math.max(0, prev - 1));
      
      // Mark as read in persistent notifications
      setNotifications(prev => prev.map(n => 
        n.id === id ? { ...n, isRead: true } : n
      ));
    }
  };

  // HD Wallet address management functions
  const generateNewReceiveAddress = async () => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      addNotification('Address generation only available for HD wallets', 'warning');
      return null;
    }

    try {
      const newAddress = await walletState.hdWallet.generateNextReceiveAddress();
      
      // Update wallet state with new address as current
      setWalletState(prev => ({
        ...prev,
        address: newAddress.address,
        allAddresses: prev.hdWallet.getAllAddresses()
      }));
      
      addNotification(`New receive address generated: ${newAddress.address.substring(0, 20)}...`, 'success');
      return newAddress;
    } catch (error) {
      console.error('Address generation error:', error);
      addNotification('Failed to generate new address: ' + error.message, 'error');
      return null;
    }
  };

  const generateNewChangeAddress = async () => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      return null;
    }

    try {
      const newAddress = await walletState.hdWallet.generateNextChangeAddress();
      
      // Update wallet state
      setWalletState(prev => ({
        ...prev,
        allAddresses: prev.hdWallet.getAllAddresses()
      }));
      
      return newAddress;
    } catch (error) {
      console.error('Change address generation error:', error);
      return null;
    }
  };

  const updateAddressBalance = (address, balance, utxos = []) => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      // For single address wallets, update the main balance
      setWalletState(prev => ({
        ...prev,
        balance: balance
      }));
      return;
    }

    try {
      walletState.hdWallet.updateAddressBalance(address, balance, utxos);
      
      // Update wallet state with new total balance
      setWalletState(prev => ({
        ...prev,
        balance: prev.hdWallet.getTotalBalance(),
        allAddresses: prev.hdWallet.getAllAddresses()
      }));
    } catch (error) {
      console.error('Balance update error:', error);
    }
  };

  const markAddressAsUsed = async (address) => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      return;
    }

    try {
      walletState.hdWallet.markAddressAsUsed(address);

      // Check if we should generate a new receive address (async check for UTXOs)
      if (await walletState.hdWallet.shouldGenerateNewReceiveAddress()) {
        await generateNewReceiveAddress();
      }
    } catch (error) {
      console.error('Mark address as used error:', error);
    }
  };

  // Check and update receive address if current one has UTXOs
  const ensureCleanReceiveAddress = async () => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      return;
    }

    try {
      const currentAddress = walletState.address;
      if (currentAddress && await walletState.hdWallet.addressHasUTXOs(currentAddress)) {
        console.log('🚨 SECURITY: Current receive address has UTXOs, generating new one');
        const newAddress = await generateNewReceiveAddress();
        if (newAddress) {
          addNotification('Generated new receive address for security', 'info');
        }
      }
    } catch (error) {
      console.error('Error checking receive address:', error);
    }
  };

  // Handle caching UTXOs from WalletDashboard
  const handleCacheUTXOs = (utxoData) => {
    setCachedUTXOs(utxoData);
  };

  // Clear cached UTXOs
  const clearCachedUTXOs = () => {
    setCachedUTXOs(null);
  };

  return React.createElement('div', { className: 'wallet-app' },
    // Header Navigation
    React.createElement(WalletHeader, {
      theme,
      onThemeToggle: toggleTheme,
      walletState,
      onLogout: handleWalletLogout,
      notifications,
      toastNotifications,
      unreadNotificationCount,
      onClearBadge: clearNotificationBadge,
      network: walletState.network,
      onNetworkChange: (network) => setWalletState(prev => ({ ...prev, network })),
      onNavigate: navigateToView,
      showNetworkSelector: false
    }),

    // Main Content Area
    React.createElement('main', { className: 'container py-4' },
      // Loading/Session Check Screen
      (currentView === 'loading' || isCheckingSession) && React.createElement('div', { 
        className: 'row justify-content-center align-items-center',
        style: { minHeight: '60vh' }
      },
        React.createElement('div', { className: 'col-md-6 text-center' },
          React.createElement('div', { className: 'card' },
            React.createElement('div', { className: 'card-body py-5' },
              React.createElement('div', { className: 'mb-4' },
                React.createElement('div', { 
                  className: 'spinner-border text-primary',
                  style: { width: '3rem', height: '3rem' },
                  role: 'status'
                },
                  React.createElement('span', { className: 'visually-hidden' }, 'Loading...')
                )
              ),
              React.createElement('h4', { className: 'card-title mb-3' }, 'Starting Kaspa Wallet'),
              React.createElement('p', { className: 'text-muted mb-0' },
                isCheckingSession ? 'Checking for previous session...' : 'Initializing wallet system...'
              )
            )
          )
        )
      ),

      // Welcome Screen
      currentView === 'welcome' && React.createElement(WelcomeScreen, {
        onNavigate: navigateToView
      }),

      // Wallet Creation
      currentView === 'wallet-creation' && React.createElement(WalletCreation, {
        onNavigate: navigateToView,
        onWalletCreated: handleWalletLogin,
        addNotification,
        network: walletState.network
      }),

      // Wallet Login
      currentView === 'wallet-login' && React.createElement(WalletLogin, {
        onNavigate: navigateToView,
        onWalletLogin: handleWalletLogin,
        addNotification
      }),

      // Wallet Restore
      currentView === 'wallet-restore' && React.createElement(WalletRestore, {
        onNavigate: navigateToView,
        onWalletRestored: handleWalletLogin,
        addNotification,
        network: walletState.network
      }),

      // Wallet Dashboard
      currentView === 'wallet-dashboard' && React.createElement(WalletDashboard, {
        walletState,
        onNavigate: navigateToView,
        addNotification,
        onGenerateNewAddress: generateNewReceiveAddress,
        onUpdateBalance: updateAddressBalance,
        onMarkAddressUsed: markAddressAsUsed,
        onEnsureCleanReceiveAddress: ensureCleanReceiveAddress,
        cachedUTXOs,
        onCacheUTXOs: handleCacheUTXOs,
        onClearCachedUTXOs: clearCachedUTXOs
      }),

      // Transaction Manager
      currentView === 'transaction' && React.createElement(TransactionManager, {
        walletState,
        onNavigate: navigateToView,
        addNotification,
        onGenerateChangeAddress: generateNewChangeAddress,
        onGenerateNewAddress: generateNewReceiveAddress,
        onMarkAddressUsed: markAddressAsUsed,
        cachedUTXOs,
        onClearCachedUTXOs: clearCachedUTXOs,
        navigationData
      }),

      // Message Signing
      currentView === 'message-signing' && React.createElement(MessageSigning, {
        walletState,
        onNavigate: navigateToView,
        addNotification
      }),

      // Script Builder
      currentView === 'script-builder' && React.createElement(ScriptBuilder, {
        walletState,
        onNavigate: navigateToView,
        addNotification
      }),

      // Wallet Settings
      currentView === 'wallet-settings' && React.createElement(WalletSettings, {
        walletState,
        onNavigate: navigateToView,
        addNotification,
        onGenerateNewAddress: generateNewReceiveAddress,
        sessionManager: sessionManager.current
      })
    ),

    // Toast Notifications
    React.createElement(ToastContainer, {
      notifications: toastNotifications,
      onDismiss: handleToastDismiss
    }),

    // Session Warning Modal
    showSessionWarning && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.8)' }
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered'
      },
        React.createElement('div', { className: 'modal-content border-warning' },
          React.createElement('div', { className: 'modal-header bg-warning text-dark' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-exclamation-triangle me-2' }),
              'Session Expiring Soon'
            )
          ),
          React.createElement('div', { className: 'modal-body text-center' },
            React.createElement('div', { className: 'mb-3' },
              React.createElement('i', { 
                className: 'bi bi-clock text-warning',
                style: { fontSize: '3rem' }
              })
            ),
            React.createElement('h6', { className: 'mb-3' },
              `Your session will expire in ${Math.floor(warningCountdown / 60)}:${String(warningCountdown % 60).padStart(2, '0')}`
            ),
            React.createElement('p', { className: 'text-muted' },
              'Would you like to extend your session? If you don\'t respond, you will be automatically logged out.'
            ),
            React.createElement('div', { className: 'progress mb-3', style: { height: '8px' } },
              React.createElement('div', {
                className: 'progress-bar bg-warning',
                style: { 
                  width: `${(warningCountdown / (sessionWarningMinutes * 60)) * 100}%`,
                  transition: 'width 1s linear'
                }
              })
            )
          ),
          React.createElement('div', { className: 'modal-footer' },
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-secondary',
              onClick: closeSessionWarning
            }, 'Logout Now'),
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-warning',
              onClick: extendSession
            },
              React.createElement('i', { className: 'bi bi-clock-history me-2' }),
              'Extend Session'
            )
          )
        )
      )
    )
  );
} 