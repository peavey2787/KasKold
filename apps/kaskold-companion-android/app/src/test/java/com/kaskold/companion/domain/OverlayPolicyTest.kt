package com.kaskold.companion.domain

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import com.kaskold.companion.domain.overlay.OverlayPolicy
import com.kaskold.companion.domain.overlay.OverlayVisibility

class OverlayPolicyTest {
    @Test fun anyCoverHidesTheWalletAndBootingOnlyCoversIt() {
        assertEquals(OverlayVisibility(overlayNeeded = false, hideWallet = false), OverlayPolicy.visibility(false, false, false, true))
        assertEquals(OverlayVisibility(overlayNeeded = true, hideWallet = false), OverlayPolicy.visibility(false, false, false, false))
        assertEquals(OverlayVisibility(overlayNeeded = true, hideWallet = true), OverlayPolicy.visibility(true, false, false, true))
        assertEquals(OverlayVisibility(overlayNeeded = true, hideWallet = true), OverlayPolicy.visibility(false, true, false, true))
        assertEquals(OverlayVisibility(overlayNeeded = true, hideWallet = true), OverlayPolicy.visibility(false, false, true, true))
    }

    @Test fun theWeatherCoverNeedsTheLockAndTheCoverAndNoSessionPause() {
        assertTrue(OverlayPolicy.weatherCoverShown(lockEnabled = true, coverEnabled = true, suspendedForSession = false))
        assertFalse(OverlayPolicy.weatherCoverShown(lockEnabled = false, coverEnabled = true, suspendedForSession = false))
        assertFalse(OverlayPolicy.weatherCoverShown(lockEnabled = true, coverEnabled = false, suspendedForSession = false))
        assertFalse(OverlayPolicy.weatherCoverShown(lockEnabled = true, coverEnabled = true, suspendedForSession = true))
    }
}
