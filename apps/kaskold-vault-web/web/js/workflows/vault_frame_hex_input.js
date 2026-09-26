function hexToBytes(text) {
  const clean = String(text || '').replace(/\s+/g, '');
  if (!clean || clean.length % 2 || !/^[0-9a-fA-F]+$/.test(clean)) {
    throw new Error('Enter an even-length hexadecimal QR frame.');
  }
  const out = new Uint8Array(clean.length / 2);
  for (let index = 0; index < out.length; index += 1) {
    out[index] = Number.parseInt(clean.slice(index * 2, index * 2 + 2), 16);
  }
  return out;
}

function frameHexValues(text) {
  return String(text || '')
    .split(/[\r\n;,]+/)
    .map(value => value.replace(/\s+/g, ''))
    .filter(Boolean);
}

function parseFrameBatch(text) {
  const values = frameHexValues(text);
  if (!values.length) throw new Error('Enter an even-length hexadecimal QR frame.');
  return values.map((value, index) => {
    try {
      return hexToBytes(value);
    } catch (error) {
      if (values.length === 1) throw error;
      throw new Error(`QR frame ${index + 1}: ${error.message || error}`);
    }
  });
}

function looksLikeCompleteHexBatch(text) {
  try {
    parseFrameBatch(text);
    return true;
  } catch {
    return false;
  }
}

export function installFrameHexInput({ input, consume, reportError }) {
  const submit = () => {
    const value = input.value.trim();
    if (!value) return false;
    try {
      const frames = parseFrameBatch(value);
      for (const bytes of frames) {
        const result = consume(bytes);
        if (result !== false) break;
      }
      input.value = '';
      return true;
    } catch (error) {
      reportError(String(error));
      return false;
    }
  };

  input.addEventListener('paste', () => {
    setTimeout(() => {
      if (looksLikeCompleteHexBatch(input.value)) submit();
    }, 0);
  });
  input.addEventListener('keydown', event => {
    if (event.key !== 'Enter' || event.shiftKey) return;
    event.preventDefault();
    submit();
  });
}
