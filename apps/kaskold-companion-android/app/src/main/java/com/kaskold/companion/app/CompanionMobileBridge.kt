package com.kaskold.companion.app

import android.webkit.JavascriptInterface

internal class CompanionMobileBridge(
    private val openMobileSettings: () -> Unit,
    private val resetWalletSurface: () -> Unit,
) {
    @JavascriptInterface
    fun openMobileSettings() = openMobileSettings.invoke()

    @JavascriptInterface
    fun resetWalletSurface() = resetWalletSurface.invoke()
}
