package com.kaskold.vault.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Protects the 256-bit *wrapping key* used by Rust to seal the Vault seed.
 *
 * The Android Keystore master key is non-exportable and usable only for
 * [AUTH_VALIDITY_SECONDS] after the device credential is confirmed, so a copy of
 * the app's files or a bypass of the app's own UI cannot open it. Without that
 * authentication Keystore throws `UserNotAuthenticatedException`, which callers
 * handle by prompting and retrying. The random wrapping key
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
        initCipher(cipher, Cipher.ENCRYPT_MODE, null)
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
                initCipher(cipher, Cipher.DECRYPT_MODE, GCMParameterSpec(128, iv))
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

    private fun initCipher(cipher: Cipher, mode: Int, parameters: GCMParameterSpec?) {
        try {
            cipher.init(mode, masterKey(), parameters)
        } catch (error: KeyPermanentlyInvalidatedException) {
            throw IllegalStateException(
                "The device lock was removed, so Android destroyed this Vault's key. " +
                    "Delete the saved wallet and restore it from your recovery words.",
                error,
            )
        }
    }

    private fun masterKey(): SecretKey {
        val keyStore = androidKeyStore()
        (keyStore.getKey(alias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        val spec = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(true)
            .setUserAuthenticationParameters(AUTH_VALIDITY_SECONDS, KeyProperties.AUTH_DEVICE_CREDENTIAL)
            .setUnlockedDeviceRequired(true)
            .build()
        return try {
            generator.init(spec)
            generator.generateKey()
        } catch (error: GeneralSecurityException) {
            throw IllegalStateException(LOCK_REQUIRED, error)
        } catch (error: IllegalStateException) {
            throw IllegalStateException(LOCK_REQUIRED, error)
        }
    }

    private fun androidKeyStore(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    private companion object {
        const val WRAPPING_KEY_LENGTH = 32
        const val AUTH_VALIDITY_SECONDS = 60
        const val LOCK_REQUIRED = "Set a device screen lock to save this wallet on the device."
    }
}
