package com.kaskold.vault

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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp

@Composable
internal fun KasKoldVaultApp(activity: MainActivity) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val controller = remember { VaultController(context.applicationContext) }
    var state by remember { mutableStateOf<VaultScreenState>(VaultScreenState.Locked) }
    DisposableEffect(controller) { onDispose { controller.close() } }
    LaunchedEffect(state) {
        if (controller.persistencePending && state !is VaultScreenState.Error) {
            activity.requestVaultReauthentication(
                onSuccess = { state = controller.completePendingPersistence() },
                onFailure = { message ->
                    state = VaultScreenState.Error("$message The wallet is not saved yet.", controller.state)
                },
            )
        }
    }

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
