export function ScriptBuilder({ walletState, onNavigate, addNotification }) {
  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row justify-content-center' },
      React.createElement('div', { className: 'col-lg-8' },
        React.createElement('div', { className: 'card' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-code-square me-2' }),
              'Script Builder'
            )
          ),
          React.createElement('div', { className: 'card-body text-center py-5' },
            React.createElement('div', { className: 'mb-4' },
              React.createElement('i', { className: 'bi bi-tools display-4 text-muted' })
            ),
            React.createElement('h6', { className: 'text-muted mb-3' }, 'Script Builder Component'),
            React.createElement('p', { className: 'text-muted mb-4' }, 'This component is being migrated to React.'),
            React.createElement('button', {
              className: 'btn btn-outline-secondary',
              onClick: () => onNavigate('wallet-dashboard')
            }, 'Back to Dashboard')
          )
        )
      )
    )
  );
} 