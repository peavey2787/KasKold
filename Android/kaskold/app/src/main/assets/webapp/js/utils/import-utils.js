// Import Utilities for Kaspa Wallet
// Provides robust path resolution for dynamic imports that works in both local and live environments

/**
 * Get the correct base path for imports based on the current script location
 * @returns {string} The base path for imports
 */
export const getBasePath = () => {
  // Try to get the current script location from various sources
  const currentScript = 
    document.currentScript || 
    document.querySelector('script[src*="WalletApp.js"]') ||
    document.querySelector('script[src*="components/"]') ||
    document.querySelector('script[src*="js/"]');
    
  if (currentScript && currentScript.src) {
    try {
      // Get the directory of the current script
      const scriptUrl = new URL(currentScript.src);
      const pathParts = scriptUrl.pathname.split('/');
      
      // Remove the filename and navigate to root
      pathParts.pop(); // Remove the current file
      
      // If we're in a subdirectory, navigate up to root
      while (pathParts.length > 1 && (
        pathParts[pathParts.length - 1] === 'components' || 
        pathParts[pathParts.length - 1] === 'js' ||
        pathParts[pathParts.length - 1] === 'utils'
      )) {
        pathParts.pop();
      }
      
      // Reconstruct the URL with origin
      const basePath = scriptUrl.origin + pathParts.join('/');
      return basePath.endsWith('/') ? basePath : basePath + '/';
    } catch (error) {
      console.warn('Failed to parse script URL:', error);
    }
  }
  
  // Fallback: try to detect based on current location
  const currentPath = window.location.pathname;
  if (currentPath.endsWith('/') || currentPath.endsWith('.html')) {
    const basePath = currentPath.endsWith('/') ? 
      currentPath : 
      currentPath.substring(0, currentPath.lastIndexOf('/') + 1);
    return window.location.origin + basePath;
  }
  
  // Final fallback to root
  return window.location.origin + '/';
};

/**
 * Construct import paths for Kaspa modules
 * @param {string} modulePath - The module path relative to kaspa/js/
 * @returns {string} The full import path
 */
export const getKaspaImportPath = (modulePath) => {
  let basePath = getBasePath();
  
  // Ensure basePath ends with a slash
  if (!basePath.endsWith('/')) {
    basePath += '/';
  }
  
  // Remove leading slashes and construct the full path
  const cleanModulePath = modulePath.startsWith('./') ? modulePath.substring(2) : modulePath;
  const fullPath = basePath + 'kaspa/js/' + cleanModulePath;
  
  // Log for debugging in development
  if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1' || window.location.hostname.startsWith('192.168.')) {
    console.debug(`Import path resolved: ${modulePath} -> ${fullPath}`);
    console.debug(`Base path: ${basePath}`);
  }
  
  return fullPath;
};

/**
 * Import a Kaspa module with error handling and fallback
 * @param {string} modulePath - The module path relative to kaspa/js/
 * @param {Array<string>} exportNames - Array of export names to destructure
 * @returns {Promise<Object>} The imported module exports
 */
export const importKaspaModule = async (modulePath, exportNames = []) => {
  try {
    const fullPath = getKaspaImportPath(modulePath);
    const module = await import(fullPath);
    
    // If specific exports are requested, return only those
    if (exportNames.length > 0) {
      const result = {};
      for (const exportName of exportNames) {
        if (module[exportName]) {
          result[exportName] = module[exportName];
        } else {
          console.warn(`Export '${exportName}' not found in module '${modulePath}'`);
        }
      }
      return result;
    }
    
    return module;
  } catch (error) {
    // Enhanced error reporting
    console.error(`Failed to import Kaspa module '${modulePath}':`, error);
    
    if (error.message.includes('Failed to fetch dynamically imported module')) {
      const enhancedError = new Error(
        `Failed to load Kaspa module '${modulePath}'. ` +
        `This usually means the kaspa folder is missing from your web server or the file path is incorrect. ` +
        `Expected path: ${getKaspaImportPath(modulePath)}`
      );
      enhancedError.originalError = error;
      throw enhancedError;
    }
    
    throw error;
  }
};

/**
 * Check if a Kaspa module exists by attempting to fetch it
 * @param {string} modulePath - The module path relative to kaspa/js/
 * @returns {Promise<boolean>} True if the module exists and can be loaded
 */
export const checkKaspaModuleExists = async (modulePath) => {
  try {
    const fullPath = getKaspaImportPath(modulePath);
    const response = await fetch(fullPath, { method: 'HEAD' });
    return response.ok;
  } catch (error) {
    return false;
  }
};

/**
 * Get diagnostic information about the import environment
 * @returns {Object} Diagnostic information
 */
export const getImportDiagnostics = () => {
  return {
    basePath: getBasePath(),
    currentURL: window.location.href,
    currentScript: document.currentScript?.src || 'Not available',
    userAgent: navigator.userAgent,
    timestamp: new Date().toISOString()
  };
}; 