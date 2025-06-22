// Kaspa QR Code Manager Module
import { getKaspa, isInitialized } from './init.js';

// QR Code generation using qrcode-generator library (will be loaded dynamically)
let qrCodeLib = null;

// QR Code reading using jsQR library (will be loaded dynamically)
let jsQRLib = null;

// Loading states to prevent race conditions
let qrLibraryLoading = false;
let qrLibraryLoaded = false;

// Load QR code libraries dynamically
async function loadQRLibraries() {

    
    // If already loaded, return immediately
    if (qrLibraryLoaded && qrCodeLib && jsQRLib) {

        return;
    }
    
    // If currently loading, wait for it to complete
    if (qrLibraryLoading) {
        // Wait for loading to complete with timeout
        let attempts = 0;
        while (qrLibraryLoading && attempts < 50) { // 5 second timeout
            await new Promise(resolve => setTimeout(resolve, 100));
            attempts++;
        }
        
        if (qrLibraryLoaded && qrCodeLib && jsQRLib) {
            return;
        } else {
            throw new Error('QR libraries failed to load within timeout');
        }
    }
    
    // Start loading
    qrLibraryLoading = true;
    
    try {
        if (!qrCodeLib) {
            // Try to load qrcode-generator library (more reliable)
            const qrScript = document.createElement('script');
            qrScript.src = './libs/qrcode-generator.min.js';
            qrScript.crossOrigin = 'anonymous';
            document.head.appendChild(qrScript);
            
            await new Promise((resolve, reject) => {
                const timeout = setTimeout(() => {
                    reject(new Error('QR generation library loading timeout'));
                }, 10000); // 10 second timeout
                
                qrScript.onload = () => {
                    clearTimeout(timeout);
                    
                    // qrcode-generator creates a global qrcode function
                    qrCodeLib = window.qrcode;
                    if (!qrCodeLib) {
                        reject(new Error('QR generation library not available after loading'));
                    } else {
                        resolve();
                    }
                };
                qrScript.onerror = (error) => {
                    clearTimeout(timeout);
                    reject(new Error('Failed to load QR generation library'));
                };
            });
        }
        
        if (!jsQRLib) {
            // Load QR code reading library
            const jsQRScript = document.createElement('script');
            jsQRScript.src = './libs/jsQR.js';
            jsQRScript.crossOrigin = 'anonymous';
            document.head.appendChild(jsQRScript);
            
            await new Promise((resolve, reject) => {
                const timeout = setTimeout(() => {
                    reject(new Error('QR reading library loading timeout'));
                }, 10000); // 10 second timeout
                
                jsQRScript.onload = () => {
                    clearTimeout(timeout);
                    jsQRLib = window.jsQR;
                    if (!jsQRLib) {
                        reject(new Error('QR reading library not available after loading'));
                    } else {
                        resolve();
                    }
                };
                jsQRScript.onerror = (error) => {
                    clearTimeout(timeout);
                    reject(new Error('Failed to load QR reading library'));
                };
            });
        } 
        
        // Mark as loaded
        qrLibraryLoaded = true;
        qrLibraryLoading = false;
        
    } catch (error) {
        qrLibraryLoading = false;
        throw error;
    }
}

/**
 * Generate QR code using qrcode-generator library
 * @param {string} text - Text to encode
 * @returns {string} - Base64 data URL of the QR code
 */
function generateQRCodeDataURL(text) {
    
    if (!qrCodeLib) {
        throw new Error('QR code library not loaded');
    }
    
    try {
        // Create QR code using qrcode-generator library
        const qr = qrCodeLib(0, 'H'); // Type 0 (auto), High error correction
        qr.addData(text);
        qr.make();
        
        // Generate as data URL
        const cellSize = 8; // Size of each QR module in pixels
        const margin = 4;   // Margin around QR code
        
        // Create canvas to draw QR code
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        
        const moduleCount = qr.getModuleCount();
        const canvasSize = (moduleCount * cellSize) + (margin * 2);
        
        canvas.width = canvasSize;
        canvas.height = canvasSize;
        
        // Fill background (white)
        ctx.fillStyle = '#FFFFFF';
        ctx.fillRect(0, 0, canvasSize, canvasSize);
        
        // Draw QR modules (black)
        ctx.fillStyle = '#000000';
        for (let row = 0; row < moduleCount; row++) {
            for (let col = 0; col < moduleCount; col++) {
                if (qr.isDark(row, col)) {
                    const x = margin + (col * cellSize);
                    const y = margin + (row * cellSize);
                    ctx.fillRect(x, y, cellSize, cellSize);
                }
            }
        }
        
        // Convert canvas to data URL
        const dataURL = canvas.toDataURL('image/png');
        
        return dataURL;
        
    } catch (error) {
        throw error;
    }
}

/**
 * Generate QR code for unsigned message data
 * @param {Object} messageData - Unsigned message data
 * @returns {Promise<Object>} - QR generation result with data URL
 */
async function generateUnsignedMessageQR(messageData) {
    
    try {
        await loadQRLibraries();
        
        if (!messageData.message) {
            throw new Error('Message is required for QR generation');
        }
        
        // Create QR data structure for unsigned message
        const qrData = {
            type: 'kaspa-unsigned-message-qr',
            version: '1.0',
            message: messageData.message,
            signerAddress: messageData.signerAddress,
            networkType: messageData.networkType,
            timestamp: messageData.timestamp,
            messageId: messageData.messageId || `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
        };   
        
        // Convert to JSON string
        const qrString = JSON.stringify(qrData);
        
        if (!qrCodeLib) {
            throw new Error('QR code generation library not loaded');
        }
        console.log('QR DEBUG: Generating QR code with qrcode-generator library...');
        // Generate QR code as data URL using our custom function
        const qrDataURL = generateQRCodeDataURL(qrString);
                 
        return {
            success: true,
            qrDataURL: qrDataURL,
            qrData: qrData,
            qrString: qrString,
            size: qrString.length
        };
        
    } catch (error) {
        return {
            success: false,
            error: error.message || 'Failed to generate QR code'
        };
    }
}

/**
 * Generate QR code for signed message data
 * @param {Object} signedData - Signed message data
 * @returns {Promise<Object>} - QR generation result with data URL
 */
async function generateSignedMessageQR(signedData) {
    
    try {
        await loadQRLibraries();
        
        if (!signedData.message || !signedData.signature) {
            throw new Error('Message and signature are required for QR generation');
        }
         console.log('QR DEBUG: Creating QR data structure...');
        // Create QR data structure for signed message
        const qrData = {
            type: 'kaspa-signed-message-qr',
            version: '1.0',
            message: signedData.message,
            signature: signedData.signature,
            signerAddress: signedData.signerAddress,
            signerPublicKey: signedData.signerPublicKey,
            networkType: signedData.networkType,
            timestamp: signedData.timestamp,
            signingId: signedData.signingId
        };
        
       
        // Convert to JSON string
        const qrString = JSON.stringify(qrData);
       
        if (!qrCodeLib) {
            throw new Error('QR code generation library not loaded');
        }
        
         // Generate QR code as data URL using our custom function
        const qrDataURL = generateQRCodeDataURL(qrString);
       
        return {
            success: true,
            qrDataURL: qrDataURL,
            qrData: qrData,
            qrString: qrString,
            size: qrString.length
        };
        
    } catch (error) {
        
        return {
            success: false,
            error: error.message || 'Failed to generate QR code'
        };
    }
}

/**
 * Read QR code from image file
 * @param {File} imageFile - Image file containing QR code
 * @returns {Promise<Object>} - QR reading result with parsed data
 */
async function readQRFromImage(imageFile) {
    try {
        await loadQRLibraries();
        
        if (!imageFile || !imageFile.type.startsWith('image/')) {
            throw new Error('Please select a valid image file');
        }
        
        // Create canvas to process the image
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        
        // Create image element
        const img = new Image();
        
        return new Promise((resolve, reject) => {
            img.onload = () => {
                try {
                    // Set canvas size to image size
                    canvas.width = img.width;
                    canvas.height = img.height;
                    
                    // Draw image on canvas
                    ctx.drawImage(img, 0, 0);
                    
                    // Get image data
                    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
                    
                    // Read QR code
                    const qrResult = jsQRLib(imageData.data, imageData.width, imageData.height);
                    
                    if (!qrResult) {
                        throw new Error('No QR code found in the image');
                    }
                    
                    // Parse QR data
                    let qrData;
                    try {
                        qrData = JSON.parse(qrResult.data);
                    } catch (parseError) {
                        throw new Error('QR code does not contain valid JSON data');
                    }
                    
                    // Validate QR data structure
                    if (!qrData.type || !qrData.type.startsWith('kaspa-')) {
                        throw new Error('QR code is not a valid Kaspa QR code');
                    }
                    
                    resolve({
                        success: true,
                        qrData: qrData,
                        data: qrResult.data, // Keep the raw data for backward compatibility
                        rawData: qrResult.data,
                        location: qrResult.location
                    });
                    
                } catch (error) {
                    reject(error);
                }
            };
            
            img.onerror = () => {
                reject(new Error('Failed to load image file'));
            };
            
            // Convert file to data URL and load
            const reader = new FileReader();
            reader.onload = (e) => {
                img.src = e.target.result;
            };
            reader.onerror = () => {
                reject(new Error('Failed to read image file'));
            };
            reader.readAsDataURL(imageFile);
        });
        
    } catch (error) {
        console.error('Error reading QR from image:', error);
        return {
            success: false,
            error: error.message || 'Failed to read QR code'
        };
    }
}

/**
 * Download QR code as PNG image
 * @param {string} qrDataURL - QR code data URL
 * @param {string} filename - Filename for download
 */
function downloadQRImage(qrDataURL, filename) {
    try {
        const link = document.createElement('a');
        link.href = qrDataURL;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
    } catch (error) {
        console.error('Error downloading QR image:', error);
        throw new Error('Failed to download QR image');
    }
}

/**
 * Validate QR data structure
 * @param {Object} qrData - QR data to validate
 * @param {string} expectedType - Expected QR type
 * @returns {Object} - Validation result
 */
function validateQRData(qrData, expectedType) {
    try {
        if (!qrData || typeof qrData !== 'object') {
            throw new Error('Invalid QR data structure');
        }
        
        const fullExpectedType = `kaspa-${expectedType}-qr`;
        if (qrData.type !== fullExpectedType) {
            throw new Error(`Invalid QR type - expected ${fullExpectedType}, got ${qrData.type}`);
        }
        
        if (!qrData.version) {
            throw new Error('Missing QR version');
        }
        
        if (!qrData.message) {
            throw new Error('Missing message in QR data');
        }
        
        // Type-specific validations
        switch (expectedType) {
            case 'unsigned-message':
                if (!qrData.signerAddress) {
                    throw new Error('Missing signer address in unsigned message QR');
                }
                break;
                
            case 'signed-message':
                if (!qrData.signature) {
                    throw new Error('Missing signature in signed message QR');
                }
                if (!qrData.signerAddress) {
                    throw new Error('Missing signer address in signed message QR');
                }
                break;
        }
        
        return {
            isValid: true,
            data: qrData
        };
        
    } catch (error) {
        return {
            isValid: false,
            error: error.message
        };
    }
}

/**
 * Create QR display element
 * @param {string} qrDataURL - QR code data URL
 * @param {string} title - Title for the QR display
 * @returns {HTMLElement} - QR display container
 */
function createQRDisplay(qrDataURL, title) {
    const container = document.createElement('div');
    container.style.cssText = `
        text-align: center;
        padding: 20px;
        background: white;
        border: 2px solid #ddd;
        border-radius: 8px;
        margin: 10px 0;
    `;
    
    const titleElement = document.createElement('h4');
    titleElement.textContent = title;
    titleElement.style.cssText = `
        margin: 0 0 15px 0;
        color: #333;
    `;
    
    const qrImage = document.createElement('img');
    qrImage.src = qrDataURL;
    qrImage.alt = title;
    qrImage.style.cssText = `
        max-width: 100%;
        height: auto;
        border: 1px solid #eee;
    `;
    
    container.appendChild(titleElement);
    container.appendChild(qrImage);
    
    return container;
}

// Initialize QR manager
async function initializeQRManager() {
    try {
        await loadQRLibraries();
        return true;
    } catch (error) {
        console.error('QR DEBUG: Failed to initialize QR Manager:', error);
        return false;
    }
}

// Test function for manual debugging
async function testQRGeneration() {
    try {
        await loadQRLibraries();
        
        const testData = "Hello World Test";
        
        if (!qrCodeLib) {
            throw new Error('QR library not available');
        }
        
        const result = generateQRCodeDataURL(testData);
          return result;
    } catch (error) {
        throw error;
    }
}

// Make test function available globally for debugging
window.testQRGeneration = testQRGeneration;

/**
 * Generate QR code for unsigned transaction data
 * @param {Object} transactionData - Unsigned transaction data
 * @returns {Promise<Object>} - QR generation result with data URL
 */
async function generateUnsignedTransactionQR(transactionData) {
    
    try {
        
        await loadQRLibraries();
        
        if (!transactionData.transactionId) {
            throw new Error('Transaction ID is required for QR generation');
        }
        
        // Create QR data structure for unsigned transaction
        const qrData = {
            type: 'kaspa-unsigned-transaction-qr',
            version: '1.0',
            transactionId: transactionData.transactionId,
            fromAddress: transactionData.fromAddress,
            toAddress: transactionData.toAddress,
            amount: transactionData.amount,
            amountInSompi: transactionData.amountInSompi,
            fee: transactionData.fee,
            feeMode: transactionData.feeMode,
            networkType: transactionData.networkType,
            timestamp: transactionData.timestamp || new Date().toISOString(),
            status: 'unsigned'
        };
        
        // Convert to JSON string
        const qrString = JSON.stringify(qrData);
       
        if (!qrCodeLib) {
            throw new Error('QR code generation library not loaded');
        }
        
        // Generate QR code as data URL using our custom function
        const qrDataURL = generateQRCodeDataURL(qrString);
                
        return {
            success: true,
            qrDataURL: qrDataURL,
            qrData: qrData,
            qrString: qrString,
            size: qrString.length
        };
        
    } catch (error) {
        console.error('QR DEBUG: Error in generateUnsignedTransactionQR:', error);
        return {
            success: false,
            error: error.message || 'Failed to generate QR code'
        };
    }
}

/**
 * Generate QR code for signed transaction data
 * @param {Object} signedTransactionData - Signed transaction data
 * @returns {Promise<Object>} - QR generation result with data URL
 */
async function generateSignedTransactionQR(signedTransactionData) {
    
    try {
        await loadQRLibraries();
        
        if (!signedTransactionData.transactionId) {
            throw new Error('Transaction ID is required for QR generation');
        }
        
        // Create QR data structure for signed transaction
        const qrData = {
            type: 'kaspa-signed-transaction-qr',
            version: '1.0',
            transactionId: signedTransactionData.transactionId,
            fromAddress: signedTransactionData.fromAddress,
            toAddress: signedTransactionData.toAddress,
            amount: signedTransactionData.amount,
            amountInSompi: signedTransactionData.amountInSompi,
            fee: signedTransactionData.fee,
            feeMode: signedTransactionData.feeMode,
            networkType: signedTransactionData.networkType,
            timestamp: signedTransactionData.timestamp || new Date().toISOString(),
            status: 'signed',
            // Include serialized transaction data if available
            serializedTransaction: signedTransactionData.serializedTransaction || null
        };
        
        // Convert to JSON string
        const qrString = JSON.stringify(qrData);
        
        // Use multi-part QR generation for potentially large signed transaction data
        const multiQRResult = await generateMultiPartQR(qrData, 'signed-transaction');
        
        if (!multiQRResult.success) {
            throw new Error(multiQRResult.error);
        }        
        return multiQRResult;
        
    } catch (error) {
        console.error('QR DEBUG: Error in generateSignedTransactionQR:', error);
        return {
            success: false,
            error: error.message || 'Failed to generate QR code'
        };
    }
}

/**
 * Generate QR code for submitted transaction data
 * @param {Object} submittedTransactionData - Submitted transaction data
 * @returns {Promise<Object>} - QR generation result with data URL
 */
async function generateSubmittedTransactionQR(submittedTransactionData) {
    
    try {
        await loadQRLibraries();
        
        if (!submittedTransactionData.transactionId) {
            throw new Error('Transaction ID is required for QR generation');
        }
        
        // Create QR data structure for submitted transaction
        const qrData = {
            type: 'kaspa-submitted-transaction-qr',
            version: '1.0',
            transactionId: submittedTransactionData.transactionId,
            fromAddress: submittedTransactionData.fromAddress,
            toAddress: submittedTransactionData.toAddress,
            amount: submittedTransactionData.amount,
            amountInSompi: submittedTransactionData.amountInSompi,
            fee: submittedTransactionData.fee,
            feeMode: submittedTransactionData.feeMode,
            networkType: submittedTransactionData.networkType,
            timestamp: submittedTransactionData.timestamp || new Date().toISOString(),
            status: 'submitted',
            networkResponse: submittedTransactionData.networkResponse || null,
            submissionTimestamp: submittedTransactionData.submissionTimestamp || new Date().toISOString()
        };
        
        // Convert to JSON string
        const qrString = JSON.stringify(qrData);
        if (!qrCodeLib) {
            throw new Error('QR code generation library not loaded');
        }
        // Generate QR code as data URL using our custom function
        const qrDataURL = generateQRCodeDataURL(qrString);
        
        return {
            success: true,
            qrDataURL: qrDataURL,
            qrData: qrData,
            qrString: qrString,
            size: qrString.length
        };
        
    } catch (error) {
        console.error('QR DEBUG: Error in generateSubmittedTransactionQR:', error);
        return {
            success: false,
            error: error.message || 'Failed to generate QR code'
        };
    }
}

/**
 * Validate transaction QR data structure
 * @param {Object} qrData - QR data to validate
 * @param {string} expectedType - Expected QR type (unsigned-transaction, signed-transaction, submitted-transaction)
 * @returns {Object} - Validation result
 */
function validateTransactionQRData(qrData, expectedType) {
    try {
        if (!qrData || typeof qrData !== 'object') {
            throw new Error('Invalid QR data structure');
        }
        
        const fullExpectedType = `kaspa-${expectedType}-qr`;
        if (qrData.type !== fullExpectedType) {
            throw new Error(`Invalid QR type - expected ${fullExpectedType}, got ${qrData.type}`);
        }
        
        if (!qrData.version) {
            throw new Error('Missing QR version');
        }
        
        if (!qrData.transactionId) {
            throw new Error('Missing transaction ID in QR data');
        }
        
        if (!qrData.fromAddress || !qrData.toAddress) {
            throw new Error('Missing transaction addresses in QR data');
        }
        
        if (!qrData.amount) {
            throw new Error('Missing transaction amount in QR data');
        }
        
        // Type-specific validations
        switch (expectedType) {
            case 'unsigned-transaction':
                if (qrData.status !== 'unsigned') {
                    throw new Error('Invalid status for unsigned transaction QR');
                }
                break;
                
            case 'signed-transaction':
                if (qrData.status !== 'signed') {
                    throw new Error('Invalid status for signed transaction QR');
                }
                break;
                
            case 'submitted-transaction':
                if (qrData.status !== 'submitted') {
                    throw new Error('Invalid status for submitted transaction QR');
                }
                break;
        }
        
        return {
            isValid: true,
            data: qrData
        };
        
    } catch (error) {
        return {
            isValid: false,
            error: error.message
        };
    }
}

/**
 * Split large data into multiple parts for QR codes
 * @param {string} jsonString - JSON string to split
 * @param {number} maxChunkSize - Maximum size per chunk (default: 1000 chars)
 * @returns {Array} - Array of data chunks with metadata
 */
function splitDataForMultiQR(jsonString, maxChunkSize = 1000) {
    const chunks = [];
    const totalParts = Math.ceil(jsonString.length / maxChunkSize);
    const multiQRId = `mqr_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    for (let i = 0; i < totalParts; i++) {
        const start = i * maxChunkSize;
        const end = Math.min(start + maxChunkSize, jsonString.length);
        const chunk = jsonString.substring(start, end);
        
        chunks.push({
            multiQRId: multiQRId,
            part: i + 1,
            totalParts: totalParts,
            data: chunk,
            checksum: generateSimpleChecksum(jsonString) // For validation
        });
    }
    
    return chunks;
}

/**
 * Generate a simple checksum for data validation
 * @param {string} data - Data to checksum
 * @returns {string} - Simple checksum
 */
function generateSimpleChecksum(data) {
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
        const char = data.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(36);
}

/**
 * Recombine multi-part QR data
 * @param {Array} parts - Array of QR part data
 * @returns {Object} - Recombined data result
 */
function recombineMultiQRData(parts) {
    try {
        if (!parts || parts.length === 0) {
            throw new Error('No parts provided');
        }
        
        // Validate all parts have same multiQRId and totalParts
        const firstPart = parts[0];
        const multiQRId = firstPart.multiQRId;
        const totalParts = firstPart.totalParts;
        
        if (parts.length !== totalParts) {
            throw new Error(`Missing parts: expected ${totalParts}, got ${parts.length}`);
        }
        
        // Validate all parts belong to same multi-QR
        for (const part of parts) {
            if (part.multiQRId !== multiQRId) {
                throw new Error('Parts belong to different multi-QR sets');
            }
            if (part.totalParts !== totalParts) {
                throw new Error('Inconsistent total parts count');
            }
        }
        
        // Sort parts by part number
        parts.sort((a, b) => a.part - b.part);
        
        // Validate we have all parts in sequence
        for (let i = 0; i < parts.length; i++) {
            if (parts[i].part !== i + 1) {
                throw new Error(`Missing part ${i + 1}`);
            }
        }
        
        // Recombine data
        const combinedData = parts.map(part => part.data).join('');
        
        // Validate checksum
        const expectedChecksum = firstPart.checksum;
        const actualChecksum = generateSimpleChecksum(combinedData);
        
        if (expectedChecksum !== actualChecksum) {
            throw new Error('Data corruption detected - checksum mismatch');
        }
        
        return {
            success: true,
            data: combinedData,
            multiQRId: multiQRId,
            totalParts: totalParts
        };
        
    } catch (error) {
        return {
            success: false,
            error: error.message
        };
    }
}

/**
 * Generate multiple QR codes for large data
 * @param {Object} qrData - QR data object
 * @param {string} baseType - Base type for the QR (e.g., 'signed-transaction')
 * @returns {Promise<Object>} - Multi-QR generation result
 */
async function generateMultiPartQR(qrData, baseType) {
    
    try {
        await loadQRLibraries();
        
        // Convert to JSON string
        const jsonString = JSON.stringify(qrData);
        
        // Check if we need multi-part QR (threshold: 1200 characters to be safe)
        const maxSingleQRSize = 1200;
        
        if (jsonString.length <= maxSingleQRSize) {
            // Single QR is sufficient
            const qrDataURL = generateQRCodeDataURL(jsonString);
            
            return {
                success: true,
                isMultiPart: false,
                qrDataURL: qrDataURL,
                qrData: qrData,
                qrString: jsonString,
                size: jsonString.length,
                totalParts: 1
            };
        }
        
        // Multi-part QR needed
        const chunks = splitDataForMultiQR(jsonString, 800); // Smaller chunks for QR reliability
        const qrParts = [];
        
        for (const chunk of chunks) {
            // Create multi-part QR data structure
            const multiPartQRData = {
                type: `kaspa-${baseType}-multipart-qr`,
                version: '1.0',
                multiQRId: chunk.multiQRId,
                part: chunk.part,
                totalParts: chunk.totalParts,
                data: chunk.data,
                checksum: chunk.checksum,
                timestamp: new Date().toISOString()
            };
            
            const partJsonString = JSON.stringify(multiPartQRData);
            const partQRDataURL = generateQRCodeDataURL(partJsonString);
            
            qrParts.push({
                part: chunk.part,
                totalParts: chunk.totalParts,
                qrDataURL: partQRDataURL,
                qrData: multiPartQRData,
                qrString: partJsonString,
                size: partJsonString.length
            });
        }
        
        return {
            success: true,
            isMultiPart: true,
            qrParts: qrParts,
            totalParts: chunks.length,
            originalData: qrData,
            originalSize: jsonString.length,
            multiQRId: chunks[0].multiQRId
        };
        
    } catch (error) {
        console.error('QR DEBUG: Error in generateMultiPartQR:', error);
        return {
            success: false,
            error: error.message || 'Failed to generate multi-part QR'
        };
    }
}

/**
 * Create display for multi-part QR codes
 * @param {Object} multiQRResult - Multi-part QR generation result
 * @param {string} title - Display title
 * @returns {HTMLElement} - Display element
 */
function createMultiPartQRDisplay(multiQRResult, title) {
    const container = document.createElement('div');
    container.style.cssText = 'border: 2px solid #007bff; border-radius: 8px; padding: 15px; background: #f8f9fa;';
    
    // Generate unique IDs to avoid conflicts
    const uniqueId = `mqr_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const partNumId = `currentPartNum_${uniqueId}`;
    const partSizeId = `currentPartSize_${uniqueId}`;
    const qrContainerId = `currentQRContainer_${uniqueId}`;
    const prevBtnId = `prevPartBtn_${uniqueId}`;
    const nextBtnId = `nextPartBtn_${uniqueId}`;
    
    // Header
    const header = document.createElement('div');
    header.style.cssText = 'margin-bottom: 15px; text-align: center;';
    header.innerHTML = `
        <h4 style="margin: 0 0 5px 0; color: #007bff;">${title}</h4>
        <p style="margin: 0; font-size: 0.9em; color: #666;">
            <strong>Multi-Part QR:</strong> ${multiQRResult.totalParts} parts | 
            <strong>Total Size:</strong> ${multiQRResult.originalSize} chars
        </p>
    `;
    container.appendChild(header);
    
    // Current part display
    const currentPartDiv = document.createElement('div');
    currentPartDiv.style.cssText = 'text-align: center; margin-bottom: 15px;';
    currentPartDiv.innerHTML = `
        <div style="font-weight: bold; margin-bottom: 10px;">
            Part <span id="${partNumId}">1</span> of ${multiQRResult.totalParts}
        </div>
        <div id="${qrContainerId}" style="margin-bottom: 10px;"></div>
        <div style="font-size: 0.85em; color: #666;">
            Part Size: <span id="${partSizeId}">${multiQRResult.qrParts[0].size}</span> characters
        </div>
    `;
    container.appendChild(currentPartDiv);
    
    // Initialize the first QR display
    const qrContainer = container.querySelector(`#${qrContainerId}`);
    const img = document.createElement('img');
    img.src = multiQRResult.qrParts[0].qrDataURL;
    img.style.cssText = 'max-width: 300px; height: auto; border: 1px solid #ddd;';
    img.alt = `QR Code Part 1`;
    qrContainer.appendChild(img);
    
    // Navigation controls (only if more than 1 part)
    if (multiQRResult.totalParts > 1) {
        const navDiv = document.createElement('div');
        navDiv.style.cssText = 'text-align: center; margin-bottom: 15px;';
        navDiv.innerHTML = `
            <button id="${prevBtnId}" style="margin: 0 10px; padding: 5px 15px;" disabled>← Previous</button>
            <button id="${nextBtnId}" style="margin: 0 10px; padding: 5px 15px;">Next →</button>
        `;
        container.appendChild(navDiv);
        
        // Add navigation functionality after elements are in DOM
        setTimeout(() => {
            let currentPartIndex = 0;
            
            const updateDisplay = () => {
                const currentPart = multiQRResult.qrParts[currentPartIndex];
                const partNumSpan = container.querySelector(`#${partNumId}`);
                const partSizeSpan = container.querySelector(`#${partSizeId}`);
                const qrContainerDiv = container.querySelector(`#${qrContainerId}`);
                const prevBtn = container.querySelector(`#${prevBtnId}`);
                const nextBtn = container.querySelector(`#${nextBtnId}`);
                
                if (partNumSpan) partNumSpan.textContent = currentPart.part;
                if (partSizeSpan) partSizeSpan.textContent = currentPart.size;
                
                if (qrContainerDiv) {
                    qrContainerDiv.innerHTML = '';
                    const img = document.createElement('img');
                    img.src = currentPart.qrDataURL;
                    img.style.cssText = 'max-width: 300px; height: auto; border: 1px solid #ddd;';
                    img.alt = `QR Code Part ${currentPart.part}`;
                    qrContainerDiv.appendChild(img);
                }
                
                if (prevBtn) prevBtn.disabled = currentPartIndex === 0;
                if (nextBtn) nextBtn.disabled = currentPartIndex === multiQRResult.totalParts - 1;
            };
            
            const prevBtn = container.querySelector(`#${prevBtnId}`);
            const nextBtn = container.querySelector(`#${nextBtnId}`);
            
            if (prevBtn) {
                prevBtn.onclick = () => {
                    if (currentPartIndex > 0) {
                        currentPartIndex--;
                        updateDisplay();
                    }
                };
            }
            
            if (nextBtn) {
                nextBtn.onclick = () => {
                    if (currentPartIndex < multiQRResult.totalParts - 1) {
                        currentPartIndex++;
                        updateDisplay();
                    }
                };
            }
        }, 10); // Small delay to ensure DOM is ready
    }
    
    return container;
}

/**
 * Read and recombine multi-part QR codes from multiple images
 * @param {FileList} imageFiles - Array of image files containing QR parts
 * @returns {Promise<Object>} - Combined QR reading result
 */
async function readMultiPartQRFromImages(imageFiles) {
    
    try {
        const qrParts = [];
        
        // Read each QR image
        for (let i = 0; i < imageFiles.length; i++) {
            const file = imageFiles[i];
            
            const qrResult = await readQRFromImage(file);
            if (!qrResult.success) {
                throw new Error(`Failed to read QR from ${file.name}: ${qrResult.error}`);
            }
            
            // Parse QR data
            let qrData;
            if (qrResult.qrData && typeof qrResult.qrData === 'object') {
                qrData = qrResult.qrData;
            } else if (qrResult.data && typeof qrResult.data === 'string') {
                qrData = JSON.parse(qrResult.data);
            } else {
                throw new Error(`Invalid QR data format in ${file.name}`);
            }
            
            // Check if this is a multi-part QR
            if (!qrData.type || !qrData.type.includes('-multipart-qr')) {
                throw new Error(`File ${file.name} does not contain a multi-part QR code`);
            }
            
            // Validate multi-part structure
            if (!qrData.multiQRId || !qrData.part || !qrData.totalParts || !qrData.data || !qrData.checksum) {
                throw new Error(`Invalid multi-part QR structure in ${file.name}`);
            }
            
            qrParts.push(qrData);
        }
                
        // Recombine the parts
        const recombineResult = recombineMultiQRData(qrParts);
        
        if (!recombineResult.success) {
            throw new Error(`Failed to recombine QR parts: ${recombineResult.error}`);
        }

        // Parse the recombined data
        const combinedQRData = JSON.parse(recombineResult.data);
        
        return {
            success: true,
            qrData: combinedQRData,
            data: recombineResult.data,
            multiQRId: recombineResult.multiQRId,
            totalParts: recombineResult.totalParts,
            isMultiPart: true
        };
        
    } catch (error) {
        console.error('QR DEBUG: Error in readMultiPartQRFromImages:', error);
        return {
            success: false,
            error: error.message || 'Failed to read multi-part QR codes'
        };
    }
}

/**
 * Camera QR Scanner Class
 */
class CameraQRScanner {
    constructor() {
        this.stream = null;
        this.video = null;
        this.canvas = null;
        this.context = null;
        this.scanning = false;
        this.scannedParts = [];
        this.onScanCallback = null;
        this.scanInterval = null;
    }

    /**
     * Initialize camera access and create scanner UI
     */
    async initializeCamera() {
        try {
            // Check if we're in a secure context (HTTPS or localhost)
            if (!window.isSecureContext) {
                return {
                    success: false,
                    error: 'Camera access requires HTTPS. Please access this page over HTTPS or use localhost for development.'
                };
            }

            // Check if mediaDevices is available
            if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
                return {
                    success: false,
                    error: 'Camera API not available. Please use a modern browser with camera support.'
                };
            }

            // Check if camera permissions are available
            try {
                const permissions = await navigator.permissions.query({ name: 'camera' });
                
                if (permissions.state === 'denied') {
                    return {
                        success: false,
                        error: 'Camera access denied. Please enable camera permissions in your browser settings and refresh the page.'
                    };
                }
            } catch (permError) {
                console.log('Permission query not supported, proceeding with camera request');
            }

            // Request camera permission with fallback options
            const constraints = {
                video: {
                    facingMode: 'environment', // Prefer back camera
                    width: { ideal: 640, min: 320 },
                    height: { ideal: 480, min: 240 }
                }
            };

            try {
                this.stream = await navigator.mediaDevices.getUserMedia(constraints);
            } catch (envError) {
                // Fallback to any available camera
                const fallbackConstraints = {
                    video: {
                        width: { ideal: 640, min: 320 },
                        height: { ideal: 480, min: 240 }
                    }
                };
                this.stream = await navigator.mediaDevices.getUserMedia(fallbackConstraints);
            }
            
            return { success: true };
        } catch (error) {            
            let errorMessage = 'Camera access failed. ';
            
            if (error.name === 'NotAllowedError') {
                errorMessage += 'Please allow camera access when prompted and try again.';
            } else if (error.name === 'NotFoundError') {
                errorMessage += 'No camera found on this device.';
            } else if (error.name === 'NotSupportedError') {
                errorMessage += 'Camera not supported on this device.';
            } else if (error.name === 'NotReadableError') {
                errorMessage += 'Camera is being used by another application.';
            } else if (error.name === 'SecurityError') {
                errorMessage += 'Camera access blocked due to security restrictions. Please use HTTPS.';
            } else {
                errorMessage += `Error: ${error.message}`;
            }
            
            return { 
                success: false, 
                error: errorMessage
            };
        }
    }

    /**
     * Create scanner UI modal
     */
    createScannerUI(onScanCallback) {
        this.onScanCallback = onScanCallback;
        
        // Create modal overlay
        const modal = document.createElement('div');
        modal.id = 'qr-scanner-modal';
        modal.style.cssText = `
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.8);
            z-index: 10000;
            display: flex;
            justify-content: center;
            align-items: center;
        `;

        // Create scanner container
        const container = document.createElement('div');
        container.style.cssText = `
            background: white;
            border-radius: 10px;
            padding: 20px;
            max-width: 90vw;
            max-height: 90vh;
            overflow: auto;
        `;

        // Create header
        const header = document.createElement('div');
        header.style.cssText = 'text-align: center; margin-bottom: 15px;';
        header.innerHTML = `
            <h3 style="margin: 0 0 10px 0;">📱 QR Code Scanner</h3>
            <p style="margin: 0; color: #666;">Position QR code in camera view</p>
        `;

        // Create video element
        this.video = document.createElement('video');
        this.video.style.cssText = `
            width: 100%;
            max-width: 400px;
            height: auto;
            border: 2px solid #007bff;
            border-radius: 8px;
            margin-bottom: 15px;
        `;
        this.video.autoplay = true;
        this.video.playsInline = true;

        // Create canvas for QR processing (hidden)
        this.canvas = document.createElement('canvas');
        this.canvas.style.display = 'none';
        this.context = this.canvas.getContext('2d');

        // Create scan status
        const scanStatus = document.createElement('div');
        scanStatus.id = 'scan-status';
        scanStatus.style.cssText = `
            text-align: center;
            margin: 10px 0;
            padding: 10px;
            border-radius: 5px;
            font-weight: bold;
        `;
        scanStatus.innerHTML = '🔍 Scanning for QR codes...';

        // Create scanned parts display
        const scannedPartsDiv = document.createElement('div');
        scannedPartsDiv.id = 'scanned-parts';
        scannedPartsDiv.style.cssText = `
            margin: 15px 0;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 5px;
            min-height: 50px;
        `;
        scannedPartsDiv.innerHTML = '<strong>Scanned Parts:</strong><br><em>No parts scanned yet</em>';

        // Create control buttons
        const buttonContainer = document.createElement('div');
        buttonContainer.style.cssText = 'text-align: center; margin-top: 15px;';
        
        const scanButton = document.createElement('button');
        scanButton.textContent = '📷 Start Scanning';
        scanButton.style.cssText = `
            background: #28a745;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            margin: 0 10px;
            cursor: pointer;
            font-weight: bold;
        `;
        
        const clearButton = document.createElement('button');
        clearButton.textContent = '🗑️ Clear Parts';
        clearButton.style.cssText = `
            background: #ffc107;
            color: #212529;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            margin: 0 10px;
            cursor: pointer;
            font-weight: bold;
        `;
        
        const useButton = document.createElement('button');
        useButton.textContent = '✅ Use Scanned Data';
        useButton.style.cssText = `
            background: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            margin: 0 10px;
            cursor: pointer;
            font-weight: bold;
        `;
        useButton.disabled = true;
        
        const closeButton = document.createElement('button');
        closeButton.textContent = '❌ Close';
        closeButton.style.cssText = `
            background: #dc3545;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            margin: 0 10px;
            cursor: pointer;
            font-weight: bold;
        `;

        // Add event listeners
        scanButton.onclick = () => this.toggleScanning(scanButton);
        clearButton.onclick = () => this.clearScannedParts();
        useButton.onclick = () => this.useScannedData(modal);
        closeButton.onclick = () => this.closeScanner(modal);

        // Assemble UI
        buttonContainer.appendChild(scanButton);
        buttonContainer.appendChild(clearButton);
        buttonContainer.appendChild(useButton);
        buttonContainer.appendChild(closeButton);

        container.appendChild(header);
        container.appendChild(this.video);
        container.appendChild(this.canvas);
        container.appendChild(scanStatus);
        container.appendChild(scannedPartsDiv);
        container.appendChild(buttonContainer);

        modal.appendChild(container);
        document.body.appendChild(modal);

        // Store references
        this.scanButton = scanButton;
        this.useButton = useButton;

        return modal;
    }

    /**
     * Start video stream
     */
    async startVideo() {
        if (this.stream && this.video) {
            this.video.srcObject = this.stream;
            await this.video.play();
            
            // Set canvas size to match video
            this.canvas.width = this.video.videoWidth || 640;
            this.canvas.height = this.video.videoHeight || 480;
        }
    }

    /**
     * Toggle scanning on/off
     */
    toggleScanning(button) {
        if (this.scanning) {
            this.stopScanning();
            button.textContent = '📷 Start Scanning';
            button.style.background = '#28a745';
            this.updateScanStatus('🔍 Scanning stopped. Click "Start Scanning" to resume.');
        } else {
            this.startScanning();
            button.textContent = '⏸️ Stop Scanning';
            button.style.background = '#ffc107';
            this.updateScanStatus('🔍 Scanning for QR codes...');
        }
    }

    /**
     * Start QR code scanning
     */
    startScanning() {
        this.scanning = true;
        this.scanInterval = setInterval(() => {
            this.scanFrame();
        }, 500); // Scan every 500ms
    }

    /**
     * Stop QR code scanning
     */
    stopScanning() {
        this.scanning = false;
        if (this.scanInterval) {
            clearInterval(this.scanInterval);
            this.scanInterval = null;
        }
    }

    /**
     * Scan current video frame for QR codes
     */
    scanFrame() {
        if (!this.video || !this.canvas || !this.context) return;

        try {
            // Draw current video frame to canvas
            this.context.drawImage(this.video, 0, 0, this.canvas.width, this.canvas.height);
            
            // Get image data
            const imageData = this.context.getImageData(0, 0, this.canvas.width, this.canvas.height);
            
            // Scan for QR code
            const qrResult = jsQR(imageData.data, imageData.width, imageData.height);
            
            if (qrResult) {
                this.handleQRDetected(qrResult.data);
            }
        } catch (error) {
            console.error('QR scan error:', error);
        }
    }

    /**
     * Handle detected QR code
     */
    handleQRDetected(qrData) {
        try {
            const parsedData = JSON.parse(qrData);
            
            // Check if it's a multi-part QR
            if (parsedData.type && parsedData.type.includes('-multipart-qr')) {
                this.handleMultiPartQR(parsedData);
            } else {
                this.handleSingleQR(parsedData);
            }
        } catch (error) {
            console.error('Error parsing QR data:', error);
            this.updateScanStatus('❌ Invalid QR code format detected');
        }
    }

    /**
     * Handle single QR code
     */
    handleSingleQR(qrData) {
        this.scannedParts = [{ type: 'single', data: qrData }];
        this.updateScanStatus('✅ Single QR code scanned successfully!');
        this.updateScannedPartsDisplay();
        this.useButton.disabled = false;
        this.stopScanning();
        this.scanButton.textContent = '📷 Start Scanning';
        this.scanButton.style.background = '#28a745';
    }

    /**
     * Handle multi-part QR code
     */
    handleMultiPartQR(qrData) {
        const { multiQRId, part, totalParts } = qrData;
        
        // Check if we already have this part
        const existingPart = this.scannedParts.find(p => 
            p.type === 'multipart' && 
            p.multiQRId === multiQRId && 
            p.part === part
        );
        
        if (existingPart) {
            this.updateScanStatus(`⚠️ Part ${part} already scanned`);
            return;
        }
        
        // Add new part
        this.scannedParts.push({
            type: 'multipart',
            multiQRId,
            part,
            totalParts,
            data: qrData
        });
        
        // Sort parts by part number
        this.scannedParts.sort((a, b) => a.part - b.part);
        
        const currentParts = this.scannedParts.filter(p => p.multiQRId === multiQRId).length;
        
        if (currentParts === totalParts) {
            this.updateScanStatus(`✅ All ${totalParts} parts scanned! Ready to use.`);
            this.useButton.disabled = false;
            this.stopScanning();
            this.scanButton.textContent = '📷 Start Scanning';
            this.scanButton.style.background = '#28a745';
        } else {
            this.updateScanStatus(`📱 Part ${part}/${totalParts} scanned. Need ${totalParts - currentParts} more parts.`);
        }
        
        this.updateScannedPartsDisplay();
    }

    /**
     * Update scan status message
     */
    updateScanStatus(message) {
        const statusDiv = document.getElementById('scan-status');
        if (statusDiv) {
            statusDiv.textContent = message;
            
            // Color coding
            if (message.includes('✅')) {
                statusDiv.style.background = '#d4edda';
                statusDiv.style.color = '#155724';
            } else if (message.includes('❌') || message.includes('⚠️')) {
                statusDiv.style.background = '#f8d7da';
                statusDiv.style.color = '#721c24';
            } else {
                statusDiv.style.background = '#d1ecf1';
                statusDiv.style.color = '#0c5460';
            }
        }
    }

    /**
     * Update scanned parts display
     */
    updateScannedPartsDisplay() {
        const partsDiv = document.getElementById('scanned-parts');
        if (!partsDiv) return;

        if (this.scannedParts.length === 0) {
            partsDiv.innerHTML = '<strong>Scanned Parts:</strong><br><em>No parts scanned yet</em>';
            return;
        }

        let html = '<strong>Scanned Parts:</strong><br>';
        
        if (this.scannedParts[0].type === 'single') {
            html += `<div style="margin: 5px 0; padding: 5px; background: #e8f5e8; border-radius: 3px;">
                ✅ Single QR Code (${this.scannedParts[0].data.type})
            </div>`;
        } else {
            // Group by multiQRId
            const groups = {};
            this.scannedParts.forEach(part => {
                if (!groups[part.multiQRId]) {
                    groups[part.multiQRId] = [];
                }
                groups[part.multiQRId].push(part);
            });

            Object.entries(groups).forEach(([multiQRId, parts]) => {
                const totalParts = parts[0].totalParts;
                const scannedCount = parts.length;
                
                html += `<div style="margin: 5px 0; padding: 5px; background: #e8f4f8; border-radius: 3px;">
                    📱 Multi-part QR: ${scannedCount}/${totalParts} parts
                    <div style="font-size: 0.9em; margin-top: 3px;">
                        Parts: ${parts.map(p => p.part).join(', ')}
                    </div>
                </div>`;
            });
        }

        partsDiv.innerHTML = html;
    }

    /**
     * Clear all scanned parts
     */
    clearScannedParts() {
        this.scannedParts = [];
        this.updateScannedPartsDisplay();
        this.updateScanStatus('🗑️ Cleared all scanned parts');
        this.useButton.disabled = true;
    }

    /**
     * Use scanned data
     */
    async useScannedData(modal) {
        try {
            let finalData;

            if (this.scannedParts[0].type === 'single') {
                finalData = {
                    success: true,
                    qrData: this.scannedParts[0].data,
                    data: JSON.stringify(this.scannedParts[0].data),
                    isMultiPart: false
                };
            } else {
                // Recombine multi-part data
                const parts = this.scannedParts.map(p => p.data);
                const recombineResult = recombineMultiQRData(parts);
                
                if (!recombineResult.success) {
                    throw new Error(recombineResult.error);
                }

                finalData = {
                    success: true,
                    qrData: JSON.parse(recombineResult.data),
                    data: recombineResult.data,
                    isMultiPart: true,
                    totalParts: parts.length
                };
            }

            // Call the callback with the scanned data
            if (this.onScanCallback) {
                await this.onScanCallback(finalData);
            }

            this.closeScanner(modal);
        } catch (error) {
            console.error('Error using scanned data:', error);
            this.updateScanStatus(`❌ Error: ${error.message}`);
        }
    }

    /**
     * Close scanner and cleanup
     */
    closeScanner(modal) {
        this.stopScanning();
        
        // Stop video stream
        if (this.stream) {
            this.stream.getTracks().forEach(track => track.stop());
            this.stream = null;
        }

        // Remove modal
        if (modal && modal.parentNode) {
            modal.parentNode.removeChild(modal);
        }

        // Reset state
        this.video = null;
        this.canvas = null;
        this.context = null;
        this.scannedParts = [];
        this.onScanCallback = null;
    }
}

/**
 * Global camera scanner instance
 */
let cameraScanner = null;

/**
 * Open camera QR scanner
 * @param {Function} onScanCallback - Callback function when QR is scanned
 * @returns {Promise<void>}
 */
async function openCameraQRScanner(onScanCallback) {
    try {
        // Check if jsQR is available
        if (typeof jsQR === 'undefined') {
            throw new Error('QR reading library not loaded. Please refresh the page and try again.');
        }

        // Show initial loading message
        const loadingAlert = showCameraLoadingMessage();

        // Create new scanner instance
        cameraScanner = new CameraQRScanner();
        
        // Initialize camera
        const cameraResult = await cameraScanner.initializeCamera();
        
        // Hide loading message
        if (loadingAlert && loadingAlert.parentNode) {
            loadingAlert.parentNode.removeChild(loadingAlert);
        }
        
        if (!cameraResult.success) {
            // Show helpful error message with solutions
            showCameraErrorDialog(cameraResult.error);
            return;
        }

        // Create and show scanner UI
        const modal = cameraScanner.createScannerUI(onScanCallback);
        
        // Start video
        await cameraScanner.startVideo();
                
    } catch (error) {
        console.error('Error opening camera scanner:', error);
        showCameraErrorDialog(`Camera scanner error: ${error.message}`);
        
        if (cameraScanner) {
            cameraScanner.closeScanner();
            cameraScanner = null;
        }
    }
}

/**
 * Show loading message while requesting camera access
 */
function showCameraLoadingMessage() {
    const overlay = document.createElement('div');
    overlay.style.cssText = `
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.7);
        z-index: 9999;
        display: flex;
        justify-content: center;
        align-items: center;
        color: white;
        font-size: 1.2em;
        text-align: center;
    `;
    
    overlay.innerHTML = `
        <div style="background: rgba(0, 0, 0, 0.8); padding: 20px; border-radius: 10px;">
            <div style="margin-bottom: 15px;">📷 Requesting Camera Access...</div>
            <div style="font-size: 0.9em; color: #ccc;">Please allow camera permissions when prompted</div>
        </div>
    `;
    
    document.body.appendChild(overlay);
    return overlay;
}

/**
 * Show detailed error dialog with solutions
 */
function showCameraErrorDialog(errorMessage) {
    const isHttps = window.location.protocol === 'https:';
    const isLocalhost = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';
    
    let solutions = '';
    
    if (!isHttps && !isLocalhost) {
        solutions += `
            <div style="margin: 10px 0; padding: 10px; background: #fff3cd; border-radius: 5px;">
                <strong>🔒 HTTPS Required:</strong><br>
                Camera access requires a secure connection. Try:
                <ul style="margin: 5px 0; padding-left: 20px;">
                    <li>Access via HTTPS (https://your-domain.com)</li>
                    <li>Use localhost for development</li>
                    <li>Use a service like ngrok for HTTPS tunneling</li>
                </ul>
            </div>
        `;
    }
    
    solutions += `
        <div style="margin: 10px 0; padding: 10px; background: #d1ecf1; border-radius: 5px;">
            <strong>📱 Alternative Options:</strong><br>
            <ul style="margin: 5px 0; padding-left: 20px;">
                <li>Use the "📤 Upload QR" button to select QR images from your gallery</li>
                <li>Take a screenshot of the QR code and upload it</li>
                <li>Use the multi-part QR upload feature for large QR codes</li>
            </ul>
        </div>
    `;
    
    const modal = document.createElement('div');
    modal.style.cssText = `
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.8);
        z-index: 10000;
        display: flex;
        justify-content: center;
        align-items: center;
        padding: 20px;
        box-sizing: border-box;
    `;
    
    const container = document.createElement('div');
    container.style.cssText = `
        background: white;
        border-radius: 10px;
        padding: 20px;
        max-width: 500px;
        max-height: 80vh;
        overflow: auto;
    `;
    
    container.innerHTML = `
        <h3 style="margin: 0 0 15px 0; color: #dc3545;">📷 Camera Scanner Error</h3>
        <div style="margin-bottom: 15px; padding: 10px; background: #f8d7da; border-radius: 5px; color: #721c24;">
            ${errorMessage}
        </div>
        ${solutions}
        <div style="text-align: center; margin-top: 20px;">
            <button onclick="this.closest('[style*=fixed]').remove()" style="
                background: #007bff;
                color: white;
                border: none;
                padding: 10px 20px;
                border-radius: 5px;
                cursor: pointer;
                font-weight: bold;
            ">OK, I'll try the alternatives</button>
        </div>
    `;
    
    modal.appendChild(container);
    document.body.appendChild(modal);
}

/**
 * Generate QR code for any text/data
 * @param {string} text - Text to encode in QR code
 * @param {Object} options - QR generation options (currently unused but for future compatibility)
 * @returns {Promise<Object>} - QR generation result
 */
async function generateQRCode(text, options = {}) {
    console.log('QR DEBUG: generateQRCode called with text:', text);
    
    try {
        // Ensure QR libraries are loaded
        await loadQRLibraries();
        
        // Generate QR code data URL
        const qrDataURL = generateQRCodeDataURL(text);
        
        return {
            success: true,
            qrDataURL: qrDataURL,
            text: text
        };
        
    } catch (error) {
        console.error('QR DEBUG: Error in generateQRCode:', error);
        return {
            success: false,
            error: error.message
        };
    }
}

export {
    generateQRCode,
    generateUnsignedMessageQR,
    generateSignedMessageQR,
    readQRFromImage,
    downloadQRImage,
    validateQRData,
    createQRDisplay,
    initializeQRManager,
    testQRGeneration,
    generateUnsignedTransactionQR,
    generateSignedTransactionQR,
    generateSubmittedTransactionQR,
    validateTransactionQRData,
    splitDataForMultiQR,
    generateSimpleChecksum,
    recombineMultiQRData,
    generateMultiPartQR,
    createMultiPartQRDisplay,
    readMultiPartQRFromImages,
    openCameraQRScanner
}; 