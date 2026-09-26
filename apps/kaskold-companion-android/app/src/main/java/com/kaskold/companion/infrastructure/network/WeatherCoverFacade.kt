package com.kaskold.companion.infrastructure.network

import com.kaskold.companion.domain.weather.WeatherLocation
import com.kaskold.companion.domain.weather.WeatherSnapshot
import com.kaskold.companion.infrastructure.persistence.WeatherCoverSettings
import com.kaskold.companion.infrastructure.persistence.WeatherSnapshotCache

class WeatherCoverFacade(
    private val service: WeatherService,
    private val cache: WeatherSnapshotCache,
) {
    fun cached(settings: WeatherCoverSettings): WeatherSnapshot? = cache.load(settings)

    suspend fun refresh(settings: WeatherCoverSettings): WeatherSnapshot =
        service.forecast(settings).also(cache::save)

    suspend fun searchCities(query: String): List<WeatherLocation> = service.searchCities(query)

    fun clearCache() = cache.clear()
}
