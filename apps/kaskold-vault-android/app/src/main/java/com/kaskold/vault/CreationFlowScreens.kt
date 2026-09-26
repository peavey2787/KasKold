package com.kaskold.vault

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp

@Composable
internal fun CreationNameScreen(
    state: VaultScreenState.CreationName,
    onContinue: (String) -> Unit,
    onCancel: () -> Unit,
) {
    var name by remember(state.suggestedName) { mutableStateOf(state.suggestedName) }
    Text("Wallet Name", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedTextField(
        value = name,
        onValueChange = { name = it },
        label = { Text("Wallet name") },
        modifier = Modifier.fillMaxWidth(),
    )
    Button(onClick = { onContinue(name) }, modifier = Modifier.fillMaxWidth()) { Text("Continue") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationWordCountScreen(onSelect: (Int) -> Unit, onCancel: () -> Unit) {
    Text("Recovery Words", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Button(onClick = { onSelect(24) }, modifier = Modifier.fillMaxWidth()) { Text("24 Words") }
    OutlinedButton(onClick = { onSelect(12) }, modifier = Modifier.fillMaxWidth()) { Text("12 Words") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationDiceChoiceScreen(onChoose: (Boolean) -> Unit, onCancel: () -> Unit) {
    Text("Add Dice", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Platform CSPRNG entropy is always used. Dice can add extra user entropy.")
    Button(onClick = { onChoose(false) }, modifier = Modifier.fillMaxWidth()) { Text("No Dice") }
    OutlinedButton(onClick = { onChoose(true) }, modifier = Modifier.fillMaxWidth()) { Text("Add Dice Rolls") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationDiceCountScreen(
    state: VaultScreenState.CreationDiceCount,
    onSelect: (Int) -> Unit,
    onCancel: () -> Unit,
) {
    Text("How Many Dice Rolls?", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    state.targets.forEach { target ->
        OutlinedButton(onClick = { onSelect(target) }, modifier = Modifier.fillMaxWidth()) {
            Text("$target Rolls")
        }
    }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationDiceRollScreen(
    state: VaultScreenState.CreationDiceRoll,
    onRoll: (Int) -> Unit,
    onUndo: () -> Unit,
    onReset: () -> Unit,
    onCancel: () -> Unit,
) {
    Text("Dice Rolls", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("${state.collected}/${state.target} rolls collected.")
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        for (row in 0..1) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                for (column in 1..3) {
                    val value = row * 3 + column
                    Button(onClick = { onRoll(value) }, modifier = Modifier.weight(1f)) { Text(value.toString()) }
                }
            }
        }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
        OutlinedButton(onClick = onUndo, modifier = Modifier.weight(1f)) { Text("Undo") }
        OutlinedButton(onClick = onReset, modifier = Modifier.weight(1f)) { Text("Reset") }
    }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationTouchChoiceScreen(onChoose: (Boolean) -> Unit, onCancel: () -> Unit) {
    Text("Add Touch", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Touch is optional additive entropy; the platform CSPRNG remains mandatory.")
    Button(onClick = { onChoose(false) }, modifier = Modifier.fillMaxWidth()) { Text("No Touch Entropy") }
    OutlinedButton(onClick = { onChoose(true) }, modifier = Modifier.fillMaxWidth()) { Text("Add Touch Entropy") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationTouchScreen(
    state: VaultScreenState.CreationTouch,
    onSample: (Long, Int, Int) -> Unit,
    onReset: () -> Unit,
    onCancel: () -> Unit,
) {
    Text("Touch Entropy", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("${state.collected}/${state.target} movement samples collected. Move your finger continuously below.")
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(240.dp)
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .pointerInput(state.target) {
                detectDragGestures { change, _ ->
                    val width = size.width.coerceAtLeast(1).toFloat()
                    val height = size.height.coerceAtLeast(1).toFloat()
                    val x = (change.position.x * 65535f / width).toInt().coerceIn(0, 65535)
                    val y = (change.position.y * 65535f / height).toInt().coerceIn(0, 65535)
                    onSample(change.uptimeMillis * 1000L, x, y)
                }
            }
            .padding(16.dp),
    ) {
        Text("Move finger here", style = MaterialTheme.typography.bodyLarge)
    }
    OutlinedButton(onClick = onReset, modifier = Modifier.fillMaxWidth()) { Text("Reset") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationPassphraseChoiceScreen(onChoose: (Boolean) -> Unit, onCancel: () -> Unit) {
    Text("BIP39 Pass.", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Optional extra secret for your words. It creates a different wallet and is required for recovery.")
    Button(onClick = { onChoose(false) }, modifier = Modifier.fillMaxWidth()) { Text("No Passphrase") }
    OutlinedButton(onClick = { onChoose(true) }, modifier = Modifier.fillMaxWidth()) { Text("Use Passphrase") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationPassphraseScreen(
    onSubmit: (String, String) -> Unit,
    onCancel: () -> Unit,
) {
    var passphrase by remember { mutableStateOf("") }
    var confirmation by remember { mutableStateOf("") }
    Text("Passphrase", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedTextField(
        value = passphrase,
        onValueChange = { passphrase = it },
        label = { Text("BIP39 passphrase") },
        visualTransformation = PasswordVisualTransformation(),
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedTextField(
        value = confirmation,
        onValueChange = { confirmation = it },
        label = { Text("Confirm passphrase") },
        visualTransformation = PasswordVisualTransformation(),
        modifier = Modifier.fillMaxWidth(),
    )
    Button(onClick = { onSubmit(passphrase, confirmation) }, modifier = Modifier.fillMaxWidth()) { Text("Continue") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun CreationRecoveryAcknowledgementScreen(onDone: () -> Unit) {
    Text("Recovery Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Your recovery words are the master backup for this wallet.")
    Text("Write them down in order and keep them private, offline, and in a secure location.")
    Text("Anyone who has these words can restore the wallet and spend its funds.")
    Text("Verify your recovery backup before relying on encrypted device storage.")
    Button(onClick = onDone, modifier = Modifier.fillMaxWidth()) { Text("I Backed Up My Words") }
}

@Composable
internal fun CreationStorageFinalizeScreen(onSave: () -> Unit, onSession: () -> Unit) {
    Text("Storage", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Button(onClick = onSave, modifier = Modifier.fillMaxWidth()) { Text("Save Securely on Device") }
    Text("Android Vault seals the shared Rust wallet inventory with an Android Keystore wrapping key.")
    OutlinedButton(onClick = onSession, modifier = Modifier.fillMaxWidth()) { Text("Use for This Session Only") }
}

@Composable
internal fun CreationStorageProtectionScreen(onProtect: () -> Unit, onSession: () -> Unit) {
    Text("PROTECT THIS WALLET?", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Button(onClick = onProtect, modifier = Modifier.fillMaxWidth()) { Text("Use Android Device Protection") }
    Text("The device-bound Android Keystore key is non-exportable. KasKold does not write plaintext private material to app storage.")
    OutlinedButton(onClick = onSession, modifier = Modifier.fillMaxWidth()) { Text("Session Only") }
}

/** Restore a wallet from its recovery words (and optional BIP39 passphrase). */
@Composable
internal fun RestoreScreen(
    initial: VaultScreenState.Restore,
    onCancel: () -> Unit,
    onRestore: (String, String) -> Unit,
) {
    var phrase by remember(initial) { mutableStateOf(initial.phrase) }
    var passphrase by remember(initial) { mutableStateOf(initial.passphrase) }
    Text("Restore Wallet", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Enter the recovery words in order, separated by spaces.")
    OutlinedTextField(
        value = phrase,
        onValueChange = { phrase = it },
        label = { Text("Recovery words") },
        minLines = 3,
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedTextField(
        value = passphrase,
        onValueChange = { passphrase = it },
        label = { Text("BIP39 passphrase (optional)") },
        visualTransformation = PasswordVisualTransformation(),
        modifier = Modifier.fillMaxWidth(),
    )
    Button(
        onClick = { onRestore(phrase, passphrase) },
        enabled = phrase.isNotBlank(),
        modifier = Modifier.fillMaxWidth(),
    ) { Text("Restore") }
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Cancel") }
}
