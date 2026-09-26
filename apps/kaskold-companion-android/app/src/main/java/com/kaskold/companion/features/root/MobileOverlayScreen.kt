package com.kaskold.companion.features.root

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.launch
import com.kaskold.companion.app.AppContainer
import com.kaskold.companion.features.cover.WeatherCoverScreen
import com.kaskold.companion.features.settings.SecuritySettingsScreen

internal sealed interface CompanionBootState {
    data object Loading : CompanionBootState
    data object Ready : CompanionBootState
    data class Failed(val message: String) : CompanionBootState
}

@Composable
internal fun MobileOverlayScreen(
    container: AppContainer,
    activity: FragmentActivity,
    bootState: CompanionBootState,
    showMobileSettings: Boolean,
    onRetry: () -> Unit,
    onCloseMobileSettings: () -> Unit,
) {
    val security by container.appLock.state.collectAsState()
    val weather by container.weatherCoverPreferences.settings.collectAsState()
    val showWeatherCover = security.enabled && weather.enabled && !security.privacyCoverSuspendedForSession

    when {
        security.locked -> LockedScreen { container.appLock.unlock(activity) }
        showWeatherCover -> WeatherCoverScreen(container, activity)
        showMobileSettings -> Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            SecuritySettingsScreen(container, activity, onCloseMobileSettings, onCloseMobileSettings)
        }
        bootState == CompanionBootState.Loading -> CompanionBootOverlay("Loading Companion…")
        bootState is CompanionBootState.Failed -> CompanionBootOverlay(
            message = bootState.message,
            actionLabel = "Retry",
            onAction = onRetry,
        )
        else -> Unit
    }
}

@Composable
private fun CompanionBootOverlay(
    message: String,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            Modifier.fillMaxSize().padding(28.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            if (actionLabel == null) CircularProgressIndicator()
            Text(
                text = message,
                modifier = Modifier.padding(top = 18.dp),
                color = MaterialTheme.colorScheme.onBackground,
                textAlign = TextAlign.Center,
            )
            if (actionLabel != null && onAction != null) {
                Button(onClick = onAction, modifier = Modifier.padding(top = 18.dp)) { Text(actionLabel) }
            }
        }
    }
}

@Composable
private fun LockedScreen(unlock: suspend () -> Boolean) {
    val scope = rememberCoroutineScope()
    Box(
        Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(Icons.Default.Lock, null, tint = MaterialTheme.colorScheme.primary)
            Text("KasKold Locked", style = MaterialTheme.typography.headlineSmall)
            Button(onClick = { scope.launch { unlock() } }) { Text("Unlock") }
        }
    }
}
