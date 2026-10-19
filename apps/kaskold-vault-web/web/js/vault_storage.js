const DATABASE = 'KasKoldVaultEncryptedStore';
const VERSION = 2;
const STORE = 'wallets';
const DESCRIPTOR_STORE = 'multisigDescriptors';
const ACTIVE = 'active';

function database() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE, VERSION);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE);
      if (!request.result.objectStoreNames.contains(DESCRIPTOR_STORE)) request.result.createObjectStore(DESCRIPTOR_STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error('Encrypted browser storage is unavailable.'));
  });
}

async function transaction(mode, operation, storeName = STORE) {
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(storeName, mode);
      const store = tx.objectStore(storeName);
      const request = operation(store);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error || new Error('Encrypted browser storage operation failed.'));
    });
  } finally { db.close(); }
}

function walletId() {
  const browserCrypto = globalThis.crypto;
  if (!browserCrypto?.getRandomValues) throw new Error('Secure browser randomness is unavailable.');
  if (typeof browserCrypto.randomUUID === 'function') return `wallet-${browserCrypto.randomUUID()}`;
  const bytes = new Uint8Array(16);
  browserCrypto.getRandomValues(bytes);
  return `wallet-${Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')}`;
}

function normalizedWallet(wallet, fallbackId, savedAt) {
  if (!wallet?.ciphertext) return null;
  return {
    id: String(wallet.id || fallbackId),
    name: String(wallet.name || 'Wallet'),
    ciphertext: wallet.ciphertext,
    credentialType: wallet.credentialType === 'pin' ? 'pin' : 'password',
    fingerprint: wallet.fingerprint ? String(wallet.fingerprint) : '',
    kind: wallet.kind ? String(wallet.kind) : '',
    savedAt: Number(wallet.savedAt) || savedAt,
  };
}

function normalizeRecord(record) {
  if (record?.schema !== 3 || !Array.isArray(record.wallets)) return null;
  const savedAt = Number(record.savedAt) || 0;
  const wallets = record.wallets
    .map((wallet, index) => normalizedWallet(wallet, `saved-${savedAt}-${index}`, savedAt))
    .filter(Boolean);
  return { schema: 3, wallets, savedAt };
}

function identityMatches(saved, fingerprint, kind) {
  return Boolean(fingerprint) && saved.fingerprint === fingerprint && (!kind || !saved.kind || saved.kind === kind);
}

async function writeRecord(record) {
  await transaction('readwrite', store => store.put(record, ACTIVE));
}

export async function saveEncryptedWallet(wallet) {
  if (!wallet?.ciphertext) throw new Error('No encrypted wallet is available to save.');
  const current = await loadEncryptedWallet();
  const wallets = current?.wallets ? [...current.wallets] : [];
  const fingerprint = wallet.fingerprint ? String(wallet.fingerprint) : '';
  const kind = wallet.kind ? String(wallet.kind) : '';
  const matchingIndex = wallets.findIndex(saved => identityMatches(saved, fingerprint, kind));
  const previous = matchingIndex >= 0 ? wallets[matchingIndex] : null;
  const savedAt = Date.now();
  const entry = {
    id: previous?.id || walletId(),
    name: String(wallet.name || previous?.name || 'Wallet'),
    ciphertext: new Uint8Array(wallet.ciphertext),
    credentialType: 'password',
    fingerprint,
    kind,
    savedAt,
  };
  if (matchingIndex >= 0) wallets[matchingIndex] = entry;
  else wallets.push(entry);
  await writeRecord({ schema: 3, wallets, savedAt });
  return entry.id;
}

export async function loadEncryptedWallet() {
  return normalizeRecord(await transaction('readonly', store => store.get(ACTIVE)));
}

export async function updateEncryptedWalletIdentity(id, identity) {
  const current = await loadEncryptedWallet();
  if (!current?.wallets?.length) return;
  const index = current.wallets.findIndex(wallet => wallet.id === id);
  if (index < 0) return;
  const wallet = current.wallets[index];
  current.wallets[index] = {
    ...wallet,
    name: identity?.name ? String(identity.name) : wallet.name,
    fingerprint: identity?.fingerprint ? String(identity.fingerprint) : wallet.fingerprint,
    kind: identity?.kind ? String(identity.kind) : wallet.kind,
  };
  await writeRecord({ schema: 3, wallets: current.wallets, savedAt: Date.now() });
}

// Erase every saved ciphertext for one wallet identity; returns how many were removed.
// Replace one saved wallet's ciphertext with a fresh password-sealed backup.
export async function resealEncryptedWallet(id, ciphertext) {
  const current = await loadEncryptedWallet();
  const index = current?.wallets?.findIndex(wallet => wallet.id === id) ?? -1;
  if (index < 0) throw new Error('The saved wallet is no longer available.');
  const savedAt = Date.now();
  current.wallets[index] = { ...current.wallets[index], ciphertext: new Uint8Array(ciphertext), credentialType: 'password', savedAt };
  await writeRecord({ schema: 3, wallets: current.wallets, savedAt });
}

export async function deleteSavedWalletIdentity(identity) {
  const fingerprint = identity?.fingerprint ? String(identity.fingerprint) : '';
  const kind = identity?.kind ? String(identity.kind) : '';
  const current = await loadEncryptedWallet();
  const wallets = current?.wallets || [];
  const remaining = wallets.filter(saved => !identityMatches(saved, fingerprint, kind));
  if (remaining.length === wallets.length) return 0;
  await writeRecord({ schema: 3, wallets: remaining, savedAt: Date.now() });
  return wallets.length - remaining.length;
}

export async function deleteEncryptedWallet() {
  await transaction('readwrite', store => store.delete(ACTIVE));
}

export async function saveMultisigDescriptor(ownerKpub, record) {
  if (!ownerKpub || !record?.descriptor) throw new Error('Descriptor ownership metadata is incomplete.');
  const key = `${ownerKpub}\n${record.descriptor}`;
  const value = {
    ownerKpub,
    descriptor: record.descriptor,
    address: record.address || '',
    network: record.network || 'mainnet',
    chain: Number(record.chain) || 0,
    index: Number(record.index) || 0,
    savedAt: Date.now(),
  };
  await transaction('readwrite', store => store.put(value, key), DESCRIPTOR_STORE);
}

export async function listMultisigDescriptors(ownerKpub) {
  if (!ownerKpub) return [];
  const records = await transaction('readonly', store => store.getAll(), DESCRIPTOR_STORE);
  return (records || []).filter(record => record?.ownerKpub === ownerKpub);
}

export async function deleteMultisigDescriptor(ownerKpub, descriptor) {
  if (!ownerKpub || !descriptor) return;
  const key = `${ownerKpub}\n${descriptor}`;
  await transaction('readwrite', store => store.delete(key), DESCRIPTOR_STORE);
}
