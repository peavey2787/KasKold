const { useEffect, useRef } = React;

export function ToastContainer({ notifications, onDismiss }) {
  const timeoutsRef = useRef(new Map());
  
  useEffect(() => {
    // Auto-dismiss notifications after 5 seconds (all types including errors)
    notifications.forEach(notification => {
      // Only set timeout if we haven't already set one for this notification
      if (!timeoutsRef.current.has(notification.id)) {
        const timeoutId = setTimeout(() => {
          onDismiss(notification.id);
          timeoutsRef.current.delete(notification.id);
        }, 5000);
        timeoutsRef.current.set(notification.id, timeoutId);
      }
    });

    // Clean up timeouts for notifications that are no longer in the array
    const currentNotificationIds = new Set(notifications.map(n => n.id));
    for (const [notificationId, timeoutId] of timeoutsRef.current.entries()) {
      if (!currentNotificationIds.has(notificationId)) {
        clearTimeout(timeoutId);
        timeoutsRef.current.delete(notificationId);
      }
    }
  }, [notifications, onDismiss]);

  // Clean up all timeouts on unmount
  useEffect(() => {
    return () => {
      for (const timeoutId of timeoutsRef.current.values()) {
        clearTimeout(timeoutId);
      }
      timeoutsRef.current.clear();
    };
  }, []);

  const getToastClass = (type) => {
    switch (type) {
      case 'success': return 'toast-success bg-success';
      case 'error': return 'toast-error bg-danger';
      case 'warning': return 'toast-warning bg-warning';
      default: return 'toast-info bg-info';
    }
  };

  const getToastIcon = (type) => {
    switch (type) {
      case 'success': return 'bi bi-check-circle';
      case 'error': return 'bi bi-exclamation-triangle';
      case 'warning': return 'bi bi-exclamation-circle';
      default: return 'bi bi-info-circle';
    }
  };

  return React.createElement('div', {
    className: 'toast-container position-fixed top-0 end-0 p-3',
    style: { zIndex: 1060 }
  },
    ...notifications.slice(0, 5).map(notification => // Show max 5 notifications
      React.createElement('div', {
        key: notification.id,
        className: `toast show ${getToastClass(notification.type)} text-white`,
        role: 'alert'
      },
        React.createElement('div', { className: 'toast-header' },
          React.createElement('i', { className: `${getToastIcon(notification.type)} me-2` }),
          React.createElement('strong', { className: 'me-auto' }, 
            notification.type.charAt(0).toUpperCase() + notification.type.slice(1)
          ),
          React.createElement('small', null, 
            new Date(notification.timestamp).toLocaleTimeString()
          ),
          React.createElement('button', {
            type: 'button',
            className: 'btn-close',
            onClick: () => onDismiss(notification.id)
          })
        ),
        React.createElement('div', { className: 'toast-body' },
          notification.message
        )
      )
    )
  );
} 