export function hasMagic(bytes, magic) {
  if (!(bytes instanceof Uint8Array) || bytes.length < magic.length) return false;
  for (let index = 0; index < magic.length; index += 1) {
    if (bytes[index] !== magic.charCodeAt(index)) return false;
  }
  return true;
}

export function bytesToText(bytes, fatal = false) {
  return new TextDecoder('utf-8', { fatal }).decode(bytes);
}

export function hexToBytes(text, label = 'hexadecimal payload') {
  const clean = String(text || '').replace(/\s+/g, '');
  if (!clean || clean.length % 2 || !/^[0-9a-fA-F]+$/.test(clean)) {
    throw new Error(`Invalid ${label}.`);
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(clean.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

export function bytesToHex(bytes) {
  let out = '';
  for (const byte of bytes) out += byte.toString(16).padStart(2, '0');
  return out;
}

export function decodeHexPayloadIfMagic(bytes, magics) {
  let text;
  try { text = bytesToText(bytes, true).trim(); } catch { return null; }
  if (!text || text.length % 2 || !/^[0-9a-fA-F]+$/.test(text)) return null;
  let decoded;
  try { decoded = hexToBytes(text); } catch { return null; }
  return magics.some(magic => hasMagic(decoded, magic)) ? decoded : null;
}

export function downloadBytes(bytes, filename, type = 'application/octet-stream') {
  const blob = new Blob([bytes], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function downloadText(text, filename) {
  downloadBytes(new TextEncoder().encode(text), filename, 'text/plain');
}

export async function readFile(input, label) {
  const file = input.files?.[0];
  if (!file) throw new Error(`Choose ${label} first.`);
  return new Uint8Array(await file.arrayBuffer());
}
export function nonNegativeInteger(value, label, max = 0x7fffffff) {
  const text = String(value ?? '').trim();
  if (!/^\d+$/.test(text)) throw new Error(`${label} must be a non-negative integer.`);
  const number = Number(text);
  if (!Number.isSafeInteger(number) || number > max) throw new Error(`${label} is out of range.`);
  return number;
}

export function requireMatchingPasswords(password, confirmation) {
  if (!password) throw new Error('Enter a backup password.');
  if (password !== confirmation) throw new Error('Backup passwords do not match.');
  return password;
}
