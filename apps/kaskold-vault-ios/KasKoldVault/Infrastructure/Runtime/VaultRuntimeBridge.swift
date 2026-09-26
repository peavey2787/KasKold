import Foundation

@_silgen_name("kaskold_vault_new") private func kkVaultNew() -> UnsafeMutableRawPointer?
@_silgen_name("kaskold_vault_destroy") private func kkVaultDestroy(_ handle: UnsafeMutableRawPointer?)
@_silgen_name("kaskold_vault_restore") private func kkVaultRestore(_ handle: UnsafeMutableRawPointer?, _ phrase: UnsafePointer<UInt8>?, _ phraseCount: Int, _ passphrase: UnsafePointer<UInt8>?, _ passphraseCount: Int) -> Int32
@_silgen_name("kaskold_vault_lock") private func kkVaultLock(_ handle: UnsafeMutableRawPointer?)
@_silgen_name("kaskold_vault_export_public_account") private func kkVaultExportPublic(_ handle: UnsafeMutableRawPointer?) -> Int32
@_silgen_name("kaskold_vault_backup_words") private func kkVaultBackupWords(_ handle: UnsafeMutableRawPointer?) -> Int32
@_silgen_name("kaskold_vault_backup_seedqr") private func kkVaultBackupSeedQR(_ handle: UnsafeMutableRawPointer?, _ compact: UInt8) -> Int32
@_silgen_name("kaskold_vault_backup_xprv") private func kkVaultBackupXPrv(_ handle: UnsafeMutableRawPointer?) -> Int32
@_silgen_name("kaskold_vault_export_receive_key") private func kkVaultExportReceiveKey(_ handle: UnsafeMutableRawPointer?, _ index: UInt32) -> Int32
@_silgen_name("kaskold_vault_workflow_text") private func kkVaultWorkflowText(_ handle: UnsafeMutableRawPointer?, _ operation: UnsafePointer<UInt8>?, _ operationCount: Int, _ input: UnsafePointer<UInt8>?, _ inputCount: Int) -> Int32
@_silgen_name("kaskold_vault_workflow_bytes") private func kkVaultWorkflowBytes(_ handle: UnsafeMutableRawPointer?, _ operation: UnsafePointer<UInt8>?, _ operationCount: Int, _ input: UnsafePointer<UInt8>?, _ inputCount: Int, _ data: UnsafePointer<UInt8>?, _ dataCount: Int) -> Int32
@_silgen_name("kaskold_vault_workflow_text_with_bytes") private func kkVaultWorkflowTextWithBytes(_ handle: UnsafeMutableRawPointer?, _ operation: UnsafePointer<UInt8>?, _ operationCount: Int, _ input: UnsafePointer<UInt8>?, _ inputCount: Int, _ data: UnsafePointer<UInt8>?, _ dataCount: Int) -> Int32
@_silgen_name("kaskold_vault_seal") private func kkVaultSeal(_ handle: UnsafeMutableRawPointer?, _ key: UnsafePointer<UInt8>?, _ keyCount: Int) -> Int32
@_silgen_name("kaskold_vault_unlock_sealed") private func kkVaultUnlockSealed(_ handle: UnsafeMutableRawPointer?, _ sealed: UnsafePointer<UInt8>?, _ sealedCount: Int, _ key: UnsafePointer<UInt8>?, _ keyCount: Int) -> Int32
@_silgen_name("kaskold_vault_begin_scan") private func kkVaultBeginScan(_ handle: UnsafeMutableRawPointer?) -> Int32
@_silgen_name("kaskold_vault_accept_frame") private func kkVaultAcceptFrame(_ handle: UnsafeMutableRawPointer?, _ frame: UnsafePointer<UInt8>?, _ frameCount: Int) -> Int32
@_silgen_name("kaskold_vault_approve") private func kkVaultApprove(_ handle: UnsafeMutableRawPointer?, _ nowUnix: UInt64) -> Int32
@_silgen_name("kaskold_vault_reject") private func kkVaultReject(_ handle: UnsafeMutableRawPointer?)
@_silgen_name("kaskold_vault_response_count") private func kkVaultResponseCount(_ handle: UnsafeMutableRawPointer?) -> Int
@_silgen_name("kaskold_vault_response_frame_copy") private func kkVaultResponseFrameCopy(_ handle: UnsafeMutableRawPointer?, _ index: Int, _ output: UnsafeMutablePointer<UInt8>?, _ capacity: Int) -> Int
@_silgen_name("kaskold_vault_last_text_copy") private func kkVaultLastTextCopy(_ handle: UnsafeMutableRawPointer?, _ output: UnsafeMutablePointer<UInt8>?, _ capacity: Int) -> Int
@_silgen_name("kaskold_vault_last_bytes_copy") private func kkVaultLastBytesCopy(_ handle: UnsafeMutableRawPointer?, _ output: UnsafeMutablePointer<UInt8>?, _ capacity: Int) -> Int
@_silgen_name("kaskold_vault_last_error_copy") private func kkVaultLastErrorCopy(_ handle: UnsafeMutableRawPointer?, _ output: UnsafeMutablePointer<UInt8>?, _ capacity: Int) -> Int

struct VaultReviewInput: Equatable {
    let index: Int
    let outpoint: String
    let amount: String
    let scriptType: String
    let address: String?
}

struct VaultReviewOutput: Equatable {
    let index: Int
    let amount: String
    let ownership: String
    let address: String?
}

struct VaultReview: Equatable {
    let network: String
    let inputCount: Int
    let outputCount: Int
    let inputTotal: String
    let outputTotal: String
    let fee: String
    let inputs: [VaultReviewInput]
    let outputs: [VaultReviewOutput]
}

enum VaultBridgeError: LocalizedError {
    case runtimeUnavailable
    case native(String)
    case malformedResponse

    var errorDescription: String? {
        switch self {
        case .runtimeUnavailable: "KasKold Vault Rust runtime is unavailable."
        case .native(let message): message
        case .malformedResponse: "KasKold Vault returned a malformed native response."
        }
    }
}

final class VaultRuntimeBridge {
    private var handle: UnsafeMutableRawPointer?

    init() throws {
        handle = kkVaultNew()
        guard handle != nil else { throw VaultBridgeError.runtimeUnavailable }
    }

    deinit {
        if let handle { kkVaultDestroy(handle) }
        handle = nil
    }


    func restoreWallet(phrase: String, passphrase: String) throws -> String {
        var phraseData = Data(phrase.utf8)
        var passphraseData = Data(passphrase.utf8)
        defer { phraseData.secureErase(); passphraseData.secureErase() }
        let status = phraseData.withUnsafeBytes { phraseBytes in
            passphraseData.withUnsafeBytes { passphraseBytes in
                kkVaultRestore(
                    requiredHandleUnchecked(),
                    phraseBytes.bindMemory(to: UInt8.self).baseAddress,
                    phraseData.count,
                    passphraseBytes.bindMemory(to: UInt8.self).baseAddress,
                    passphraseData.count
                )
            }
        }
        try check(status)
        guard let kpub = try jsonObject()["kpub"] as? String else { throw VaultBridgeError.malformedResponse }
        return kpub
    }

    func sealWallet(wrappingKey: inout Data) throws -> Data {
        let status = wrappingKey.withUnsafeBytes { bytes in
            kkVaultSeal(requiredHandleUnchecked(), bytes.bindMemory(to: UInt8.self).baseAddress, wrappingKey.count)
        }
        try check(status)
        return try lastBytes()
    }

    func unlockSealedWallet(_ sealed: inout Data, wrappingKey: inout Data) throws -> String? {
        let status = sealed.withUnsafeBytes { sealedBytes in
            wrappingKey.withUnsafeBytes { keyBytes in
                kkVaultUnlockSealed(
                    requiredHandleUnchecked(),
                    sealedBytes.bindMemory(to: UInt8.self).baseAddress,
                    sealed.count,
                    keyBytes.bindMemory(to: UInt8.self).baseAddress,
                    wrappingKey.count
                )
            }
        }
        try check(status)
        let object = try jsonObject()
        if object["kpub"] is NSNull { return nil }
        return object["kpub"] as? String
    }

    func exportPublicAccount() throws -> String {
        try check(kkVaultExportPublic(requiredHandle()))
        return try lastText()
    }

    func backupWords() throws -> String {
        try check(kkVaultBackupWords(requiredHandle()))
        return try lastText()
    }

    func backupSeedQR(compact: Bool = false) throws -> Data {
        try check(kkVaultBackupSeedQR(requiredHandle(), compact ? 1 : 0))
        return try lastBytes()
    }

    func backupXPrv() throws -> String {
        try check(kkVaultBackupXPrv(requiredHandle()))
        return try lastText()
    }

    func exportReceiveKey(index: Int) throws -> String {
        guard (0...65535).contains(index) else { throw VaultBridgeError.malformedResponse }
        try check(kkVaultExportReceiveKey(requiredHandle(), UInt32(index)))
        return try lastText()
    }

    func workflowText(_ operation: String, input: [String: Any] = [:]) throws -> [String: Any] {
        var operationData = Data(operation.utf8)
        var inputData = try JSONSerialization.data(withJSONObject: input)
        defer { operationData.secureErase(); inputData.secureErase() }
        let status = operationData.withUnsafeBytes { operationBytes in
            inputData.withUnsafeBytes { inputBytes in
                kkVaultWorkflowText(
                    requiredHandleUnchecked(),
                    operationBytes.bindMemory(to: UInt8.self).baseAddress, operationData.count,
                    inputBytes.bindMemory(to: UInt8.self).baseAddress, inputData.count
                )
            }
        }
        try check(status)
        return try jsonObject()
    }

    func workflowTextWithBytes(_ operation: String, input: [String: Any] = [:], data: inout Data) throws -> [String: Any] {
        var operationData = Data(operation.utf8)
        var inputData = try JSONSerialization.data(withJSONObject: input)
        defer { operationData.secureErase(); inputData.secureErase() }
        let status = operationData.withUnsafeBytes { operationBytes in
            inputData.withUnsafeBytes { inputBytes in
                data.withUnsafeBytes { dataBytes in
                    kkVaultWorkflowTextWithBytes(
                        requiredHandleUnchecked(),
                        operationBytes.bindMemory(to: UInt8.self).baseAddress, operationData.count,
                        inputBytes.bindMemory(to: UInt8.self).baseAddress, inputData.count,
                        dataBytes.bindMemory(to: UInt8.self).baseAddress, data.count
                    )
                }
            }
        }
        try check(status)
        return try jsonObject()
    }

    func workflowBytes(_ operation: String, input: [String: Any] = [:], data: inout Data) throws -> Data {
        var operationData = Data(operation.utf8)
        var inputData = try JSONSerialization.data(withJSONObject: input)
        defer { operationData.secureErase(); inputData.secureErase() }
        let status = operationData.withUnsafeBytes { operationBytes in
            inputData.withUnsafeBytes { inputBytes in
                data.withUnsafeBytes { dataBytes in
                    kkVaultWorkflowBytes(
                        requiredHandleUnchecked(),
                        operationBytes.bindMemory(to: UInt8.self).baseAddress, operationData.count,
                        inputBytes.bindMemory(to: UInt8.self).baseAddress, inputData.count,
                        dataBytes.bindMemory(to: UInt8.self).baseAddress, data.count
                    )
                }
            }
        }
        try check(status)
        return try lastBytes()
    }

    func lock() { if let handle { kkVaultLock(handle) } }

    func beginScan() throws {
        try check(kkVaultBeginScan(requiredHandle()))
    }

    func accept(frame: Data) throws -> (progress: (Int, Int)?, review: VaultReview?) {
        let status = frame.withUnsafeBytes { bytes in
            kkVaultAcceptFrame(requiredHandleUnchecked(), bytes.bindMemory(to: UInt8.self).baseAddress, frame.count)
        }
        try check(status)
        let object = try jsonObject()
        switch object["state"] as? String {
        case "progress":
            guard let received = object["received"] as? Int, let total = object["total"] as? Int else {
                throw VaultBridgeError.malformedResponse
            }
            return ((received, total), nil)
        case "review":
            return (nil, try parseReview(object))
        default:
            throw VaultBridgeError.malformedResponse
        }
    }

    private func parseReview(_ object: [String: Any]) throws -> VaultReview {
        guard let network = object["network"] as? String,
              let inputCount = object["inputCount"] as? Int,
              let outputCount = object["outputCount"] as? Int,
              let inputTotal = object["inputTotal"] as? String,
              let outputTotal = object["outputTotal"] as? String,
              let fee = object["fee"] as? String,
              let inputObjects = object["inputs"] as? [[String: Any]],
              let outputObjects = object["outputs"] as? [[String: Any]] else {
            throw VaultBridgeError.malformedResponse
        }
        let inputs = try inputObjects.map { item -> VaultReviewInput in
            guard let index = item["index"] as? Int,
                  let outpoint = item["outpoint"] as? String,
                  let amount = item["amount"] as? String,
                  let scriptType = item["scriptType"] as? String else {
                throw VaultBridgeError.malformedResponse
            }
            return VaultReviewInput(index: index, outpoint: outpoint, amount: amount, scriptType: scriptType, address: item["address"] as? String)
        }
        let outputs = try outputObjects.map { item -> VaultReviewOutput in
            guard let index = item["index"] as? Int,
                  let amount = item["amount"] as? String,
                  let ownership = item["ownership"] as? String else {
                throw VaultBridgeError.malformedResponse
            }
            return VaultReviewOutput(index: index, amount: amount, ownership: ownership, address: item["address"] as? String)
        }
        return VaultReview(
            network: network, inputCount: inputCount, outputCount: outputCount,
            inputTotal: inputTotal, outputTotal: outputTotal, fee: fee,
            inputs: inputs, outputs: outputs
        )
    }

    func approve() throws -> [Data] {
        try check(kkVaultApprove(requiredHandle(), UInt64(Date().timeIntervalSince1970)))
        let count = kkVaultResponseCount(requiredHandle())
        guard count > 0 else { throw VaultBridgeError.malformedResponse }
        return try (0..<count).map { index in
            let needed = kkVaultResponseFrameCopy(requiredHandleUnchecked(), index, nil, 0)
            guard needed < 0 else { throw VaultBridgeError.malformedResponse }
            var data = Data(count: -needed)
            let written = data.withUnsafeMutableBytes { bytes in
                kkVaultResponseFrameCopy(requiredHandleUnchecked(), index, bytes.bindMemory(to: UInt8.self).baseAddress, data.count)
            }
            guard written == data.count else { throw VaultBridgeError.malformedResponse }
            return data
        }
    }

    func reject() { if let handle { kkVaultReject(handle) } }

    private func requiredHandle() throws -> UnsafeMutableRawPointer {
        guard let handle else { throw VaultBridgeError.runtimeUnavailable }
        return handle
    }

    private func requiredHandleUnchecked() -> UnsafeMutableRawPointer? { handle }

    private func check(_ status: Int32) throws {
        guard status >= 0 else { throw VaultBridgeError.native(lastError()) }
    }

    private func jsonObject() throws -> [String: Any] {
        let data = Data(try lastText().utf8)
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw VaultBridgeError.malformedResponse
        }
        return object
    }

    private func lastText() throws -> String {
        let data = try copyBuffer(kkVaultLastTextCopy)
        guard let text = String(data: data, encoding: .utf8) else { throw VaultBridgeError.malformedResponse }
        return text
    }

    private func lastBytes() throws -> Data { try copyBuffer(kkVaultLastBytesCopy) }

    private func lastError() -> String {
        guard let data = try? copyBuffer(kkVaultLastErrorCopy),
              let text = String(data: data, encoding: .utf8), !text.isEmpty else {
            return "KasKold Vault native operation failed."
        }
        return text
    }

    private func copyBuffer(_ function: (UnsafeMutableRawPointer?, UnsafeMutablePointer<UInt8>?, Int) -> Int) throws -> Data {
        let needed = function(requiredHandleUnchecked(), nil, 0)
        if needed == 0 { return Data() }
        guard needed < 0 else { throw VaultBridgeError.malformedResponse }
        var data = Data(count: -needed)
        let written = data.withUnsafeMutableBytes { bytes in
            function(requiredHandleUnchecked(), bytes.bindMemory(to: UInt8.self).baseAddress, data.count)
        }
        guard written == data.count else { throw VaultBridgeError.malformedResponse }
        return data
    }
}

extension Data {
    func hexLowercased() -> String { map { String(format: "%02x", $0) }.joined() }

    mutating func secureErase() {
        withUnsafeMutableBytes { raw in
            guard let base = raw.baseAddress else { return }
            memset_s(base, raw.count, 0, raw.count)
        }
        removeAll(keepingCapacity: false)
    }
}
