package com.kaskold.vault

import android.app.Activity
import android.app.KeyguardManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.IconButton
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

class MainActivity : ComponentActivity() {
    private var pendingVaultAuthentication: (() -> Unit)? = null
    private var pendingVaultAuthenticationFailure: ((String) -> Unit)? = null
    private val credentialLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        val success = pendingVaultAuthentication
        val failure = pendingVaultAuthenticationFailure
        pendingVaultAuthentication = null
        pendingVaultAuthenticationFailure = null
        if (result.resultCode == Activity.RESULT_OK) success?.invoke()
        else failure?.invoke("Vault authentication was cancelled or failed.")
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { KasKoldVaultApp(this) }
    }

    fun requestVaultReauthentication(onSuccess: () -> Unit, onFailure: (String) -> Unit) {
        val keyguard = getSystemService(KeyguardManager::class.java)
        if (keyguard == null || !keyguard.isDeviceSecure) {
            onFailure("A secure device lock is required to unlock a saved KasKold Vault.")
            return
        }
        val intent = keyguard.createConfirmDeviceCredentialIntent(
            "Unlock KasKold Vault",
            "Authenticate with your device credential to unlock private key custody.",
        )
        if (intent == null) {
            onFailure("Device reauthentication is unavailable.")
            return
        }
        pendingVaultAuthentication = onSuccess
        pendingVaultAuthenticationFailure = onFailure
        credentialLauncher.launch(intent)
    }
}

@Composable
private fun KasKoldVaultApp(activity: MainActivity) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val controller = remember { VaultController(context.applicationContext) }
    var state by remember { mutableStateOf<VaultScreenState>(VaultScreenState.Locked) }
    DisposableEffect(controller) { onDispose { controller.close() } }

    MaterialTheme {
        Column(
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Image(
                        painter = painterResource(R.drawable.kaskold_wordmark),
                        contentDescription = "KasKold Vault",
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text("Air-gapped signer · no Internet permission", style = MaterialTheme.typography.bodyMedium)
                    Text("v 2.0.0", style = MaterialTheme.typography.bodySmall)
                }
                if (controller.homeShortcutVisible(state)) {
                    IconButton(onClick = { state = controller.mainMenu() }) {
                        Image(
                            painter = painterResource(R.drawable.icon_home_24),
                            contentDescription = "Home",
                            modifier = Modifier.size(24.dp),
                        )
                    }
                }
            }
            when (val current = state) {
                VaultScreenState.Locked -> LockedScreen(
                    onUnlock = {
                        activity.requestVaultReauthentication(
                            onSuccess = { state = controller.unlockPersisted() },
                            onFailure = { message -> state = VaultScreenState.Error(message, VaultScreenState.Locked) },
                        )
                    },
                    onCreate = { state = controller.beginCreation() },
                    onRestore = { state = controller.showRestore() },
                )
                is VaultScreenState.CreationName -> CreationNameScreen(
                    current,
                    onContinue = { name -> state = controller.creationSetName(name) },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationWordCount -> CreationWordCountScreen(
                    onSelect = { words -> state = controller.creationSetWordCount(words) },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationDiceChoice -> CreationDiceChoiceScreen(
                    onChoose = { add -> state = controller.creationChooseDice(add) },
                    onCancel = { state = controller.cancelCreation() },
                )
                is VaultScreenState.CreationDiceCount -> CreationDiceCountScreen(
                    current,
                    onSelect = { target -> state = controller.creationSetDiceTarget(target) },
                    onCancel = { state = controller.cancelCreation() },
                )
                is VaultScreenState.CreationDiceRoll -> CreationDiceRollScreen(
                    current,
                    onRoll = { value -> state = controller.creationAddDice(value) },
                    onUndo = { state = controller.creationUndoDice() },
                    onReset = { state = controller.creationResetDice() },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationTouchChoice -> CreationTouchChoiceScreen(
                    onChoose = { add -> state = controller.creationChooseTouch(add) },
                    onCancel = { state = controller.cancelCreation() },
                )
                is VaultScreenState.CreationTouch -> CreationTouchScreen(
                    current,
                    onSample = { time, x, y -> state = controller.creationAddTouch(time, x, y) },
                    onReset = { state = controller.creationResetTouch() },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationPassphraseChoice -> CreationPassphraseChoiceScreen(
                    onChoose = { use -> state = controller.creationChoosePassphrase(use) },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationPassphrase -> CreationPassphraseScreen(
                    onSubmit = { value, confirmation -> state = controller.creationSubmitPassphrase(value, confirmation) },
                    onCancel = { state = controller.cancelCreation() },
                )
                VaultScreenState.CreationRecoveryAcknowledgement -> CreationRecoveryAcknowledgementScreen(
                    onDone = { state = controller.acknowledgeCreationBackup() },
                )
                VaultScreenState.CreationStorageFinalize -> CreationStorageFinalizeScreen(
                    onSave = { state = controller.creationStorageChoice(true) },
                    onSession = { state = controller.creationStorageChoice(false) },
                )
                VaultScreenState.CreationStorageProtection -> CreationStorageProtectionScreen(
                    onProtect = { state = controller.creationUseDeviceProtection() },
                    onSession = { state = controller.creationNoProtection() },
                )
                VaultScreenState.MainMenu -> MainMenuScreen(
                    onConnect = { state = controller.connect() },
                    onScan = { state = controller.beginScan() },
                    onWallet = { state = controller.walletMenu() },
                    onSettings = { state = controller.settings() },
                )
                is VaultScreenState.Connect -> ConnectScreen(current.kpub) { state = controller.mainMenu() }
                VaultScreenState.WalletMenu -> WalletMenuScreen(
                    onReceive = { state = controller.receive() },
                    onBackup = { state = controller.backupMethods() },
                    onRecovery = { state = controller.recoveryMenu() },
                    onDetails = { state = controller.walletDetails() },
                    onSwitch = { state = controller.walletInventory() },
                    onMultisig = { state = controller.multisigMenu() },
                    onAdvanced = { state = controller.walletAdvanced() },
                    onBack = { state = controller.mainMenu() },
                )
                is VaultScreenState.Backup -> BackupScreen(
                    current,
                    onAdvanced = { state = controller.advancedBackup(current) },
                    onDone = { state = controller.finishBackup() },
                )
                VaultScreenState.BackupMethods -> BackupMethodsScreen(
                    onWords = { state = controller.revealWords() },
                    onSeedQr = { state = controller.seedQr(false, "SeedQR Backup", VaultScreenState.BackupMethods) },
                    onEncryptedSd = { state = controller.fileWorkflow(NativeFileWorkflow.PortableExport, VaultScreenState.BackupMethods) },
                    onAdvanced = { state = controller.advancedBackup(VaultScreenState.BackupMethods) },
                    onBack = { state = controller.walletMenu() },
                )
                is VaultScreenState.RecoveryWords -> RecoveryWordsScreen(current.phrase) { state = controller.backupMethods() }
                is VaultScreenState.SeedQr -> SeedQrScreen(current.title, current.payload) { state = controller.setState(current.previous) }
                is VaultScreenState.AdvancedBackup -> AdvancedBackupScreen(
                    onCompact = { state = controller.seedQr(true, "Compact SeedQR", current) },
                    onPlain = { state = controller.plainSeedQr(current) },
                    onStego = { state = controller.fileWorkflow(NativeFileWorkflow.StegoExport, current) },
                    onXprv = { state = controller.xprvBackup() },
                    onExportKey = { state = controller.setState(VaultScreenState.ExportKey(current)) },
                    onBack = { state = controller.setState(current.previous) },
                )
                VaultScreenState.XprvExport -> XprvExportMenuScreen(
                    onShowQr = { state = controller.showXprvQr() },
                    onEncrypt = { state = controller.fileWorkflow(NativeFileWorkflow.PortableXprvExport, VaultScreenState.XprvExport) },
                    onBack = { state = controller.setState(VaultScreenState.AdvancedBackup(VaultScreenState.BackupMethods)) },
                )
                is VaultScreenState.SecretText -> SecretTextScreen(current) { state = controller.setState(current.previous) }
                is VaultScreenState.ExportKey -> ExportKeyScreen(
                    onShow = { index -> state = controller.exportReceiveKey(index) },
                    onBack = { state = controller.setState(current.previous) },
                )
                is VaultScreenState.WalletDetails -> WalletDetailsScreen(current.kpub) { state = controller.walletMenu() }
                VaultScreenState.WalletAdvanced -> AdvancedToolsMenuScreen(
                    onBip85 = { state = controller.tool(NativeTool.Bip85, VaultScreenState.WalletAdvanced) },
                    onSignMessage = { state = controller.tool(NativeTool.SignMessage, VaultScreenState.WalletAdvanced) },
                    onCommitSecret = { state = controller.tool(NativeTool.CommitSecret, VaultScreenState.WalletAdvanced) },
                    onDecryptSecret = { state = controller.tool(NativeTool.DecryptSecret, VaultScreenState.WalletAdvanced) },
                    onBack = { state = controller.walletMenu() },
                )
                VaultScreenState.Settings -> SettingsWorkflowMenuScreen(
                    onSecurity = { state = controller.tool(NativeTool.SigningPolicy, VaultScreenState.Settings) },
                    onStorage = { state = controller.info("Storage", "Wallet custody is sealed by the shared Rust KHV format and wrapped with an Android Keystore key. Portable encrypted backup is available from Wallet → Backup.", VaultScreenState.Settings) },
                    onAbout = { state = controller.info("About", "KasKold Vault 2.0.0 · native Rust custody runtime · offline application surface.", VaultScreenState.Settings) },
                    onLock = { state = controller.lock() },
                    onDelete = { state = controller.deleteWallet() },
                    onBack = { state = controller.mainMenu() },
                )
                VaultScreenState.RecoveryMenu -> RecoveryMenuScreen(
                    onWalletBackup = { state = controller.fileWorkflow(NativeFileWorkflow.WalletBackup, VaultScreenState.RecoveryMenu) },
                    onTransaction = { state = controller.fileWorkflow(NativeFileWorkflow.Transaction, VaultScreenState.RecoveryMenu) },
                    onKpub = { state = controller.fileWorkflow(NativeFileWorkflow.Kpub, VaultScreenState.RecoveryMenu) },
                    onMultisigAddress = { state = controller.fileWorkflow(NativeFileWorkflow.MultisigAddress, VaultScreenState.RecoveryMenu) },
                    onCovenant = { state = controller.fileWorkflow(NativeFileWorkflow.CovenantRestore, VaultScreenState.RecoveryMenu) },
                    onRawKey = { state = controller.tool(NativeTool.ImportRawKey, VaultScreenState.RecoveryMenu) },
                    onBack = { state = controller.walletMenu() },
                )
                VaultScreenState.MultisigMenu -> MultisigMenuScreen(
                    onCreate = { state = controller.tool(NativeTool.MultisigCreate, VaultScreenState.MultisigMenu) },
                    onDescriptors = { state = controller.tool(NativeTool.MultisigImport, VaultScreenState.MultisigMenu) },
                    onKpub = { state = controller.multisigKpub() },
                    onBack = { state = controller.walletMenu() },
                )
                is VaultScreenState.Receive -> ReceiveWorkflowScreen(
                    current,
                    onRefresh = { change, index -> state = controller.receive(change, index) },
                    onBack = { state = controller.walletMenu() },
                )
                is VaultScreenState.WalletInventory -> WalletInventoryScreen(
                    wallets = current.wallets,
                    onSwitch = { index -> state = controller.switchWallet(index) },
                    onCreate = { state = controller.beginCreation(true, current) },
                    onRestore = { state = controller.showRestore(true, current) },
                    onImportXprv = { state = controller.tool(NativeTool.ImportXprv, current) },
                    onImportRaw = { state = controller.tool(NativeTool.ImportRawKey, current) },
                    onBack = { state = controller.walletMenu() },
                )
                is VaultScreenState.ToolForm -> ToolFormScreen(
                    current,
                    onSubmit = { values -> state = controller.submitTool(current.tool, values, current.previous) },
                    onBack = { state = controller.setState(current.previous) },
                )
                is VaultScreenState.FileWorkflow -> FileWorkflowScreen(
                    current,
                    onProcess = { bytes, password -> state = controller.importFile(current.workflow, bytes, password, current.previous) },
                    onBack = { state = controller.setState(current.previous) },
                )
                is VaultScreenState.ExportFile -> ExportFileScreen(
                    current,
                    onDone = { current.data.fill(0); state = controller.setState(current.previous) },
                    onBack = { current.data.fill(0); state = controller.setState(current.previous) },
                )
                is VaultScreenState.Info -> InfoWorkflowScreen(current) { state = controller.setState(current.previous) }
                is VaultScreenState.Restore -> RestoreScreen(
                    initial = current,
                    onCancel = { state = controller.setState(current.previous) },
                    onRestore = { phrase, passphrase -> state = controller.restore(phrase, passphrase, current.addToInventory, current.previous) },
                )
                is VaultScreenState.Scanning -> ScanningScreen(
                    current,
                    onFrame = { frame -> state = controller.acceptFrame(frame) },
                    onCancel = { state = controller.reject() },
                    onError = { message -> state = controller.setState(VaultScreenState.Error(message, current)) },
                )
                is VaultScreenState.Review -> ReviewScreen(
                    current.review,
                    onApprove = { state = controller.approve() },
                    onReject = { state = controller.reject() },
                )
                is VaultScreenState.Response -> ResponseScreen(current.frames) { state = controller.finishResponse() }
                is VaultScreenState.Error -> ErrorScreen(
                    current.message,
                    onBack = { state = controller.setState(current.previous) },
                )
            }
        }
    }
}

@Composable
private fun LockedScreen(onUnlock: () -> Unit, onCreate: () -> Unit, onRestore: () -> Unit) {
    Text("Vault is locked. Wallet secrets are held only by the native Rust signing runtime.")
    Button(onClick = onUnlock, modifier = Modifier.fillMaxWidth()) { Text("Unlock saved wallet") }
    Button(onClick = onCreate, modifier = Modifier.fillMaxWidth()) { Text("Create Wallet") }
    OutlinedButton(onClick = onRestore, modifier = Modifier.fillMaxWidth()) { Text("Restore wallet") }
}

@Composable
private fun BackupScreen(state: VaultScreenState.Backup, onAdvanced: () -> Unit, onDone: () -> Unit) {
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
private fun MainMenuScreen(onConnect: () -> Unit, onScan: () -> Unit, onWallet: () -> Unit, onSettings: () -> Unit) {
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
private fun ConnectScreen(kpub: String, onBack: () -> Unit) {
    Text("Connect", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    SelectionContainer { Text(kpub, style = MaterialTheme.typography.bodySmall) }
    QrCodeView(kpub.toByteArray(Charsets.UTF_8), Modifier.fillMaxWidth())
    Text("Scan this public account in KasKold Companion. It contains no private keys.")
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun WalletMenuScreen(
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
private fun BackupMethodsScreen(onWords: () -> Unit, onSeedQr: () -> Unit, onEncryptedSd: () -> Unit, onAdvanced: () -> Unit, onBack: () -> Unit) {
    Text("Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onWords, modifier = Modifier.fillMaxWidth()) { Text("View Words") }
    OutlinedButton(onClick = onSeedQr, modifier = Modifier.fillMaxWidth()) { Text("SeedQR Backup") }
    OutlinedButton(onClick = onEncryptedSd, modifier = Modifier.fillMaxWidth()) { Text("Encrypted SD Card") }
    Button(onClick = onAdvanced, modifier = Modifier.fillMaxWidth()) { Text("Advanced") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun RecoveryWordsScreen(phrase: String, onBack: () -> Unit) {
    Text("Recovery Words", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("Anyone with these words can control this wallet. Keep them private and offline.")
    Card(modifier = Modifier.fillMaxWidth()) { SelectionContainer { Text(phrase, modifier = Modifier.padding(18.dp)) } }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun SeedQrScreen(title: String, payload: ByteArray, onBack: () -> Unit) {
    Text(title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("This QR contains wallet recovery material. Keep it private.")
    QrCodeView(payload, Modifier.fillMaxWidth())
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun AdvancedBackupScreen(onCompact: () -> Unit, onPlain: () -> Unit, onStego: () -> Unit, onXprv: () -> Unit, onExportKey: () -> Unit, onBack: () -> Unit) {
    Text("Advanced Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onCompact, modifier = Modifier.fillMaxWidth()) { Text("Compact SeedQR") }
    OutlinedButton(onClick = onPlain, modifier = Modifier.fillMaxWidth()) { Text("Plain-text SeedQR") }
    OutlinedButton(onClick = onStego, modifier = Modifier.fillMaxWidth()) { Text("Steganographic") }
    OutlinedButton(onClick = onXprv, modifier = Modifier.fillMaxWidth()) { Text("XPrv Backup") }
    OutlinedButton(onClick = onExportKey, modifier = Modifier.fillMaxWidth()) { Text("Export Key") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun SecretTextScreen(state: VaultScreenState.SecretText, onBack: () -> Unit) {
    Text(state.title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(state.warning)
    Card(modifier = Modifier.fillMaxWidth()) { SelectionContainer { Text(state.value, modifier = Modifier.padding(18.dp)) } }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun ExportKeyScreen(onShow: (Int) -> Unit, onBack: () -> Unit) {
    var indexText by remember { mutableStateOf("0") }
    OutlinedTextField(value = indexText, onValueChange = { if (it.all(Char::isDigit)) indexText = it }, label = { Text("Address index") }, modifier = Modifier.fillMaxWidth())
    Button(onClick = { indexText.toIntOrNull()?.let(onShow) }, enabled = indexText.toIntOrNull()?.let { it in 0..65535 } == true, modifier = Modifier.fillMaxWidth()) { Text("Show Private Key") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun WalletDetailsScreen(kpub: String, onBack: () -> Unit) {
    Text("Wallet Details", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    SelectionContainer { Text(kpub, style = MaterialTheme.typography.bodySmall) }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
private fun ScanningScreen(state: VaultScreenState.Scanning, onFrame: (ByteArray) -> Unit, onCancel: () -> Unit, onError: (String) -> Unit) {
    Text(if (state.total > 0) "Scanning request · ${state.received}/${state.total} frames" else "Scanning signing request")
    QrScannerView(onFrame = onFrame, onError = onError)
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Cancel") }
}

@Composable
private fun ReviewScreen(review: VaultReview, onApprove: () -> Unit, onReject: () -> Unit) {
    Text("Review transaction", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    ReviewRow("Network", review.network)
    ReviewRow("Inputs", review.inputCount.toString())
    ReviewRow("Outputs", review.outputCount.toString())
    ReviewRow("Input total (sompi)", review.inputTotal)
    ReviewRow("Output total (sompi)", review.outputTotal)
    ReviewRow("Fee (sompi)", review.fee)
    review.inputs.forEach { input ->
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Input ${input.index + 1} · ${input.scriptType}", fontWeight = FontWeight.SemiBold)
                Text("Outpoint: ${input.outpoint}", style = MaterialTheme.typography.bodySmall)
                Text("Amount: ${input.amount} sompi")
                Text("Address: ${input.address ?: "Unavailable"}", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
    review.outputs.forEach { output ->
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Output ${output.index + 1} · ${output.ownership}", fontWeight = FontWeight.SemiBold)
                Text("Amount: ${output.amount} sompi")
                Text("Address: ${output.address ?: "Unavailable"}", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
    Text("Approval signs exactly the scanned, validated KSPT. Advanced hidden transaction semantics are rejected. Scanning alone never signs.")
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        OutlinedButton(onClick = onReject, modifier = Modifier.weight(1f)) { Text("Reject") }
        Button(onClick = onApprove, modifier = Modifier.weight(1f)) { Text("Approve & sign") }
    }
}

@Composable
private fun ReviewRow(label: String, value: String) {
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label)
        Text(value, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun ResponseScreen(frames: List<ByteArray>, onDone: () -> Unit) {
    var index by remember { mutableIntStateOf(0) }
    LaunchedEffect(frames.size) {
        while (frames.size > 1) {
            delay(700)
            index = (index + 1) % frames.size
        }
    }
    Text("Signed response", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    if (frames.isNotEmpty()) {
        QrCodeView(frames[index], Modifier.fillMaxWidth())
        Text("Frame ${index + 1}/${frames.size}")
        Text("Scan the response with KasKold Companion.")
    }
    Spacer(Modifier.height(4.dp))
    Button(onClick = onDone, modifier = Modifier.fillMaxWidth()) { Text("Done") }
}

@Composable
private fun ErrorScreen(message: String, onBack: () -> Unit) {
    Text("Vault operation failed", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(message)
    Button(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}
