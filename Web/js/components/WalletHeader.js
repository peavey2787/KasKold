const { useState } = React;

export function WalletHeader({ theme, onThemeToggle, walletState, onLogout, notifications, toastNotifications, unreadNotificationCount, onClearBadge, network, onNetworkChange, onNavigate, showNetworkSelector = false }) {
  const [showNotificationModal, setShowNotificationModal] = useState(false);
  
  const handleNetworkChange = (e) => {
    onNetworkChange(e.target.value);
  };

  const handleNotificationClick = () => {
    setShowNotificationModal(true);
    // Clear the badge when user opens notification history
    if (onClearBadge) {
      onClearBadge();
    }
  };

  const closeNotificationModal = () => {
    setShowNotificationModal(false);
  };

  const formatNotificationTime = (timestamp) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const getNotificationIcon = (type) => {
    switch (type) {
      case 'success': return 'bi-check-circle-fill text-success';
      case 'error': return 'bi-exclamation-circle-fill text-danger';
      case 'warning': return 'bi-exclamation-triangle-fill text-warning';
      case 'info': return 'bi-info-circle-fill text-info';
      default: return 'bi-bell-fill text-primary';
    }
  };

  const handleLogoClick = () => {
    if (walletState.isLoggedIn) {
      onNavigate('wallet-dashboard');
    } else {
      onNavigate('welcome');
    }
  };

  return React.createElement(React.Fragment, null,
    // Navigation Bar
    React.createElement('nav', { className: 'navbar navbar-expand-lg sticky-top' },
      React.createElement('div', { className: 'container' },
        React.createElement('div', { 
          className: 'navbar-brand d-flex align-items-center cursor-pointer',
          onClick: handleLogoClick,
          style: { cursor: 'pointer' }
        },
          React.createElement('img', { 
            src: 'kaspa/assets/images/kaskold-logo.png',
            alt: 'Kaspa Kold Logo',
            className: 'me-2',
            style: { height: '32px', width: 'auto' }
          }),
          React.createElement('img', { 
            src: 'kaspa/assets/images/kaskold-text.png',
            alt: 'Kaspa Kold',
            style: { height: '24px', width: 'auto' }
          })
        ),
        
        React.createElement('div', { className: 'navbar-nav ms-auto d-flex flex-row align-items-center' },
          // Network Selector (only show when specified) or Network Indicator for testnet
          showNetworkSelector ? 
            React.createElement('div', { className: 'nav-item me-3' },
              React.createElement('select', {
                id: 'networkType',
                className: 'form-select form-select-sm',
                value: network,
                onChange: handleNetworkChange
              },
                React.createElement('option', { value: 'mainnet' }, 'Mainnet'),
                React.createElement('option', { value: 'testnet-10' }, 'Testnet-10'),
                React.createElement('option', { value: 'testnet-11' }, 'Testnet-11'),
                React.createElement('option', { value: 'devnet' }, 'Devnet'),
                React.createElement('option', { value: 'simnet' }, 'Simnet')
              )
            ) :
            // Show network indicator for non-mainnet when logged in
            (walletState.isLoggedIn && network !== 'mainnet') &&
            React.createElement('div', { className: 'nav-item me-3' },
              React.createElement('span', { 
                className: 'badge bg-warning text-dark px-2 py-1',
                style: { fontSize: '0.75rem' }
              }, network.toUpperCase())
            ),
          
          // Theme Toggle
          React.createElement('button', {
            className: 'btn btn-outline-primary btn-sm me-2',
            onClick: onThemeToggle
          },
            React.createElement('i', { 
              className: theme === 'dark' ? 'bi bi-sun-fill' : 'bi bi-moon-fill' 
            })
          ),
          
          // Notification Badge
          React.createElement('button', {
            className: 'btn btn-outline-secondary btn-sm me-2 position-relative',
            onClick: handleNotificationClick
          },
            React.createElement('i', { className: 'bi bi-bell' }),
            unreadNotificationCount > 0 && React.createElement('span', {
              className: 'badge bg-danger'
            }, unreadNotificationCount > 99 ? '99+' : unreadNotificationCount)
          ),
          
          // Logout Button (only show when logged in)
          walletState.isLoggedIn && React.createElement('button', {
            className: 'btn btn-outline-danger btn-sm',
            onClick: onLogout
          },
            React.createElement('i', { className: 'bi bi-box-arrow-right me-1' }),
            'Logout'
          )
        )
      )
    ),

    // Notification Modal
    showNotificationModal && React.createElement('div', {
      className: 'modal fade show',
      style: { display: 'block', backgroundColor: 'rgba(0,0,0,0.5)' },
      onClick: closeNotificationModal
    },
      React.createElement('div', {
        className: 'modal-dialog modal-dialog-centered modal-lg',
        onClick: (e) => e.stopPropagation()
      },
        React.createElement('div', { className: 'modal-content' },
          React.createElement('div', { className: 'modal-header' },
            React.createElement('h5', { className: 'modal-title' },
              React.createElement('i', { className: 'bi bi-bell me-2' }),
              'Notification History'
            ),
            React.createElement('button', {
              type: 'button',
              className: 'btn-close',
              onClick: closeNotificationModal
            })
          ),
          React.createElement('div', { className: 'modal-body' },
            notifications.length === 0 ? 
              React.createElement('div', { className: 'text-center py-4' },
                React.createElement('i', { className: 'bi bi-bell-slash display-4 text-muted mb-3' }),
                React.createElement('h6', { className: 'text-muted' }, 'No notifications yet'),
                React.createElement('p', { className: 'text-muted mb-0' }, 'Notifications will appear here when you perform actions in the wallet.')
              ) :
              React.createElement('div', { className: 'list-group list-group-flush' },
                notifications.slice(0, 20).map((notification, index) =>
                  React.createElement('div', {
                    key: `notification-${notification.id}-${index}`,
                    className: `list-group-item d-flex align-items-start ${index === 0 ? 'border-top-0' : ''}`
                  },
                    React.createElement('div', { className: 'me-3 mt-1' },
                      React.createElement('i', { className: getNotificationIcon(notification.type) })
                    ),
                    React.createElement('div', { className: 'flex-grow-1' },
                      React.createElement('div', { className: 'fw-medium mb-1' }, notification.message),
                      React.createElement('small', { className: 'text-muted' }, 
                        formatNotificationTime(notification.timestamp)
                      )
                    )
                  )
                ),
                notifications.length > 20 && React.createElement('div', {
                  className: 'list-group-item text-center text-muted'
                },
                  React.createElement('small', null, `... and ${notifications.length - 20} more notifications`)
                )
              )
          ),
          React.createElement('div', { className: 'modal-footer' },
            React.createElement('button', {
              type: 'button',
              className: 'btn btn-secondary',
              onClick: closeNotificationModal
            }, 'Close')
          )
        )
      )
    )
  );
} 