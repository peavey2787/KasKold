package com.kaskold.vault

import org.json.JSONObject

/** File imports and encrypted exports, each ending on the screen that shows its result. */
internal class NativeFileWorkflows(
    private val runtime: RuntimeBridge,
    private val persist: () -> Unit,
) {
    fun process(workflow: NativeFileWorkflow, data: ByteArray, password: String, previous: VaultScreenState): VaultScreenState =
        when (workflow) {
            NativeFileWorkflow.RecoveryMaterial -> restore("recover_material", JSONObject().put("passphrase", password), data)
            NativeFileWorkflow.WalletBackup -> restoreWalletBackup(data, password)
            NativeFileWorkflow.PortableBackup -> restorePortable(data, password)
            NativeFileWorkflow.StegoBackup -> restore("restore_stego", JSONObject().put("password", password), data)
            NativeFileWorkflow.Transaction -> review(data)
            NativeFileWorkflow.Kpub -> validatedKpub(data, previous)
            NativeFileWorkflow.MultisigAddress -> validatedAddress(data, previous)
            NativeFileWorkflow.MultisigDescriptor -> importedDescriptor(data, previous)
            NativeFileWorkflow.CovenantRestore -> normalizedCovenant(data, previous)
            NativeFileWorkflow.PortableExport -> export(
                "portable_backup", password, ByteArray(0),
                "Encrypted Wallet Backup", "kaskold-vault-backup.kwp", "application/octet-stream", previous,
            )
            NativeFileWorkflow.PortableXprvExport -> export(
                "portable_xprv_backup", password, ByteArray(0),
                "Encrypted XPrv Backup", "kaskold-xprv-backup.kwp", "application/octet-stream", previous,
            )
            NativeFileWorkflow.StegoExport -> export(
                "stego_backup", password, data,
                "Steganographic Backup", "kaskold-steganographic-backup.jpg", "image/jpeg", previous,
            )
        }

    private fun restore(operation: String, input: JSONObject, data: ByteArray): VaultScreenState {
        runtime.workflowTextWithBytes(operation, input, data)
        persist()
        return VaultScreenState.WalletMenu
    }

    private fun restorePortable(data: ByteArray, password: String): VaultScreenState =
        restore("restore_portable", JSONObject().put("password", password), data)

    private fun restoreWalletBackup(data: ByteArray, password: String): VaultScreenState {
        val text = data.toString(Charsets.UTF_8).trim()
        if (!text.startsWith("kprv")) {
            require(password.isNotEmpty()) { "Enter the encrypted backup password" }
            return restorePortable(data, password)
        }
        runtime.workflowText("import_xprv", JSONObject().put("xprv", text))
        persist()
        return VaultScreenState.WalletMenu
    }

    private fun review(data: ByteArray): VaultScreenState {
        val review = runtime.workflowTextWithBytes("transaction_file", JSONObject(), data)
        return VaultScreenState.Review(runtime.parseReview(review))
    }

    private fun validatedKpub(data: ByteArray, previous: VaultScreenState): VaultScreenState {
        val value = runtime.workflowText("normalize_kpub", JSONObject().put("value", data.toString(Charsets.UTF_8).trim())).getString("value")
        return VaultScreenState.SecretText("kpub (Watch-Only)", "Validated public account. This value contains no private keys.", value, previous)
    }

    private fun validatedAddress(data: ByteArray, previous: VaultScreenState): VaultScreenState {
        val value = runtime.workflowText("validate_address", JSONObject().put("value", data.toString(Charsets.UTF_8).trim())).getString("value")
        return VaultScreenState.SecretText("Multisig Address", "Validated Kaspa address. Verify it with every participant before use.", value, previous)
    }

    private fun importedDescriptor(data: ByteArray, previous: VaultScreenState): VaultScreenState {
        val descriptor = data.toString(Charsets.UTF_8).trim()
        val result = runtime.workflowText(
            "import_multisig",
            JSONObject().put("descriptor", descriptor).put("network", "mainnet").put("chain", 0).put("index", 0),
        )
        return VaultScreenState.SecretText("Multisig Descriptor", "Descriptor validated and retained for multisig signing/change verification.", result.toString(2), previous)
    }

    private fun normalizedCovenant(data: ByteArray, previous: VaultScreenState): VaultScreenState {
        val normalized = runtime.workflowBytes("normalize_covenant_backup", JSONObject(), data)
        return VaultScreenState.SecretText(
            "Covenant Restore", "Validated against the shared M5 covenant-backup format.", normalized.toHex(), previous,
        )
    }

    private fun export(
        operation: String, password: String, carrier: ByteArray,
        title: String, filename: String, mimeType: String, previous: VaultScreenState,
    ): VaultScreenState {
        val bytes = runtime.workflowBytes(operation, JSONObject().put("password", password), carrier)
        return VaultScreenState.ExportFile(title, filename, mimeType, bytes, previous)
    }
}

private fun ByteArray.toHex(): String = joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }
