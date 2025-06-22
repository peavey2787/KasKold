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

const { useState, useEffect, useRef } = React;

// Utility function to resolve kaspa module paths
const getKaspaModulePath = (modulePath) => {
  // Use new URL constructor to resolve path relative to the document base
  const baseUrl = new URL(document.baseURI || window.location.href);
  return new URL(`kaspa/js/${modulePath}`, baseUrl).href;
};

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
    isHDWallet: false // Whether this is an HD wallet
  });
  const [theme, setTheme] = useState('dark');
  const [notifications, setNotifications] = useState([]); // For persistent notification history
  const [toastNotifications, setToastNotifications] = useState([]); // For temporary toast notifications
  const [unreadNotificationCount, setUnreadNotificationCount] = useState(0); // For badge count
  const [isCheckingSession, setIsCheckingSession] = useState(true); // Session check state
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
      // Check if this is an HD wallet and restore HD wallet manager
      if (savedSession.isHDWallet && savedSession.currentWallet?.mnemonic) {
        const { HDWalletManager } = await import(getKaspaModulePath('hd-wallet-manager.js'));
        const hdWallet = new HDWalletManager(
          savedSession.currentWallet.mnemonic, 
          savedSession.network, 
          savedSession.currentWallet.derivationPath
        );
        await hdWallet.initialize();
        
        // Restore the wallet state with HD wallet manager
        setWalletState({
          ...savedSession,
          hdWallet: hdWallet,
          allAddresses: hdWallet.getAllAddresses()
        });
      } else {
        // Single address wallet
        setWalletState(savedSession);
      }
      
      setCurrentView('wallet-dashboard');
      addNotification('Session restored successfully', 'success');
    } catch (error) {
      console.error('Failed to restore session:', error);
      addNotification('Failed to restore session: ' + error.message, 'error');
      // Clear invalid session
      if (sessionManager.current) {
        sessionManager.current.clearSession();
      }
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
          handleWalletLogout();
          addNotification('Session expired - you have been logged out', 'warning');
        });
      }
      
      // Quick session check
      const savedSession = sessionManager.current.loadSession();
      if (savedSession && savedSession.isLoggedIn) {
        // Don't restore session yet, just continue to full initialization
        return savedSession;
      } else {
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
      const { initKaspa } = await import(getKaspaModulePath('init.js'));
      const { WalletStorage } = await import(getKaspaModulePath('wallet-storage.js'));
      
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
      sessionManager.current.saveSession(walletState);
    }
  }, [walletState.isLoggedIn, walletState.currentWallet, walletState.address]);

  const toggleTheme = () => {
    setTheme(prev => prev === 'dark' ? 'light' : 'dark');
  };

  const handleWalletLogin = async (wallet) => {
    try {
      // Check if this is an HD wallet (has mnemonic)
      const isHDWallet = !!(wallet.mnemonic);
      
      if (isHDWallet) {
        // Initialize HD wallet manager
        const { HDWalletManager } = await import(getKaspaModulePath('hd-wallet-manager.js'));
        const hdWallet = new HDWalletManager(wallet.mnemonic, wallet.network, wallet.derivationPath);
        await hdWallet.initialize();
        
        // Get current receive address
        const currentAddress = hdWallet.getCurrentReceiveAddress();
        
        setWalletState(prev => ({
          ...prev,
          isLoggedIn: true,
          currentWallet: wallet,
          address: currentAddress,
          network: wallet.network,
          hdWallet: hdWallet,
          allAddresses: hdWallet.getAllAddresses(),
          isHDWallet: true
        }));
      } else {
        // Single address wallet
        setWalletState(prev => ({
          ...prev,
          isLoggedIn: true,
          currentWallet: wallet,
          address: wallet.address,
          network: wallet.network,
          hdWallet: null,
          allAddresses: [{ address: wallet.address, type: 'single', index: 0 }],
          isHDWallet: false
        }));
      }
      
      setCurrentView('wallet-dashboard');
      addNotification(`Wallet logged in successfully (${wallet.network})`, 'success');
    } catch (error) {
      console.error('Wallet login error:', error);
      addNotification('Failed to initialize wallet: ' + error.message, 'error');
    }
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
      isHDWallet: false
    });
    setCurrentView('welcome');
    addNotification('Wallet logged out', 'info');
  };

  const navigateToView = (view) => {
    setCurrentView(view);
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

  const markAddressAsUsed = (address) => {
    if (!walletState.hdWallet || !walletState.isHDWallet) {
      return;
    }

    try {
      walletState.hdWallet.markAddressAsUsed(address);
      
      // Check if we should generate a new receive address
      if (walletState.hdWallet.shouldGenerateNewReceiveAddress()) {
        generateNewReceiveAddress();
      }
    } catch (error) {
      console.error('Mark address as used error:', error);
    }
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
        onMarkAddressUsed: markAddressAsUsed
      }),

      // Transaction Manager
      currentView === 'transaction' && React.createElement(TransactionManager, {
        walletState,
        onNavigate: navigateToView,
        addNotification,
        onGenerateChangeAddress: generateNewChangeAddress,
        onMarkAddressUsed: markAddressAsUsed
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
    })
  );
} 