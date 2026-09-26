package com.kaskold.vault.security

import android.content.Context

/**
 * Persists only the authenticated ciphertext produced by Rust `vault-runtime`.
 *
 * No mnemonic, seed, xprv, or other wallet secret is accepted by this class.
 * The Rust custody boundary encrypts wallet custody state before it reaches Kotlin. The
 * platform wrapping key used for that operation is managed separately by
 * [VaultWrappingKeyStore].
 */
class VaultBlobStore(private val context: Context) {
    private val fileName = "vault-sealed-wallet-v1.bin"

    fun writeSealedWallet(sealedWallet: ByteArray) {
        require(isInventory(sealedWallet) || isCurrent(sealedWallet)) { "invalid sealed Vault wallet container" }
        context.openFileOutput(fileName, Context.MODE_PRIVATE).use { output ->
            output.write(sealedWallet)
        }
    }

    fun readSealedWallet(): ByteArray? {
        val file = context.getFileStreamPath(fileName)
        if (!file.isFile) return null
        val sealedWallet = file.readBytes()
        require(isInventory(sealedWallet) || isCurrent(sealedWallet) || isV2(sealedWallet) || isLegacy(sealedWallet)) { "invalid sealed Vault wallet container" }
        return sealedWallet
    }

    fun delete() {
        context.deleteFile(fileName)
    }

    private companion object {
        val INVENTORY_MAGIC = byteArrayOf('K'.code.toByte(), 'V'.code.toByte(), 'I'.code.toByte(), '1'.code.toByte())
        const val MIN_INVENTORY_LENGTH = 128
        const val MAX_INVENTORY_LENGTH = 4096
        val CURRENT_MAGIC = byteArrayOf('K'.code.toByte(), 'H'.code.toByte(), 'V'.code.toByte(), '3'.code.toByte())
        val V2_MAGIC = byteArrayOf('K'.code.toByte(), 'H'.code.toByte(), 'V'.code.toByte(), '2'.code.toByte())
        val LEGACY_MAGIC = byteArrayOf('K'.code.toByte(), 'H'.code.toByte(), 'V'.code.toByte(), '1'.code.toByte())
        const val CURRENT_SEALED_WALLET_LENGTH = 216
        const val V2_SEALED_WALLET_LENGTH = 210
        const val LEGACY_SEALED_WALLET_LENGTH = 96

        fun isInventory(value: ByteArray): Boolean =
            value.size in MIN_INVENTORY_LENGTH..MAX_INVENTORY_LENGTH && value.copyOfRange(0, 4).contentEquals(INVENTORY_MAGIC)

        fun isCurrent(value: ByteArray): Boolean =
            value.size == CURRENT_SEALED_WALLET_LENGTH && value.copyOfRange(0, 4).contentEquals(CURRENT_MAGIC)

        fun isV2(value: ByteArray): Boolean =
            value.size == V2_SEALED_WALLET_LENGTH && value.copyOfRange(0, 4).contentEquals(V2_MAGIC)

        fun isLegacy(value: ByteArray): Boolean =
            value.size == LEGACY_SEALED_WALLET_LENGTH && value.copyOfRange(0, 4).contentEquals(LEGACY_MAGIC)
    }
}
