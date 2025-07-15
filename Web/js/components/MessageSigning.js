const { useState, useRef } = React;

export function MessageSigning({ walletState, onNavigate, addNotification }) {
  const [message, setMessage] = useState('');
  const [unsignedMessageData, setUnsignedMessageData] = useState(null);
  const [signedMessageData, setSignedMessageData] = useState(null);
  const [qrCodeData, setQrCodeData] = useState(null);
  const [uploadedFile, setUploadedFile] = useState(null);
  const [uploadedQrData, setUploadedQrData] = useState(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const fileInputRef = useRef(null);
  const qrInputRef = useRef(null);

  // BigInt conversion functions
  const convertBigIntToString = (obj) => {
    if (obj === null || obj === undefined) return obj;
    if (typeof obj === 'bigint') return obj.toString();
    if (Array.isArray(obj)) return obj.map(convertBigIntToString);
    if (typeof obj === 'object') {
      const converted = {};
      for (const [key, value] of Object.entries(obj)) {
        converted[key] = convertBigIntToString(value);
      }
      return converted;
    }
    return obj;
  };

  const convertStringToBigInt = (obj, bigIntFields = ['amount', 'fee', 'value', 'satoshis']) => {
    if (obj === null || obj === undefined) return obj;
    if (Array.isArray(obj)) return obj.map(item => convertStringToBigInt(item, bigIntFields));
    if (typeof obj === 'object') {
      const converted = {};
      for (const [key, value] of Object.entries(obj)) {
        if (bigIntFields.includes(key) && typeof value === 'string' && /^\d+$/.test(value)) {
          converted[key] = BigInt(value);
        } else {
          converted[key] = convertStringToBigInt(value, bigIntFields);
        }
      }
      return converted;
    }
    return obj;
  };

  // Create unsigned message
  const handleCreateMessage = async (e) => {
    e.preventDefault();
    
    if (!message.trim()) {
      addNotification('Please enter a message to sign', 'error');
      return;
    }

    if (!walletState.currentWallet || !walletState.address) {
      addNotification('No wallet loaded', 'error');
      return;
    }

    setIsLoading(true);

    try {
      const { createUnsignedMessage } = await import('../../kaspa/js/message-signing.js');
      
      const messageData = {
        message: message.trim(),
        address: walletState.address,
        network: walletState.network,
        timestamp: new Date().toISOString()
      };

      const result = await createUnsignedMessage(messageData);

      if (result.success) {
        setUnsignedMessageData(result.messageData);
        await generateQRCode(result.messageData, 'unsigned');
        addNotification('Unsigned message created successfully', 'success');
      } else {
        addNotification('Failed to create message: ' + result.error, 'error');
      }
    } catch (error) {
      console.error('Message creation error:', error);
      addNotification('Failed to create message: ' + error.message, 'error');
    } finally {
      setIsLoading(false);
    }
  };

  // Sign message
  const handleSignMessage = async () => {
    if (!unsignedMessageData) {
      addNotification('No unsigned message to sign', 'error');
      return;
    }

    if (!walletState.currentWallet) {
      addNotification('No wallet loaded', 'error');
      return;
    }

    setIsLoading(true);

    try {
      const { signMessage } = await import('../../kaspa/js/message-signing.js');
      
      const result = await signMessage(
        unsignedMessageData.message,
        walletState.currentWallet.privateKey,
        walletState.address,
        walletState.network
      );

      if (result.success) {
        setSignedMessageData(result);
        await generateQRCode(result, 'signed');
        addNotification('Message signed successfully', 'success');
      } else {
        addNotification('Failed to sign message: ' + result.error, 'error');
      }
    } catch (error) {
      console.error('Message signing error:', error);
      addNotification('Failed to sign message: ' + error.message, 'error');
    } finally {
      setIsLoading(false);
    }
  };

  // Generate QR code
  const generateQRCode = async (messageData, type = 'unsigned') => {
    try {
      const { generateUnsignedMessageQR, generateSignedMessageQR } = await import('../../kaspa/js/qr-manager.js');
      
      let qrResult;
      if (type === 'signed') {
        qrResult = await generateSignedMessageQR(messageData);
      } else {
        qrResult = await generateUnsignedMessageQR(messageData);
      }

      if (qrResult.success) {
        setQrCodeData({ 
          qrDataURL: qrResult.qrDataURL, 
          size: qrResult.size || 0, 
          type: type 
        });
      } else {
        console.error('QR generation failed:', qrResult.error);
      }
    } catch (error) {
      console.error('QR generation error:', error);
    }
  };

  // Download QR code
  const handleDownloadQR = () => {
    if (!qrCodeData || !qrCodeData.qrDataURL) {
      addNotification('No QR code to download', 'error');
      return;
    }

    try {
      const link = document.createElement('a');
      link.href = qrCodeData.qrDataURL;
      link.download = `kaspa-${qrCodeData.type}-message-qr.png`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      addNotification('QR code downloaded successfully', 'success');
    } catch (error) {
      addNotification('Failed to download QR code: ' + error.message, 'error');
    }
  };

  // Download JSON
  const handleDownloadJSON = () => {
    const dataToDownload = signedMessageData || unsignedMessageData;
    if (!dataToDownload) {
      addNotification('No message data to download', 'error');
      return;
    }

    try {
      const convertedData = convertBigIntToString(dataToDownload);
      const jsonString = JSON.stringify(convertedData, null, 2);
      const blob = new Blob([jsonString], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      
      const link = document.createElement('a');
      link.href = url;
      const type = signedMessageData ? 'signed' : 'unsigned';
      link.download = `kaspa-${type}-message.json`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
      
      addNotification('Message data downloaded successfully', 'success');
    } catch (error) {
      addNotification('Failed to download message data: ' + error.message, 'error');
    }
  };

  // File upload
  const handleFileUpload = async (event) => {
    const file = event.target.files[0];
    if (!file) return;

    try {
      const text = await file.text();
      const data = JSON.parse(text);
      
      const messageType = detectMessageType(data);
      
      if (messageType === 'unknown') {
        throw new Error('Unrecognized message format. Please ensure this is a valid Kaspa message file.');
      }
      
      const processedData = convertStringToBigInt(data);
      
      if (messageType === 'unsigned') {
        setUnsignedMessageData(processedData);
        setSignedMessageData(null);
        await generateQRCode(processedData, 'unsigned');
      } else if (messageType === 'signed') {
        setUnsignedMessageData(null);
        setSignedMessageData(processedData);
        await generateQRCode(processedData, 'signed');
      }
      
      setUploadedFile({ file, data: processedData, type: messageType });
      setUploadedQrData(null);
      addNotification(`${messageType} message uploaded successfully`, 'success');
      
    } catch (error) {
      console.error('File upload failed:', error);
      addNotification('Failed to parse message file: ' + error.message, 'error');
    }
    
    event.target.value = '';
  };

  // QR upload
  const handleQRUpload = async (event) => {
    const files = Array.from(event.target.files);
    if (files.length === 0) return;

    try {
      const { readQRFromImage, readMultiPartQRFromImages } = await import('../../kaspa/js/qr-manager.js');
      
      let result;
      if (files.length === 1) {
        result = await readQRFromImage(files[0]);
        if (result.success) {
          result.qrData = JSON.parse(result.data);
        }
      } else {
        result = await readMultiPartQRFromImages(files);
      }
      
      if (!result.success) {
        throw new Error(result.error || 'Failed to read QR code');
      }

      const qrData = result.qrData;
      const messageType = detectMessageType(qrData);
      
      if (messageType === 'unknown') {
        throw new Error(`Unrecognized QR message format. Found type: ${qrData.type || 'undefined'}`);
      }

      const processedQrData = convertStringToBigInt(qrData);

      if (messageType === 'unsigned') {
        setUnsignedMessageData(processedQrData);
        setSignedMessageData(null);
        await generateQRCode(processedQrData, 'unsigned');
      } else if (messageType === 'signed') {
        setUnsignedMessageData(null);
        setSignedMessageData(processedQrData);
        await generateQRCode(processedQrData, 'signed');
      }

      setUploadedQrData({ 
        source: files.length === 1 ? 'single-file' : 'multiple-files', 
        data: processedQrData, 
        type: messageType,
        files: files
      });
      setUploadedFile(null);
      
      const partText = files.length > 1 ? ` from ${files.length} QR parts` : '';
      addNotification(`${messageType} message uploaded${partText}`, 'success');

    } catch (error) {
      console.error('QR upload failed:', error);
      addNotification('Failed to read QR code: ' + error.message, 'error');
    }

    event.target.value = '';
  };

  // Camera QR scan
  const handleCameraQRScan = async () => {
    setIsScanning(true);
    
    try {
      const { openCameraQRScanner } = await import('../../kaspa/js/qr-manager.js');
      
      await openCameraQRScanner(async (qrResult) => {
        try {
          if (!qrResult.success) {
            throw new Error(qrResult.error || 'Failed to scan QR code');
          }

          const qrData = qrResult.qrData;
          const messageType = detectMessageType(qrData);

          if (messageType === 'unknown') {
            throw new Error(`Unrecognized QR message format. Found type: ${qrData.type || 'undefined'}`);
          }

          const processedQrData = convertStringToBigInt(qrData);

          if (messageType === 'unsigned') {
            setUnsignedMessageData(processedQrData);
            setSignedMessageData(null);
            await generateQRCode(processedQrData, 'unsigned');
          } else if (messageType === 'signed') {
            setUnsignedMessageData(null);
            setSignedMessageData(processedQrData);
            await generateQRCode(processedQrData, 'signed');
          }

          setUploadedQrData({ 
            source: 'camera', 
            data: processedQrData, 
            type: messageType
          });
          setUploadedFile(null);
          
          addNotification(`${messageType} message scanned successfully`, 'success');

        } catch (error) {
          console.error('Camera scan processing failed:', error);
          addNotification('Failed to process scanned QR: ' + error.message, 'error');
        }
      });

    } catch (error) {
      console.error('Camera QR scan failed:', error);
      addNotification('Failed to open camera scanner: ' + error.message, 'error');
    } finally {
      setIsScanning(false);
    }
  };

  // Reset message
  const handleResetMessage = () => {
    setMessage('');
    setUnsignedMessageData(null);
    setSignedMessageData(null);
    setQrCodeData(null);
    setUploadedFile(null);
    setUploadedQrData(null);
    if (fileInputRef.current) fileInputRef.current.value = '';
    if (qrInputRef.current) qrInputRef.current.value = '';
    addNotification('Message signing reset', 'info');
  };

  // Detect message type
  const detectMessageType = (data) => {
    if (data.status) {
      if (data.status === 'unsigned') return 'unsigned';
      if (data.status === 'signed') return 'signed';
    }
    
    if (data.signature || data.signatureHex) {
      return 'signed';
    }
    
    if (data.message && (data.address || data.publicKey)) {
      return 'unsigned';
    }
    
    if (data.type) {
      if (data.type.includes('unsigned-message')) return 'unsigned';
      if (data.type.includes('signed-message')) return 'signed';
    }
    
    return 'unknown';
  };

  return React.createElement('section', { className: 'py-4' },
    React.createElement('div', { className: 'row' },
      React.createElement('div', { className: 'col-12' },
        // Header with Reset Button
        React.createElement('div', { className: 'd-flex justify-content-between align-items-center mb-4' },
          React.createElement('h4', { className: 'mb-0' },
            React.createElement('i', { className: 'bi bi-pencil-square me-2' }),
            'Message Signing'
          ),
          React.createElement('div', null,
            React.createElement('button', {
              className: 'btn btn-outline-secondary btn-sm me-2',
              onClick: handleResetMessage
            },
              React.createElement('i', { className: 'bi bi-arrow-clockwise me-1' }),
              'Reset'
            ),
            React.createElement('button', {
              className: 'btn btn-outline-primary btn-sm',
              onClick: () => onNavigate('wallet-dashboard')
            },
              React.createElement('i', { className: 'bi bi-arrow-left me-1' }),
              'Back to Dashboard'
            )
          )
        ),

        // Message Creation Card
        React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
              'Create Message'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('form', { onSubmit: handleCreateMessage },
              React.createElement('div', { className: 'mb-3' },
                React.createElement('label', { className: 'form-label' }, 'Message to Sign'),
                React.createElement('textarea', {
                  className: 'form-control',
                  rows: 4,
                  value: message,
                  onChange: (e) => setMessage(e.target.value),
                  placeholder: 'Enter the message you want to sign...',
                  required: true
                })
              ),

              React.createElement('button', {
                type: 'submit',
                className: `btn btn-primary ${isLoading ? 'disabled' : ''}`,
                disabled: isLoading
              },
                isLoading ? 
                  React.createElement('span', null,
                    React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                    'Creating...'
                  ) :
                  React.createElement('span', null,
                    React.createElement('i', { className: 'bi bi-plus-circle me-2' }),
                    'Create Unsigned Message'
                  )
              )
            )
          )
        ),

        // Upload Card
        React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h5', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-upload me-2' }),
              'Upload Message'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row g-2' },
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('label', { className: 'form-label small' }, 'Upload JSON File'),
                React.createElement('input', {
                  ref: fileInputRef,
                  type: 'file',
                  className: 'form-control form-control-sm',
                  accept: '.json',
                  onChange: handleFileUpload
                })
              ),
              React.createElement('div', { className: 'col-md-6' },
                React.createElement('label', { className: 'form-label small' }, 'Upload QR Code Image(s)'),
                React.createElement('input', {
                  ref: qrInputRef,
                  type: 'file',
                  className: 'form-control form-control-sm',
                  accept: 'image/*',
                  multiple: true,
                  onChange: handleQRUpload,
                  title: 'Select single image or multiple images for multi-part QR codes'
                })
              )
            ),
            React.createElement('div', { className: 'text-center mt-3' },
              React.createElement('button', {
                className: `btn btn-outline-info btn-sm ${isScanning ? 'disabled' : ''}`,
                onClick: handleCameraQRScan,
                disabled: isScanning
              },
                isScanning ?
                  React.createElement('span', null,
                    React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                    'Opening Camera...'
                  ) :
                  React.createElement('span', null,
                    React.createElement('i', { className: 'bi bi-camera me-2' }),
                    'Scan QR with Camera'
                  )
              )
            )
          )
        ),

        // Unsigned Message Display
        unsignedMessageData && React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header bg-warning' },
            React.createElement('h5', { className: 'card-title mb-0 text-dark' },
              React.createElement('i', { className: 'bi bi-exclamation-triangle me-2' }),
              'Step 1: Unsigned Message Created'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row align-items-center' },
              React.createElement('div', { className: 'col-md-8' },
                React.createElement('p', { className: 'mb-2' }, 
                  React.createElement('strong', null, 'Message: '), 
                  unsignedMessageData.message
                ),
                React.createElement('p', { className: 'mb-2' }, 
                  React.createElement('strong', null, 'Address: '), 
                  React.createElement('code', { className: 'text-primary' }, unsignedMessageData.address)
                ),
                React.createElement('p', { className: 'mb-0' }, 
                  React.createElement('strong', null, 'Status: '), 
                  React.createElement('span', { className: 'badge bg-warning text-dark' }, 'Unsigned')
                )
              ),
              React.createElement('div', { className: 'col-md-4 text-end' },
                !signedMessageData && React.createElement('div', { className: 'btn-group' },
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary btn-sm',
                    onClick: handleDownloadJSON
                  },
                    React.createElement('i', { className: 'bi bi-download me-1' }),
                    'JSON'
                  ),
                  qrCodeData && qrCodeData.type === 'unsigned' && React.createElement('button', {
                    className: 'btn btn-outline-secondary btn-sm',
                    onClick: handleDownloadQR
                  },
                    React.createElement('i', { className: 'bi bi-qr-code me-1' }),
                    'QR'
                  )
                )
              )
            ),
            qrCodeData && qrCodeData.type === 'unsigned' && React.createElement('div', { className: 'text-center mt-3' },
              React.createElement('img', {
                src: qrCodeData.qrDataURL,
                alt: 'Unsigned Message QR Code',
                className: 'img-fluid border rounded',
                style: { maxWidth: '300px' }
              })
            )
          )
        ),

        // Sign Message Section
        unsignedMessageData && !signedMessageData && React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header' },
            React.createElement('h6', { className: 'card-title mb-0' },
              React.createElement('i', { className: 'bi bi-pen me-2' }),
              'Sign Message'
            )
          ),
          React.createElement('div', { className: 'card-body text-center' },
            React.createElement('p', { className: 'text-muted mb-3' },
              'Message created successfully. Sign it to complete the message signing process.'
            ),
            React.createElement('button', {
              className: `btn btn-success ${isLoading ? 'disabled' : ''}`,
              onClick: handleSignMessage,
              disabled: isLoading
            },
              isLoading ? 
                React.createElement('span', null,
                  React.createElement('span', { className: 'spinner-border spinner-border-sm me-2' }),
                  'Signing Message...'
                ) :
                React.createElement('span', null,
                  React.createElement('i', { className: 'bi bi-pen me-2' }),
                  'Sign Message'
                )
            )
          )
        ),

        signedMessageData && React.createElement('div', { className: 'card mb-4' },
          React.createElement('div', { className: 'card-header bg-success' },
            React.createElement('h5', { className: 'card-title mb-0 text-white' },
              React.createElement('i', { className: 'bi bi-check-circle me-2' }),
              'Step 2: Message Signed Successfully'
            )
          ),
          React.createElement('div', { className: 'card-body' },
            React.createElement('div', { className: 'row align-items-center' },
              React.createElement('div', { className: 'col-md-8' },
                React.createElement('p', { className: 'mb-2' }, 
                  React.createElement('strong', null, 'Message: '), 
                  signedMessageData.message
                ),
                React.createElement('p', { className: 'mb-2' }, 
                  React.createElement('strong', null, 'Signature: '), 
                  React.createElement('code', { 
                    className: 'text-success text-break',
                    style: { fontSize: '0.8em' }
                  }, 
                    signedMessageData.signature || signedMessageData.signatureHex || 'Generated'
                  )
                ),
                React.createElement('p', { className: 'mb-0' }, 
                  React.createElement('strong', null, 'Status: '), 
                  React.createElement('span', { className: 'badge bg-success' }, 'Signed')
                )
              ),
              React.createElement('div', { className: 'col-md-4 text-end' },
                React.createElement('div', { className: 'btn-group' },
                  React.createElement('button', {
                    className: 'btn btn-outline-secondary btn-sm',
                    onClick: handleDownloadJSON
                  },
                    React.createElement('i', { className: 'bi bi-download me-1' }),
                    'JSON'
                  ),
                  qrCodeData && qrCodeData.type === 'signed' && React.createElement('button', {
                    className: 'btn btn-outline-secondary btn-sm',
                    onClick: handleDownloadQR
                  },
                    React.createElement('i', { className: 'bi bi-qr-code me-1' }),
                    'QR'
                  )
                )
              )
            ),
            qrCodeData && qrCodeData.type === 'signed' && React.createElement('div', { className: 'text-center mt-3' },
              React.createElement('img', {
                src: qrCodeData.qrDataURL,
                alt: 'Signed Message QR Code',
                className: 'img-fluid border rounded',
                style: { maxWidth: '300px' }
              })
            )
          )
        )
      )
    )
  );
} 