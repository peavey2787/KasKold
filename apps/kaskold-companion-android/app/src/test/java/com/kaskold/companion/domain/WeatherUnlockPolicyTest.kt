package com.kaskold.companion.domain

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import com.kaskold.companion.domain.weather.WeatherUnlockPolicy
import com.kaskold.companion.domain.weather.WeatherUnlockTarget

class WeatherUnlockPolicyTest {
    @Test fun unlockRequiresConfiguredTargetAndExactNormalizedTapCount() {
        assertTrue(WeatherUnlockPolicy.shouldUnlock(WeatherUnlockTarget.TEMPERATURE, WeatherUnlockTarget.TEMPERATURE, 3, 3))
        assertFalse(WeatherUnlockPolicy.shouldUnlock(WeatherUnlockTarget.TEMPERATURE, WeatherUnlockTarget.LOCATION, 3, 3))
        assertFalse(WeatherUnlockPolicy.shouldUnlock(WeatherUnlockTarget.TEMPERATURE, WeatherUnlockTarget.TEMPERATURE, 2, 3))
        assertTrue(WeatherUnlockPolicy.shouldUnlock(WeatherUnlockTarget.LOCATION, WeatherUnlockTarget.LOCATION, 2, 0))
        assertTrue(WeatherUnlockPolicy.shouldUnlock(WeatherUnlockTarget.LOCATION, WeatherUnlockTarget.LOCATION, 7, 99))
    }
}
