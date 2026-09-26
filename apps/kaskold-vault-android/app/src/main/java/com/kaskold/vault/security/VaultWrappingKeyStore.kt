package com.kaskold.vault.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Protects the 256-bit *wrapping key* used by Rust to seal the Vault seed.
 *
 * The Android Keystore master key is non-exportable. The random wrapping key
 * is decrypted only long enough to be passed to `vault-runtime`; callers must
 * zeroize the returned ByteArray immediately after the Rust call. Wallet seed
 * bytes never enter Kotlin.
 */
class VaultWrappingKeyStore(private val context: Context) {
    private val alias = "kaskold-vault-wrapping-master-v1"
    private val fileName = "vault-wrapping-key-v1.bin"

    fun loadOrCreate(): ByteArray {
        val file = context.getFileStreamPath(fileName)
        if (file.isFile) return unwrap(file.readBytes())

        val wrappingKey = ByteArray(WRAPPING_KEY_LENGTH)
        SecureRandom().nextBytes(wrappingKey)
        try {
            val encoded = wrap(wrappingKey)
            try {
                context.openFileOutput(fileName, Context.MODE_PRIVATE).use { it.write(encoded) }
            } finally {
                encoded.fill(0)
            }
            return wrappingKey.copyOf()
        } finally {
            wrappingKey.fill(0)
        }
    }

    fun delete() {
        context.deleteFile(fileName)
        val keyStore = androidKeyStore()
        if (keyStore.containsAlias(alias)) keyStore.deleteEntry(alias)
    }

    private fun wrap(wrappingKey: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, masterKey())
        val ciphertext = cipher.doFinal(wrappingKey)
        return try {
            byteArrayOf(cipher.iv.size.toByte()) + cipher.iv + ciphertext
        } finally {
            ciphertext.fill(0)
        }
    }

    private fun unwrap(encoded: ByteArray): ByteArray {
        try {
            require(encoded.isNotEmpty()) { "wrapped Vault key is empty" }
            val ivLength = encoded[0].toInt() and 0xff
            require(ivLength in 12..32 && encoded.size > 1 + ivLength) { "invalid wrapped Vault key" }
            val iv = encoded.copyOfRange(1, 1 + ivLength)
            val ciphertext = encoded.copyOfRange(1 + ivLength, encoded.size)
            return try {
                val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                cipher.init(Cipher.DECRYPT_MODE, masterKey(), GCMParameterSpec(128, iv))
                cipher.doFinal(ciphertext).also {
                    require(it.size == WRAPPING_KEY_LENGTH) { "invalid Vault wrapping key length" }
                }
            } finally {
                iv.fill(0)
                ciphertext.fill(0)
            }
        } finally {
            encoded.fill(0)
        }
    }

    private fun masterKey(): SecretKey {
        val keyStore = androidKeyStore()
        (keyStore.getKey(alias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    private fun androidKeyStore(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    private companion object {
        const val WRAPPING_KEY_LENGTH = 32
    }
}
