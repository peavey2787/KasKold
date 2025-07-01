/**
 * Utility functions for managing application settings
 */

/**
 * Get auto-discovery setting
 * @returns {boolean} True if auto-discovery is enabled
 */
export function getAutoDiscoveryEnabled() {
  try {
    const saved = localStorage.getItem('kaspa_auto_discovery_enabled');
    return saved !== null ? JSON.parse(saved) : true; // Default to enabled
  } catch (error) {
    console.error('Failed to load auto-discovery setting:', error);
    return true; // Default to enabled on error
  }
}

/**
 * Set auto-discovery setting
 * @param {boolean} enabled - Whether auto-discovery should be enabled
 */
export function setAutoDiscoveryEnabled(enabled) {
  try {
    localStorage.setItem('kaspa_auto_discovery_enabled', JSON.stringify(enabled));
    return true;
  } catch (error) {
    console.error('Failed to save auto-discovery setting:', error);
    return false;
  }
} 