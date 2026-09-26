package com.kaskold.companion.app

import android.content.Context
import com.kaskold.companion.infrastructure.network.WeatherCoverFacade
import com.kaskold.companion.infrastructure.network.WeatherService
import com.kaskold.companion.infrastructure.persistence.AppPreferences
import com.kaskold.companion.infrastructure.persistence.WeatherCoverPreferences
import com.kaskold.companion.infrastructure.persistence.WeatherSnapshotCache
import com.kaskold.companion.infrastructure.security.AppLockService

/** Native services that remain intentionally outside the embedded Companion wallet surface. */
class AppContainer(context: Context) {
    private val applicationContext = context.applicationContext

    val preferences = AppPreferences(applicationContext)
    val weatherCoverPreferences = WeatherCoverPreferences(applicationContext)
    val weatherCover = WeatherCoverFacade(WeatherService(), WeatherSnapshotCache(applicationContext))
    val appLock = AppLockService(applicationContext)
}
