/**
 * Kaspa Address Lookup Service
 * Handles .kas domain resolution to Kaspa addresses
 */

const KNS_API_BASE = 'https://api.knsdomains.org';

export class AddressLookupService {
    constructor() {
        this.cache = new Map();
        this.cacheTimeout = 5 * 60 * 1000; // 5 minutes
    }

    /**
     * Resolve .kas domain to Kaspa address
     * @param {string} domain - The .kas domain (e.g., 'kaspador.kas')
     * @param {string} network - Network type ('mainnet' or 'testnet-10')
     * @returns {Promise<{success: boolean, address?: string, error?: string}>}
     */
    async resolveDomain(domain, network = 'mainnet') {
        try {
            // Validate domain format
            if (!this.isValidKasDomain(domain)) {
                return {
                    success: false,
                    error: 'Invalid .kas domain format'
                };
            }

            // Check cache first
            const cacheKey = `${domain}_${network}`;
            const cached = this.cache.get(cacheKey);
            if (cached && (Date.now() - cached.timestamp) < this.cacheTimeout) {
                return cached.result;
            }

            // Determine API endpoint based on network
            const networkPath = network === 'mainnet' ? 'mainnet' : 'testnet';
            const apiUrl = `${KNS_API_BASE}/${networkPath}/api/v1/${domain}/owner`;

            // Try multiple CORS proxy services for reliability
            const proxyServices = [
                `https://api.allorigins.win/get?url=${encodeURIComponent(apiUrl)}`,
                `https://corsproxy.io/?${encodeURIComponent(apiUrl)}`,
                `https://cors-anywhere.herokuapp.com/${apiUrl}`
            ];

            let response = null;
            let lastError = null;

            for (const proxyUrl of proxyServices) {
                try {
                    response = await fetch(proxyUrl, {
                        method: 'GET',
                        headers: {
                            'Accept': 'application/json'
                        }
                    });

                    if (response.ok) {
                        break; // Success, exit loop
                    }
                } catch (proxyError) {
                    lastError = proxyError;
                    console.warn(`Proxy ${proxyUrl} failed:`, proxyError);
                    continue; // Try next proxy
                }
            }

            if (!response || !response.ok) {
                throw lastError || new Error('All proxy services failed');
            }

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const responseData = await response.json();

            // Handle different proxy response formats
            let data;

            // allorigins.win format
            if (responseData.contents) {
                try {
                    data = JSON.parse(responseData.contents);
                } catch (parseError) {
                    if (responseData.status && responseData.status.http_code === 404) {
                        return {
                            success: false,
                            error: 'Domain not found or not registered'
                        };
                    }
                    throw new Error('Invalid response format from KNS API');
                }
            }
            // corsproxy.io or direct response format
            else if (responseData.success !== undefined) {
                data = responseData;
            }
            // cors-anywhere format (direct passthrough)
            else {
                data = responseData;
            }

            if (!data) {
                throw new Error('No valid data received from proxy');
            }

            if (!data.success || !data.data || !data.data.owner) {
                return {
                    success: false,
                    error: 'Domain has no owner address'
                };
            }

            const ownerAddress = data.data.owner;

            // Validate the returned address
            if (!this.isValidKaspaAddress(ownerAddress, network)) {
                return {
                    success: false,
                    error: 'Invalid owner address returned'
                };
            }

            const result = {
                success: true,
                address: ownerAddress,
                domain: domain,
                asset: data.data.asset,
                assetId: data.data.assetId
            };

            // Cache the result
            this.cache.set(cacheKey, {
                result: result,
                timestamp: Date.now()
            });

            return result;

        } catch (error) {
            console.error(`❌ LOOKUP: Failed to resolve ${domain}:`, error);

            if (error.name === 'TypeError' && error.message.includes('fetch')) {
                return {
                    success: false,
                    error: 'KNS service unavailable. Please enter a valid Kaspa address.'
                };
            }

            if (error.message.includes('CORS') || error.message.includes('Access-Control')) {
                return {
                    success: false,
                    error: 'KNS service blocked by browser. Please enter a valid Kaspa address.'
                };
            }

            return {
                success: false,
                error: 'Domain lookup failed. Please enter a valid Kaspa address.'
            };
        }
    }

    /**
     * Check if input is a .kas domain
     * @param {string} input - Input string to check
     * @returns {boolean}
     */
    isValidKasDomain(input) {
        if (!input || typeof input !== 'string') {
            return false;
        }

        const trimmed = input.trim().toLowerCase();
        
        // Must end with .kas
        if (!trimmed.endsWith('.kas')) {
            return false;
        }

        // Must have at least one character before .kas
        if (trimmed.length <= 4) {
            return false;
        }

        // Extract domain name (without .kas)
        const domainName = trimmed.slice(0, -4);
        
        // Basic validation: alphanumeric and hyphens only
        const validPattern = /^[a-z0-9-]+$/;
        if (!validPattern.test(domainName)) {
            return false;
        }

        // Cannot start or end with hyphen
        if (domainName.startsWith('-') || domainName.endsWith('-')) {
            return false;
        }

        // Cannot have consecutive hyphens
        if (domainName.includes('--')) {
            return false;
        }

        return true;
    }

    /**
     * Check if address is a valid Kaspa address
     * @param {string} address - Address to validate
     * @param {string} network - Network type
     * @returns {boolean}
     */
    isValidKaspaAddress(address, network) {
        if (!address || typeof address !== 'string') {
            return false;
        }

        const trimmed = address.trim();
        
        if (network === 'mainnet') {
            return trimmed.startsWith('kaspa:') && trimmed.length > 10;
        } else {
            return trimmed.startsWith('kaspatest:') && trimmed.length > 15;
        }
    }

    /**
     * Clear lookup cache
     */
    clearCache() {
        this.cache.clear();
    }

    /**
     * Get cache statistics
     * @returns {Object}
     */
    getCacheStats() {
        const now = Date.now();
        let validEntries = 0;
        let expiredEntries = 0;

        for (const [key, value] of this.cache.entries()) {
            if ((now - value.timestamp) < this.cacheTimeout) {
                validEntries++;
            } else {
                expiredEntries++;
            }
        }

        return {
            totalEntries: this.cache.size,
            validEntries,
            expiredEntries,
            cacheTimeout: this.cacheTimeout
        };
    }
}

// Create singleton instance
export const addressLookupService = new AddressLookupService();

// Convenience function for domain resolution
export const resolveDomain = (domain, network = 'mainnet') => 
    addressLookupService.resolveDomain(domain, network);

// Convenience function to check if input is a .kas domain
export const isKasDomain = (input) => 
    addressLookupService.isValidKasDomain(input);
