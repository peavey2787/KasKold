package com.kaskold.companion.domain.overlay

/** Whether the native overlay is up, and whether the wallet beneath it is hidden. */
data class OverlayVisibility(val overlayNeeded: Boolean, val hideWallet: Boolean)

object OverlayPolicy {
    /** The lock, the weather cover and mobile settings hide the wallet; booting only covers it. */
    fun visibility(locked: Boolean, weatherCover: Boolean, mobileSettings: Boolean, booted: Boolean): OverlayVisibility {
        val hideWallet = locked || weatherCover || mobileSettings
        return OverlayVisibility(overlayNeeded = hideWallet || !booted, hideWallet = hideWallet)
    }

    /** The weather cover shows only while the lock is on and the cover is not paused for this session. */
    fun weatherCoverShown(lockEnabled: Boolean, coverEnabled: Boolean, suspendedForSession: Boolean): Boolean =
        lockEnabled && coverEnabled && !suspendedForSession
}
