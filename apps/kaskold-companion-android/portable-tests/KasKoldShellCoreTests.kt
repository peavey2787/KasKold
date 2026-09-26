import com.kaskold.companion.domain.weather.WeatherUnlockPolicy
import com.kaskold.companion.domain.weather.WeatherUnlockTarget

fun main() {
    check(WeatherUnlockPolicy.normalizeTapCount(0) == 2)
    check(WeatherUnlockPolicy.normalizeTapCount(99) == 7)
    check(WeatherUnlockPolicy.shouldUnlock(
        WeatherUnlockTarget.TEMPERATURE,
        WeatherUnlockTarget.TEMPERATURE,
        3,
        3,
    ))
    check(!WeatherUnlockPolicy.shouldUnlock(
        WeatherUnlockTarget.TEMPERATURE,
        WeatherUnlockTarget.LOCATION,
        3,
        3,
    ))
    println("PASS: Android portable weather-cover policy tests")
}
