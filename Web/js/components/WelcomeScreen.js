const { useState } = React;

export function WelcomeScreen({ onNavigate }) {
  const [isLoading, setIsLoading] = useState(false);

  const handleCreateWallet = () => {
    setIsLoading(true);
    onNavigate('wallet-creation');
    setIsLoading(false);
  };

  const handleLoginWallet = () => {
    setIsLoading(true);
    onNavigate('wallet-login');
    setIsLoading(false);
  };

  const handleRestoreWallet = () => {
    setIsLoading(true);
    onNavigate('wallet-restore');
    setIsLoading(false);
  };

  return React.createElement('section', { 
    id: 'welcomeSection', 
    className: 'text-center py-5' 
  },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-6' },
        React.createElement('div', { className: 'welcome-logo mb-4' },
          React.createElement('img', {
            src: 'kaspa/assets/images/kaskold-logo.png',
            alt: 'Kaspa Kold Logo',
            style: { height: '80px', width: 'auto' },
            className: 'mb-3'
          })
        ),
        React.createElement('h1', { className: 'display-4 fw-bold mb-3' }, 'Welcome to'),
        React.createElement('div', { className: 'mb-4' },
          React.createElement('img', {
            src: 'kaspa/assets/images/kaskold-text.png',
            alt: 'Kaspa Kold',
            style: { height: '60px', width: 'auto' }
          })
        ),
        React.createElement('p', { className: 'lead mb-5' }, 'Air-gapped cold storage solution for the Kaspa network'),
        
        React.createElement('div', { className: 'd-grid gap-3' },
          React.createElement('button', {
            className: `btn btn-primary btn-lg ${isLoading ? 'disabled' : ''}`,
            onClick: handleCreateWallet,
            disabled: isLoading
          },
            React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
            'Create New Wallet'
          ),
          React.createElement('button', {
            className: `btn btn-outline-primary btn-lg ${isLoading ? 'disabled' : ''}`,
            onClick: handleLoginWallet,
            disabled: isLoading
          },
            React.createElement('i', { className: 'bi bi-key me-2' }),
            'Login to Existing Wallet'
          ),
          React.createElement('button', {
            className: `btn btn-outline-secondary btn-lg ${isLoading ? 'disabled' : ''}`,
            onClick: handleRestoreWallet,
            disabled: isLoading
          },
            React.createElement('i', { className: 'bi bi-arrow-clockwise me-2' }),
            'Restore Wallet'
          )
        )
      )
    )
  );
} 