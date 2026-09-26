package com.kaskold.vault

import org.json.JSONObject

internal data class VaultReviewInput(
    val index: Int,
    val outpoint: String,
    val amount: String,
    val scriptType: String,
    val address: String?,
)

internal data class VaultReviewOutput(
    val index: Int,
    val amount: String,
    val ownership: String,
    val address: String?,
)

internal data class VaultReview(
    val network: String,
    val inputCount: Int,
    val outputCount: Int,
    val inputTotal: String,
    val outputTotal: String,
    val fee: String,
    val inputs: List<VaultReviewInput>,
    val outputs: List<VaultReviewOutput>,
)
internal sealed interface QrAcceptResult {
    data class Progress(val received: Int, val total: Int) : QrAcceptResult
    data class Ready(val review: VaultReview) : QrAcceptResult
}

internal class RuntimeBridge : AutoCloseable {
    private var handle: Long = nativeCreateRuntime()

    init {
        check(handle != 0L) { "KasKold Vault Rust runtime failed to initialize" }
    }


    fun restoreWallet(phrase: String, passphrase: String): String {
        val phraseBytes = phrase.encodeToByteArray()
        val passphraseBytes = passphrase.encodeToByteArray()
        return try {
            JSONObject(nativeRestoreWallet(handle, phraseBytes, passphraseBytes)).getString("kpub")
        } finally {
            phraseBytes.fill(0)
            passphraseBytes.fill(0)
        }
    }

    fun sealWallet(wrappingKey: ByteArray): ByteArray = nativeSealWallet(handle, wrappingKey)

    fun unlockSealedWallet(sealedWallet: ByteArray, wrappingKey: ByteArray): String? {
        val result = JSONObject(nativeUnlockSealedWallet(handle, sealedWallet, wrappingKey))
        return if (result.isNull("kpub")) null else result.getString("kpub")
    }

    fun exportPublicAccount(): String = nativeExportPublicAccount(handle)

    fun backupWords(): String = nativeBackupWords(handle)
    fun backupSeedQr(compact: Boolean = false): ByteArray = nativeBackupSeedQr(handle, compact)
    fun backupXprv(): String = nativeBackupXprv(handle)
    fun exportReceiveKey(index: Int): String {
        require(index in 0..65535) { "Address index must be between 0 and 65535" }
        return nativeExportReceiveKey(handle, index)
    }


    fun workflowText(operation: String, input: JSONObject = JSONObject()): JSONObject {
        val operationBytes = operation.encodeToByteArray()
        val inputBytes = input.toString().encodeToByteArray()
        return try {
            JSONObject(nativeWorkflowText(handle, operationBytes, inputBytes))
        } finally {
            operationBytes.fill(0)
            inputBytes.fill(0)
        }
    }

    fun workflowTextWithBytes(operation: String, input: JSONObject = JSONObject(), data: ByteArray): JSONObject {
        val operationBytes = operation.encodeToByteArray()
        val inputBytes = input.toString().encodeToByteArray()
        return try {
            JSONObject(nativeWorkflowTextWithBytes(handle, operationBytes, inputBytes, data))
        } finally {
            operationBytes.fill(0)
            inputBytes.fill(0)
        }
    }

    fun workflowBytes(operation: String, input: JSONObject = JSONObject(), data: ByteArray = ByteArray(0)): ByteArray {
        val operationBytes = operation.encodeToByteArray()
        val inputBytes = input.toString().encodeToByteArray()
        return try {
            nativeWorkflowBytes(handle, operationBytes, inputBytes, data)
        } finally {
            operationBytes.fill(0)
            inputBytes.fill(0)
        }
    }

    fun beginScan() = nativeBeginScan(handle)

    fun acceptQrFrame(frame: ByteArray): QrAcceptResult {
        val json = JSONObject(nativeAcceptQrFrame(handle, frame))
        return when (json.getString("state")) {
            "progress" -> QrAcceptResult.Progress(json.getInt("received"), json.getInt("total"))
            "review" -> QrAcceptResult.Ready(parseReview(json))
            else -> error("KasKold Vault returned an unknown QR state")
        }
    }

    private fun parseReview(json: JSONObject): VaultReview {
        val inputsJson = json.getJSONArray("inputs")
        val outputsJson = json.getJSONArray("outputs")
        val inputs = (0 until inputsJson.length()).map { index ->
            val item = inputsJson.getJSONObject(index)
            VaultReviewInput(
                index = item.getInt("index"),
                outpoint = item.getString("outpoint"),
                amount = item.getString("amount"),
                scriptType = item.getString("scriptType"),
                address = if (item.isNull("address")) null else item.getString("address"),
            )
        }
        val outputs = (0 until outputsJson.length()).map { index ->
            val item = outputsJson.getJSONObject(index)
            VaultReviewOutput(
                index = item.getInt("index"),
                amount = item.getString("amount"),
                ownership = item.getString("ownership"),
                address = if (item.isNull("address")) null else item.getString("address"),
            )
        }
        return VaultReview(
            network = json.getString("network"),
            inputCount = json.getInt("inputCount"),
            outputCount = json.getInt("outputCount"),
            inputTotal = json.getString("inputTotal"),
            outputTotal = json.getString("outputTotal"),
            fee = json.getString("fee"),
            inputs = inputs,
            outputs = outputs,
        )
    }

    fun approve(): List<ByteArray> = nativeApprove(handle).toList()

    fun reject() = nativeReject(handle)
    fun lock() = nativeLock(handle)

    override fun close() {
        if (handle != 0L) {
            nativeDestroyRuntime(handle)
            handle = 0L
        }
    }


    private companion object {
        init { System.loadLibrary("vault_runtime_jni") }

        @JvmStatic private external fun nativeCreateRuntime(): Long
        @JvmStatic private external fun nativeDestroyRuntime(handle: Long)
        @JvmStatic private external fun nativeRestoreWallet(handle: Long, phrase: ByteArray, passphrase: ByteArray): String
        @JvmStatic private external fun nativeSealWallet(handle: Long, wrappingKey: ByteArray): ByteArray
        @JvmStatic private external fun nativeUnlockSealedWallet(handle: Long, sealedWallet: ByteArray, wrappingKey: ByteArray): String
        @JvmStatic private external fun nativeExportPublicAccount(handle: Long): String
        @JvmStatic private external fun nativeBackupWords(handle: Long): String
        @JvmStatic private external fun nativeBackupSeedQr(handle: Long, compact: Boolean): ByteArray
        @JvmStatic private external fun nativeBackupXprv(handle: Long): String
        @JvmStatic private external fun nativeExportReceiveKey(handle: Long, index: Int): String
        @JvmStatic private external fun nativeWorkflowText(handle: Long, operation: ByteArray, input: ByteArray): String
        @JvmStatic private external fun nativeWorkflowBytes(handle: Long, operation: ByteArray, input: ByteArray, data: ByteArray): ByteArray
        @JvmStatic private external fun nativeWorkflowTextWithBytes(handle: Long, operation: ByteArray, input: ByteArray, data: ByteArray): String
        @JvmStatic private external fun nativeBeginScan(handle: Long)
        @JvmStatic private external fun nativeAcceptQrFrame(handle: Long, frame: ByteArray): String
        @JvmStatic private external fun nativeApprove(handle: Long): Array<ByteArray>
        @JvmStatic private external fun nativeReject(handle: Long)
        @JvmStatic private external fun nativeLock(handle: Long)
    }
}
