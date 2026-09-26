import Foundation
import Security

/// Persists only the authenticated ciphertext emitted by Rust `vault-runtime`.
/// No mnemonic, seed, xprv, or other wallet plaintext is accepted here.
final class VaultBlobStore {
    private let fileName = "vault-sealed-wallet-v1.bin"
    private static let currentSealedWalletLength = 216
    private static let minInventoryLength = 128
    private static let maxInventoryLength = 4096
    private static let v2SealedWalletLength = 210
    private static let legacySealedWalletLength = 96
    private static let inventoryMagic = Data([0x4b, 0x56, 0x49, 0x31]) // KVI1
    private static let currentMagic = Data([0x4b, 0x48, 0x56, 0x33]) // KHV3
    private static let v2Magic = Data([0x4b, 0x48, 0x56, 0x32]) // KHV2
    private static let legacyMagic = Data([0x4b, 0x48, 0x56, 0x31]) // KHV1

    func writeSealedWallet(_ sealedWallet: Data) throws {
        guard Self.isInventory(sealedWallet) || Self.isCurrent(sealedWallet) else {
            throw VaultStorageError.invalidSealedWallet
        }
        let url = try storageURL()
        try sealedWallet.write(to: url, options: [.atomic, .completeFileProtection])
    }

    func readSealedWallet() throws -> Data? {
        let url = try storageURL()
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        let sealedWallet = try Data(contentsOf: url, options: [.mappedIfSafe])
        guard Self.isInventory(sealedWallet) || Self.isCurrent(sealedWallet) || Self.isV2(sealedWallet) || Self.isLegacy(sealedWallet) else {
            throw VaultStorageError.invalidSealedWallet
        }
        return sealedWallet
    }

    private static func isInventory(_ value: Data) -> Bool {
        (minInventoryLength...maxInventoryLength).contains(value.count) && value.prefix(4) == inventoryMagic
    }

    private static func isCurrent(_ value: Data) -> Bool {
        value.count == currentSealedWalletLength && value.prefix(4) == currentMagic
    }

    private static func isV2(_ value: Data) -> Bool {
        value.count == v2SealedWalletLength && value.prefix(4) == v2Magic
    }

    private static func isLegacy(_ value: Data) -> Bool {
        value.count == legacySealedWalletLength && value.prefix(4) == legacyMagic
    }

    func delete() throws {
        let url = try storageURL()
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }

    private func storageURL() throws -> URL {
        let base = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let directory = base.appendingPathComponent("KasKoldVault", isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true,
            attributes: [.protectionKey: FileProtectionType.complete]
        )
        return directory.appendingPathComponent(fileName, isDirectory: false)
    }
}

/// Stores only the random 256-bit platform wrapping key. Wallet seed bytes stay
/// inside Rust. Keychain policy binds this key to the current device and makes
/// it available only while the device is unlocked.
final class VaultWrappingKeyStore {
    private let service = "com.kaskold.vault.wrapping-key"
    private let account = "platform-sealed-wallet-v1"

    func loadOrCreate() throws -> Data {
        if let existing = try load() { return existing }
        var key = Data(count: 32)
        let status = key.withUnsafeMutableBytes { bytes in
            SecRandomCopyBytes(kSecRandomDefault, bytes.count, bytes.baseAddress!)
        }
        guard status == errSecSuccess else { throw VaultStorageError.randomFailure(status) }
        try save(key)
        return key
    }

    func delete() throws {
        let status = SecItemDelete(baseQuery() as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw VaultStorageError.keychainFailure(status)
        }
    }

    private func load() throws -> Data? {
        var query = baseQuery()
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess, let data = result as? Data, data.count == 32 else {
            throw VaultStorageError.keychainFailure(status)
        }
        return data
    }

    private func save(_ key: Data) throws {
        var query = baseQuery()
        query[kSecValueData as String] = key
        query[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else { throw VaultStorageError.keychainFailure(status) }
    }

    private func baseQuery() -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }
}

enum VaultStorageError: Error {
    case invalidSealedWallet
    case randomFailure(OSStatus)
    case keychainFailure(OSStatus)
}

private extension Data.WritingOptions {
    static let completeFileProtection: Data.WritingOptions = .completeFileProtectionUnlessOpen
}
