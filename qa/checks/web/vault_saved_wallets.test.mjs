// Behavioural tests for Web Vault saved-wallet storage and the new-password policy.
import assert from 'node:assert/strict';
import { beforeEach, test } from 'node:test';

const STORES = new Map();

function request(operation) {
  const pending = { result: undefined, error: null, onsuccess: null, onerror: null };
  queueMicrotask(() => {
    try {
      pending.result = operation();
      pending.onsuccess?.();
    } catch (error) {
      pending.error = error;
      pending.onerror?.();
    }
  });
  return pending;
}

function objectStore(name) {
  const rows = STORES.get(name);
  return {
    get: key => request(() => structuredClone(rows.get(key))),
    getAll: () => request(() => [...rows.values()].map(value => structuredClone(value))),
    put: (value, key) => request(() => { rows.set(key, structuredClone(value)); return key; }),
    delete: key => request(() => { rows.delete(key); }),
  };
}

globalThis.indexedDB = {
  open: () => {
    const db = {
      objectStoreNames: { contains: name => STORES.has(name) },
      createObjectStore: name => STORES.set(name, new Map()),
      transaction: () => ({ objectStore }),
      close() {},
    };
    const pending = request(() => db);
    queueMicrotask(() => pending.onupgradeneeded?.());
    return pending;
  },
};

const { meetsPasswordPolicy, requireMatchingPasswords } = await import('../../../apps/kaskold-vault-web/web/js/vault_bytes.js');
const storage = await import('../../../apps/kaskold-vault-web/web/js/vault_storage.js');

const ACTIVE = 'active';
const walletRows = () => STORES.get('wallets');

beforeEach(() => {
  STORES.clear();
  STORES.set('wallets', new Map());
  STORES.set('multisigDescriptors', new Map());
});

test('new passwords need 12-128 characters with a letter and a digit', () => {
  assert.equal(meetsPasswordPolicy('CorrectHors9'), true);
  assert.equal(meetsPasswordPolicy('CorrectHor9'), false);
  assert.equal(meetsPasswordPolicy('correcthorsebattery'), false);
  assert.equal(meetsPasswordPolicy('123456789012'), false);
  assert.equal(meetsPasswordPolicy(`${'a'.repeat(127)}7`), true);
  assert.equal(meetsPasswordPolicy(`${'a'.repeat(128)}7`), false);
  assert.equal(requireMatchingPasswords('CorrectHorse9', 'CorrectHorse9'), 'CorrectHorse9');
  assert.throws(() => requireMatchingPasswords('', ''), /Enter a password/);
  assert.throws(() => requireMatchingPasswords('CorrectHorse9', 'CorrectHorse8'), /do not match/);
  assert.throws(() => requireMatchingPasswords('short9', 'short9'), /12-128 characters/);
});

test('saving replaces the entry for the same wallet identity', async () => {
  const first = await storage.saveEncryptedWallet({ name: 'A', ciphertext: [1], fingerprint: 'f1', kind: 'mnemonic' });
  const other = await storage.saveEncryptedWallet({ name: 'B', ciphertext: [2], fingerprint: 'f2', kind: 'mnemonic' });
  const replaced = await storage.saveEncryptedWallet({ name: 'A2', ciphertext: [3], fingerprint: 'f1', kind: 'mnemonic' });
  assert.equal(replaced, first);
  const { wallets } = await storage.loadEncryptedWallet();
  assert.deepEqual(wallets.map(wallet => [wallet.id, wallet.name, [...wallet.ciphertext], wallet.credentialType]),
    [[first, 'A2', [3], 'password'], [other, 'B', [2], 'password']]);
});

test('deleting a wallet erases only that identity from browser storage', async () => {
  await storage.saveEncryptedWallet({ name: 'A', ciphertext: [1], fingerprint: 'f1', kind: 'mnemonic' });
  await storage.saveEncryptedWallet({ name: 'B', ciphertext: [2], fingerprint: 'f2', kind: 'mnemonic' });
  assert.equal(await storage.deleteSavedWalletIdentity({ fingerprint: 'f1', kind: 'xprv' }), 0);
  assert.equal(await storage.deleteSavedWalletIdentity({ fingerprint: '', kind: '' }), 0);
  assert.equal(await storage.deleteSavedWalletIdentity({ fingerprint: 'f1', kind: 'mnemonic' }), 1);
  const { wallets } = await storage.loadEncryptedWallet();
  assert.deepEqual(wallets.map(wallet => wallet.fingerprint), ['f2']);
  await storage.deleteEncryptedWallet();
  assert.equal(await storage.loadEncryptedWallet(), null);
});

test('records from retired storage schemas are not read', async () => {
  await storage.saveEncryptedWallet({ name: 'A', ciphertext: [1], fingerprint: 'f1' });
  for (const legacy of [
    { schema: 1, ciphertext: [9], walletName: 'Old', savedAt: 1 },
    { schema: 2, wallets: [{ ciphertext: [9] }], savedAt: 1 },
  ]) {
    walletRows().set(ACTIVE, legacy);
    assert.equal(await storage.loadEncryptedWallet(), null);
  }
});

test('resealing replaces a legacy PIN ciphertext with a password-sealed one', async () => {
  walletRows().set(ACTIVE, {
    schema: 3,
    savedAt: 5,
    wallets: [{ id: 'w1', name: 'Old', ciphertext: [7], credentialType: 'pin', fingerprint: 'f1', kind: 'mnemonic' }],
  });
  await storage.resealEncryptedWallet('w1', [8, 9]);
  const [wallet] = (await storage.loadEncryptedWallet()).wallets;
  assert.deepEqual([wallet.id, wallet.name, [...wallet.ciphertext], wallet.credentialType], ['w1', 'Old', [8, 9], 'password']);
  await assert.rejects(storage.resealEncryptedWallet('missing', [1]), /no longer available/);
});
