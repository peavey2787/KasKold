import Foundation

@MainActor
extension VaultViewModel {
    func receive(change: Bool = false, index: Int = 0) {
        perform(returnTo: .walletMenu) {
            let result = try runtime.workflowText(
                "receive_address",
                input: ["network": "mainnet", "change": change, "index": index]
            )
            guard let address = result["address"] as? String else { throw VaultBridgeError.malformedResponse }
            receiveIndex = String(index)
            return .receive(address, change, index)
        }
    }

    func recoveryMenu() { screen = .recoveryMenu }
    func multisigMenu() { screen = .multisigMenu }

    func multisigKpub() {
        workflowReturn = .multisigMenu
        perform(returnTo: .multisigMenu) {
            let result = try runtime.workflowText("multisig_kpub")
            guard let kpub = result["kpub"] as? String else { throw VaultBridgeError.malformedResponse }
            return .secretText(
                "kpub Multisig QR",
                "Share this public BIP45 account key with the other multisig participants. It contains no private key material.",
                kpub
            )
        }
    }

    func walletInventory() {
        perform(returnTo: .walletMenu) {
            let result = try runtime.workflowText("wallets")
            guard let items = result["wallets"] as? [[String: Any]] else { throw VaultBridgeError.malformedResponse }
            let wallets = try items.map { item -> NativeWalletSummary in
                guard let index = item["index"] as? Int,
                      let name = item["name"] as? String,
                      let active = item["active"] as? Bool,
                      let kind = item["kind"] as? String else { throw VaultBridgeError.malformedResponse }
                return NativeWalletSummary(index: index, name: name, active: active, kind: kind, kpub: item["kpub"] as? String)
            }
            return .walletInventory(wallets)
        }
    }

    func switchWallet(_ index: Int) {
        perform(returnTo: .walletMenu) {
            _ = try runtime.workflowText("switch_wallet", input: ["index": index])
            try persistUnlocked()
            return .walletMenu
        }
    }

    func tool(_ tool: NativeTool, returnTo: Screen) {
        workflowReturn = returnTo
        toolValue1 = ""
        toolValue2 = ""
        if tool == .bip85 { toolValue1 = "12"; toolValue2 = "0" }
        screen = .toolForm(tool)
    }

    func submitTool(_ tool: NativeTool) {
        perform(returnTo: .toolForm(tool)) {
            let title: String
            let warning: String
            let result: [String: Any]
            switch tool {
            case .importRawKey:
                result = try runtime.workflowText("import_raw_key", input: ["privateKey": toolValue1.trimmingCharacters(in: .whitespacesAndNewlines)])
                try persistUnlocked()
                title = "Raw Private Key Imported"
                warning = "The imported wallet is restricted to its matching single-key address."
            case .importXprv:
                result = try runtime.workflowText("import_xprv", input: ["xprv": toolValue1.trimmingCharacters(in: .whitespacesAndNewlines)])
                try persistUnlocked()
                title = "XPrv Imported"
                warning = "The imported account key is now held by the Rust Vault runtime."
            case .multisigCreate:
                let threshold = Int(toolValue1) ?? 2
                let cosigners = toolValue2.split(whereSeparator: \.isNewline).map { String($0).trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
                result = try runtime.workflowText(
                    "create_multisig",
                    input: ["threshold": threshold, "cosigners": cosigners, "network": "mainnet", "chain": 0, "index": 0]
                )
                title = "Multisig"
                warning = "Verify the descriptor and address with every participant before receiving funds."
            case .multisigImport:
                result = try runtime.workflowText(
                    "import_multisig",
                    input: ["descriptor": toolValue1.trimmingCharacters(in: .whitespacesAndNewlines), "network": "mainnet", "chain": 0, "index": 0]
                )
                title = "Multisig Descriptor"
                warning = "Verify the derived address before use."
            case .bip85:
                result = try runtime.workflowText("bip85", input: ["wordCount": Int(toolValue1) ?? 12, "index": Int(toolValue2) ?? 0])
                title = "BIP85 Child Wallet"
                warning = "These derived recovery words control a child wallet. Keep them private."
            case .signMessage:
                result = try runtime.workflowText("sign_message", input: ["message": toolValue1])
                title = "Signed Message"
                warning = "Verify the message before sharing its signature."
            case .commitSecret:
                result = try runtime.workflowText("commit_secret", input: ["secret": toolValue1])
                title = "Commit Secret"
                warning = "Save the commitment/payload needed by the counterpart protocol."
            case .decryptSecret:
                result = try runtime.workflowText("decrypt_secret", input: ["payloadHex": toolValue1.trimmingCharacters(in: .whitespacesAndNewlines)])
                title = "Decrypt Secret"
                warning = "Decrypted secret material is shown only on this explicit screen."
            case .signingPolicy:
                _ = try runtime.workflowText("set_signing_policy", input: ["notBeforeUtc": toolValue1, "weeklyWindows": toolValue2])
                result = ["status": "Session signing policy enabled"]
                title = "Security"
                warning = "Session policy is enforced at the Rust signing boundary; iOS system time is not an authenticated hardware RTC."
            }
            toolValue1 = ""
            toolValue2 = ""
            return .secretText(title, warning, try prettyJSON(result))
        }
    }

    func fileWorkflow(_ workflow: NativeFileWorkflow, returnTo: Screen) {
        workflowReturn = returnTo
        workflowPassword = ""
        screen = .fileWorkflow(workflow)
    }

    func processFileWorkflow(_ workflow: NativeFileWorkflow, data: inout Data) {
        perform(returnTo: .fileWorkflow(workflow)) {
            switch workflow {
            case .recoveryMaterial:
                _ = try runtime.workflowTextWithBytes("recover_material", input: ["passphrase": workflowPassword], data: &data)
                try persistUnlocked()
                workflowPassword = ""
                return .walletMenu
            case .walletBackup:
                let text = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                if text.hasPrefix("kprv") {
                    _ = try runtime.workflowText("import_xprv", input: ["xprv": text])
                } else {
                    guard !workflowPassword.isEmpty else { throw VaultBridgeError.native("Enter the encrypted backup password.") }
                    _ = try runtime.workflowTextWithBytes("restore_portable", input: ["password": workflowPassword], data: &data)
                }
                try persistUnlocked()
                workflowPassword = ""
                return .walletMenu
            case .portableBackup:
                _ = try runtime.workflowTextWithBytes("restore_portable", input: ["password": workflowPassword], data: &data)
                try persistUnlocked()
                workflowPassword = ""
                return .walletMenu
            case .stegoBackup:
                _ = try runtime.workflowTextWithBytes("restore_stego", input: ["password": workflowPassword], data: &data)
                try persistUnlocked()
                workflowPassword = ""
                return .walletMenu
            case .transaction:
                let result = try runtime.workflowTextWithBytes("transaction_file", data: &data)
                return .review(try review(from: result))
            case .kpub:
                guard let value = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) else { throw VaultBridgeError.malformedResponse }
                let result = try runtime.workflowText("normalize_kpub", input: ["value": value])
                guard let normalized = result["value"] as? String else { throw VaultBridgeError.malformedResponse }
                return .secretText("kpub (Watch-Only)", "Validated public account. This value contains no private keys.", normalized)
            case .multisigAddress:
                guard let value = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) else { throw VaultBridgeError.malformedResponse }
                let result = try runtime.workflowText("validate_address", input: ["value": value])
                guard let normalized = result["value"] as? String else { throw VaultBridgeError.malformedResponse }
                return .secretText("Multisig Address", "Validated Kaspa address. Verify it with every participant before use.", normalized)
            case .multisigDescriptor:
                guard let descriptor = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines), !descriptor.isEmpty else { throw VaultBridgeError.malformedResponse }
                let result = try runtime.workflowText(
                    "import_multisig",
                    input: ["descriptor": descriptor, "network": "mainnet", "chain": 0, "index": 0]
                )
                return .secretText("Multisig Descriptor", "Descriptor validated and retained for multisig signing/change verification.", try prettyJSON(result))
            case .covenantRestore:
                let normalized = try runtime.workflowBytes("normalize_covenant_backup", data: &data)
                return .secretText("Covenant Restore", "Validated against the shared M5 covenant-backup format.", normalized.hexLowercased())
            case .portableExport:
                var empty = Data()
                let exported = try runtime.workflowBytes("portable_backup", input: ["password": workflowPassword], data: &empty)
                empty.secureErase()
                workflowPassword = ""
                return .exportFile("Encrypted Wallet Backup", "kaskold-vault-backup.kwp", "application/octet-stream", exported)
            case .portableXprvExport:
                var empty = Data()
                let exported = try runtime.workflowBytes("portable_xprv_backup", input: ["password": workflowPassword], data: &empty)
                empty.secureErase()
                workflowPassword = ""
                return .exportFile("Encrypted XPrv Backup", "kaskold-xprv-backup.kwp", "application/octet-stream", exported)
            case .stegoExport:
                let exported = try runtime.workflowBytes("stego_backup", input: ["password": workflowPassword], data: &data)
                workflowPassword = ""
                return .exportFile("Steganographic Backup", "kaskold-steganographic-backup.jpg", "image/jpeg", exported)
            }
        }
    }


}
