/**
 * Centralized Loading Messages System
 * 
 * Provides rotating clever one-liner messages with emojis for loading states
 * across the Kaspa wallet application to prevent users from thinking the app is frozen.
 */

// Clever loading messages with emojis
const LOADING_MESSAGES = [
  '🦄 Fetching unicorns',
  '🧝 Helping the elves',
  '🧠 Rewiring neurons',
  '🌀 Folding space-time',
  '🧃 Juicing pixels',
  '🛸 Beaming in results',
  '🔮 Summoning clarity',
  '🧰 Tinkering with reality',
  '🧵 Threading logic',
  '🧊 Freezing bugs',
  '🐌 Encouraging faster snails',
  '🧬 Mutating variables',
  '🕳️ Patching wormholes',
  '🦠 Containing entropy',
  '🐙 Negotiating with octopi'
];

/**
 * Loading Message Manager Class
 * Handles rotating messages with automatic cleanup
 */
export class LoadingMessageManager {
  constructor() {
    this.currentMessage = '';
    this.intervalId = null;
    this.isActive = false;
  }

  /**
   * Start rotating messages
   * @param {Function} onMessageChange - Callback function called when message changes
   * @param {number} minInterval - Minimum interval in milliseconds (default: 2000)
   * @param {number} maxInterval - Maximum interval in milliseconds (default: 3000)
   */
  start(onMessageChange, minInterval = 2000, maxInterval = 3000) {
    if (this.isActive) {
      this.stop(); // Stop any existing rotation
    }

    this.isActive = true;
    
    // Set initial random message
    this.currentMessage = this.getRandomMessage();
    onMessageChange(this.currentMessage);

    // Start interval to change message every 2-3 seconds
    const rotateMessage = () => {
      if (!this.isActive) return;
      
      this.currentMessage = this.getRandomMessage();
      onMessageChange(this.currentMessage);
      
      // Schedule next rotation with random interval
      const nextInterval = minInterval + Math.random() * (maxInterval - minInterval);
      this.intervalId = setTimeout(rotateMessage, nextInterval);
    };

    // Schedule first rotation
    const firstInterval = minInterval + Math.random() * (maxInterval - minInterval);
    this.intervalId = setTimeout(rotateMessage, firstInterval);
  }

  /**
   * Stop rotating messages
   */
  stop() {
    this.isActive = false;
    if (this.intervalId) {
      clearTimeout(this.intervalId);
      this.intervalId = null;
    }
  }

  /**
   * Get a random loading message
   * @returns {string} Random loading message
   */
  getRandomMessage() {
    return LOADING_MESSAGES[Math.floor(Math.random() * LOADING_MESSAGES.length)];
  }

  /**
   * Get current message
   * @returns {string} Current message
   */
  getCurrentMessage() {
    return this.currentMessage;
  }

  /**
   * Check if manager is active
   * @returns {boolean} True if actively rotating messages
   */
  isRotating() {
    return this.isActive;
  }
}

/**
 * React Hook for Loading Messages
 * Provides easy integration with React components
 */
export function useLoadingMessages(isLoading = false, fallbackMessage = 'Loading...') {
  const { useState, useEffect, useRef } = React;
  
  const [currentMessage, setCurrentMessage] = useState(fallbackMessage);
  const managerRef = useRef(null);

  useEffect(() => {
    if (isLoading) {
      // Create and start message manager
      if (!managerRef.current) {
        managerRef.current = new LoadingMessageManager();
      }
      
      managerRef.current.start((message) => {
        setCurrentMessage(message);
      });
    } else {
      // Stop message manager and reset to fallback
      if (managerRef.current) {
        managerRef.current.stop();
      }
      setCurrentMessage(fallbackMessage);
    }

    // Cleanup on unmount
    return () => {
      if (managerRef.current) {
        managerRef.current.stop();
      }
    };
  }, [isLoading, fallbackMessage]);

  return currentMessage;
}

/**
 * Utility function to create a simple loading message manager
 * For use in non-React contexts
 * 
 * @param {Function} onMessageChange - Callback function called when message changes
 * @returns {Object} Object with start() and stop() methods
 */
export function createLoadingMessageManager(onMessageChange) {
  const manager = new LoadingMessageManager();
  
  return {
    start: () => manager.start(onMessageChange),
    stop: () => manager.stop(),
    getCurrentMessage: () => manager.getCurrentMessage(),
    isRotating: () => manager.isRotating()
  };
}

// Export the messages array for direct access if needed
export { LOADING_MESSAGES };
