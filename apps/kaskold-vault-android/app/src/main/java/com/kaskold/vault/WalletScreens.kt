package com.kaskold.vault

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
internal fun LockedScreen(onUnlock: () -> Unit, onCreate: () -> Unit, onRestore: () -> Unit) {
    Text("Vault is locked. Wallet secrets are held only by the native Rust signing runtime.")
    Button(onClick = onUnlock, modifier = Modifier.fillMaxWidth()) { Text("Unlock saved wallet") }
    Button(onClick = onCreate, modifier = Modifier.fillMaxWidth()) { Text("Create Wallet") }
    OutlinedButton(onClick = onRestore, modifier = Modifier.fillMaxWidth()) { Text("Restore wallet") }
}

@Composable
internal fun BackupScreen(state: VaultScreenState.Backup, onAdvanced: () -> Unit, onDone: () -> Unit) {
    val words = remember(state.phrase) { state.phrase.trim().split(Regex("\\s+")).filter(String::isNotEmpty) }
    var index by remember(state.phrase) { mutableIntStateOf(0) }
    var showAll by remember(state.phrase) { mutableStateOf(false) }
    Text("Recovery Words", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Write down each recovery word in order and store the backup privately and securely.")
    if (showAll) {
        Card(modifier = Modifier.fillMaxWidth()) {
            SelectionContainer {
                Text(
                    words.mapIndexed { wordIndex, word -> "${wordIndex + 1}. $word" }.joinToString("\n"),
                    modifier = Modifier.padding(24.dp),
                    style = MaterialTheme.typography.bodyLarge,
                )
            }
        }
    } else {
        Text("Word ${index + 1} of ${words.size}")
        Card(modifier = Modifier.fillMaxWidth()) {
            SelectionContainer {
                Text(words.getOrElse(index) { "" }, modifier = Modifier.padding(24.dp), style = MaterialTheme.typography.headlineMedium)
            }
        }
    }
    OutlinedButton(onClick = { showAll = !showAll }, modifier = Modifier.fillMaxWidth()) {
        Text(if (showAll) "Show One at a Time" else "Show All")
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
        OutlinedButton(
            onClick = { if (index > 0) index -= 1 },
            enabled = !showAll && index > 0,
            modifier = Modifier.weight(1f),
        ) { Text("Previous") }
        Button(
            onClick = { if (showAll || index + 1 >= words.size) onDone() else index += 1 },
            enabled = true,
            modifier = Modifier.weight(1f),
        ) { Text(if (showAll || index + 1 >= words.size) "Continue" else "Next") }
    }
    OutlinedButton(onClick = onAdvanced, modifier = Modifier.fillMaxWidth()) { Text("Advanced backup") }
}

@Composable
internal fun MainMenuScreen(onConnect: () -> Unit, onScan: () -> Unit, onWallet: () -> Unit, onSettings: () -> Unit) {
    Text("Main Menu", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            HomeMenuButton("Connect", R.drawable.icon_connect_companion_56, onConnect, Modifier.weight(1f))
            HomeMenuButton("Scan QR", R.drawable.icon_send, onScan, Modifier.weight(1f))
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            HomeMenuButton("Wallet", R.drawable.icon_wallet_56, onWallet, Modifier.weight(1f))
            HomeMenuButton("Settings", R.drawable.icon_settings, onSettings, Modifier.weight(1f))
        }
    }
}

@Composable
private fun HomeMenuButton(label: String, icon: Int, onClick: () -> Unit, modifier: Modifier = Modifier) {
    OutlinedButton(onClick = onClick, modifier = modifier.height(120.dp)) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Image(painter = painterResource(icon), contentDescription = null, modifier = Modifier.size(56.dp))
            Text(label, fontWeight = FontWeight.Bold)
        }
    }
}

@Composable
internal fun ConnectScreen(kpub: String, onBack: () -> Unit) {
    Text("Connect", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    SelectionContainer { Text(kpub, style = MaterialTheme.typography.bodySmall) }
    QrCodeView(kpub.toByteArray(Charsets.UTF_8), Modifier.fillMaxWidth())
    Text("Scan this public account in KasKold Companion. It contains no private keys.")
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun WalletMenuScreen(
    onReceive: () -> Unit, onBackup: () -> Unit, onRecovery: () -> Unit, onDetails: () -> Unit,
    onSwitch: () -> Unit, onMultisig: () -> Unit, onAdvanced: () -> Unit, onBack: () -> Unit,
) {
    Text("Wallet", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onReceive, modifier = Modifier.fillMaxWidth()) { Text("Receive") }
    OutlinedButton(onClick = onBackup, modifier = Modifier.fillMaxWidth()) { Text("Backup") }
    OutlinedButton(onClick = onRecovery, modifier = Modifier.fillMaxWidth()) { Text("Recovery") }
    OutlinedButton(onClick = onDetails, modifier = Modifier.fillMaxWidth()) { Text("Wallet Details") }
    OutlinedButton(onClick = onSwitch, modifier = Modifier.fillMaxWidth()) { Text("Switch / Add Wallet") }
    OutlinedButton(onClick = onMultisig, modifier = Modifier.fillMaxWidth()) { Text("Multisig") }
    OutlinedButton(onClick = onAdvanced, modifier = Modifier.fillMaxWidth()) { Text("Advanced") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun BackupMethodsScreen(onWords: () -> Unit, onSeedQr: () -> Unit, onEncryptedSd: () -> Unit, onAdvanced: () -> Unit, onBack: () -> Unit) {
    Text("Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onWords, modifier = Modifier.fillMaxWidth()) { Text("View Words") }
    OutlinedButton(onClick = onSeedQr, modifier = Modifier.fillMaxWidth()) { Text("SeedQR Backup") }
    OutlinedButton(onClick = onEncryptedSd, modifier = Modifier.fillMaxWidth()) { Text("Encrypted SD Card") }
    Button(onClick = onAdvanced, modifier = Modifier.fillMaxWidth()) { Text("Advanced") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun RecoveryWordsScreen(phrase: String, onBack: () -> Unit) {
    Text("Recovery Words", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Anyone with these words can control this wallet. Keep them private and offline.")
    Card(modifier = Modifier.fillMaxWidth()) { SelectionContainer { Text(phrase, modifier = Modifier.padding(18.dp)) } }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun SeedQrScreen(title: String, payload: ByteArray, onBack: () -> Unit) {
    Text(title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("This QR contains wallet recovery material. Keep it private.")
    QrCodeView(payload, Modifier.fillMaxWidth())
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun AdvancedBackupScreen(onCompact: () -> Unit, onPlain: () -> Unit, onStego: () -> Unit, onXprv: () -> Unit, onExportKey: () -> Unit, onBack: () -> Unit) {
    Text("Advanced Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onCompact, modifier = Modifier.fillMaxWidth()) { Text("Compact SeedQR") }
    OutlinedButton(onClick = onPlain, modifier = Modifier.fillMaxWidth()) { Text("Plain-text SeedQR") }
    OutlinedButton(onClick = onStego, modifier = Modifier.fillMaxWidth()) { Text("Steganographic") }
    OutlinedButton(onClick = onXprv, modifier = Modifier.fillMaxWidth()) { Text("XPrv Backup") }
    OutlinedButton(onClick = onExportKey, modifier = Modifier.fillMaxWidth()) { Text("Export Key") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun SecretTextScreen(state: VaultScreenState.SecretText, onBack: () -> Unit) {
    Text(state.title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(state.warning)
    Card(modifier = Modifier.fillMaxWidth()) { SelectionContainer { Text(state.value, modifier = Modifier.padding(18.dp)) } }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun ExportKeyScreen(onShow: (Int) -> Unit, onBack: () -> Unit) {
    var indexText by remember { mutableStateOf("0") }
    OutlinedTextField(value = indexText, onValueChange = { if (it.all(Char::isDigit)) indexText = it }, label = { Text("Address index") }, modifier = Modifier.fillMaxWidth())
    Button(onClick = { indexText.toIntOrNull()?.let(onShow) }, enabled = indexText.toIntOrNull()?.let { it in 0..65535 } == true, modifier = Modifier.fillMaxWidth()) { Text("Show Private Key") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun WalletDetailsScreen(kpub: String, onBack: () -> Unit) {
    Text("Wallet Details", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    SelectionContainer { Text(kpub, style = MaterialTheme.typography.bodySmall) }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}
