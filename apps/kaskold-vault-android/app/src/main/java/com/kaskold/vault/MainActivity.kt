package com.kaskold.vault

import android.app.KeyguardManager
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Bundle
import android.os.CancellationSignal
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // The Vault shows seed words and keys: no screenshots, recordings or recents previews.
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        setContent { KasKoldVaultApp(this) }
    }

    fun requestVaultReauthentication(onSuccess: () -> Unit, onFailure: (String) -> Unit) {
        val keyguard = getSystemService(KeyguardManager::class.java)
        if (keyguard == null || !keyguard.isDeviceSecure) {
            onFailure("A secure device lock is required to unlock a saved KasKold Vault.")
            return
        }
        BiometricPrompt.Builder(this)
            .setTitle("Unlock KasKold Vault")
            .setSubtitle("Confirm your device credential to use private key custody.")
            .setAllowedAuthenticators(BiometricManager.Authenticators.DEVICE_CREDENTIAL)
            .build()
            .authenticate(
                CancellationSignal(),
                mainExecutor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) = onSuccess()

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) =
                        onFailure("Vault authentication was cancelled or failed.")
                },
            )
    }
}
