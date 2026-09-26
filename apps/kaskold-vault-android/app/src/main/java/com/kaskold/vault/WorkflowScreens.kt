package com.kaskold.vault

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import java.io.ByteArrayOutputStream

private const val MAX_NATIVE_FILE_BYTES = 32 * 1024 * 1024

@Composable
internal fun ReceiveWorkflowScreen(
    state: VaultScreenState.Receive,
    onRefresh: (Boolean, Int) -> Unit,
    onBack: () -> Unit,
) {
    var index by remember(state.index) { mutableStateOf(state.index.toString()) }
    Text("Receive", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(if (state.change) "Change address" else "Receive address")
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(18.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(state.address, style = MaterialTheme.typography.bodySmall)
            QrCodeView(state.address.toByteArray(Charsets.UTF_8), Modifier.fillMaxWidth())
        }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = { onRefresh(!state.change, state.index) }, modifier = Modifier.weight(1f)) {
            Text(if (state.change) "Receive chain" else "Change chain")
        }
        OutlinedButton(onClick = { onRefresh(state.change, (state.index - 1).coerceAtLeast(0)) }, modifier = Modifier.weight(1f)) {
            Text("Previous")
        }
        OutlinedButton(onClick = { onRefresh(state.change, state.index + 1) }, modifier = Modifier.weight(1f)) {
            Text("Next")
        }
    }
    OutlinedTextField(
        value = index,
        onValueChange = { value -> if (value.all(Char::isDigit)) index = value },
        label = { Text("Address index") },
        modifier = Modifier.fillMaxWidth(),
    )
    Button(
        onClick = { index.toIntOrNull()?.let { onRefresh(state.change, it) } },
        enabled = index.toLongOrNull()?.let { it in 0..UInt.MAX_VALUE.toLong() } == true,
        modifier = Modifier.fillMaxWidth(),
    ) { Text("Show Address") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun RecoveryMenuScreen(
    onWalletBackup: () -> Unit,
    onTransaction: () -> Unit,
    onKpub: () -> Unit,
    onMultisigAddress: () -> Unit,
    onCovenant: () -> Unit,
    onRawKey: () -> Unit,
    onBack: () -> Unit,
) {
    Text("Recovery", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    val entries = listOf(
        "Seed / XPrv Backup" to onWalletBackup,
        "Transaction" to onTransaction,
        "kpub (Watch-Only)" to onKpub,
        "Multisig Address" to onMultisigAddress,
        "Covenant Restore" to onCovenant,
        "Import Raw Key" to onRawKey,
    )
    entries.forEach { (label, action) ->
        OutlinedButton(onClick = action, modifier = Modifier.fillMaxWidth()) { Text(label) }
    }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun MultisigMenuScreen(onCreate: () -> Unit, onDescriptors: () -> Unit, onKpub: () -> Unit, onBack: () -> Unit) {
    Text("Multisig", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Button(onClick = onCreate, modifier = Modifier.fillMaxWidth()) { Text("Create Multisig") }
    OutlinedButton(onClick = onDescriptors, modifier = Modifier.fillMaxWidth()) { Text("Descriptors") }
    OutlinedButton(onClick = onKpub, modifier = Modifier.fillMaxWidth()) { Text("kpub Multisig QR") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun WalletInventoryScreen(
    wallets: List<NativeWalletSummary>,
    onSwitch: (Int) -> Unit,
    onCreate: () -> Unit,
    onRestore: () -> Unit,
    onImportXprv: () -> Unit,
    onImportRaw: () -> Unit,
    onBack: () -> Unit,
) {
    Text("Switch / Add Wallet", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    wallets.forEach { wallet ->
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("${wallet.name}${if (wallet.active) " · active" else ""}", fontWeight = FontWeight.SemiBold)
                Text(wallet.kind)
                wallet.kpub?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
                if (!wallet.active) {
                    Button(onClick = { onSwitch(wallet.index) }) { Text("Switch") }
                }
            }
        }
    }
    Text("Add Wallet", fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onCreate, modifier = Modifier.fillMaxWidth()) { Text("Create Wallet") }
    OutlinedButton(onClick = onRestore, modifier = Modifier.fillMaxWidth()) { Text("Restore Words") }
    OutlinedButton(onClick = onImportXprv, modifier = Modifier.fillMaxWidth()) { Text("Import XPrv") }
    OutlinedButton(onClick = onImportRaw, modifier = Modifier.fillMaxWidth()) { Text("Import Raw Key") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

private data class ToolField(val label: String, val secret: Boolean = false, val multiline: Boolean = false)

private fun toolFields(tool: NativeTool): List<ToolField> = when (tool) {
    NativeTool.Receive -> listOf(ToolField("Chain (receive/change)"), ToolField("Address index"))
    NativeTool.ImportRawKey -> listOf(ToolField("Raw private key (64 hex)", secret = true))
    NativeTool.ImportXprv -> listOf(ToolField("Account XPrv", secret = true, multiline = true))
    NativeTool.MultisigCreate -> listOf(ToolField("Threshold (M)"), ToolField("Other participant kpubs — one per line", multiline = true))
    NativeTool.MultisigImport -> listOf(ToolField("Multisig descriptor", multiline = true))
    NativeTool.Bip85 -> listOf(ToolField("Word count (12/24)"), ToolField("Child index"))
    NativeTool.SignMessage -> listOf(ToolField("Message", multiline = true))
    NativeTool.CommitSecret -> listOf(ToolField("Secret", secret = true, multiline = true))
    NativeTool.DecryptSecret -> listOf(ToolField("Encrypted payload hex", secret = true, multiline = true))
    NativeTool.SigningPolicy -> listOf(ToolField("No-sign-before UTC"), ToolField("Weekly windows", multiline = true))
}

@Composable
internal fun ToolFormScreen(state: VaultScreenState.ToolForm, onSubmit: (List<String>) -> Unit, onBack: () -> Unit) {
    val fields = toolFields(state.tool)
    val values = remember(state.tool) { MutableList(fields.size) { "" } }
    var revision by remember(state.tool) { mutableStateOf(0) }
    Text(toolTitle(state.tool), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    fields.forEachIndexed { index, field ->
        OutlinedTextField(
            value = values[index],
            onValueChange = { values[index] = it; revision += 1 },
            label = { Text(field.label) },
            visualTransformation = if (field.secret) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
            minLines = if (field.multiline) 3 else 1,
            modifier = Modifier.fillMaxWidth(),
        )
    }
    @Suppress("UNUSED_VARIABLE") val keepRevision = revision
    Button(onClick = { onSubmit(values.toList()) }, modifier = Modifier.fillMaxWidth()) { Text("Continue") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

private fun toolTitle(tool: NativeTool): String = when (tool) {
    NativeTool.Receive -> "Receive"
    NativeTool.ImportRawKey -> "Import Raw Key"
    NativeTool.ImportXprv -> "Import XPrv"
    NativeTool.MultisigCreate -> "Create Multisig"
    NativeTool.MultisigImport -> "Import Multisig"
    NativeTool.Bip85 -> "BIP85 Child Wallet"
    NativeTool.SignMessage -> "Sign Message"
    NativeTool.CommitSecret -> "Commit Secret"
    NativeTool.DecryptSecret -> "Decrypt Secret"
    NativeTool.SigningPolicy -> "Security"
}

@Composable
internal fun FileWorkflowScreen(
    state: VaultScreenState.FileWorkflow,
    onProcess: (ByteArray, String) -> Unit,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    var password by remember(state.workflow) { mutableStateOf("") }
    val needsPassword = state.workflow in setOf(
        NativeFileWorkflow.RecoveryMaterial,
        NativeFileWorkflow.WalletBackup,
        NativeFileWorkflow.PortableBackup,
        NativeFileWorkflow.StegoBackup,
        NativeFileWorkflow.PortableExport,
        NativeFileWorkflow.PortableXprvExport,
        NativeFileWorkflow.StegoExport,
    )
    val needsFile = state.workflow !in setOf(NativeFileWorkflow.PortableExport, NativeFileWorkflow.PortableXprvExport)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) {
            var bytes = ByteArray(0)
            try {
                bytes = context.contentResolver.openInputStream(uri)?.use(::readBounded) ?: error("Unable to read selected file")
                onProcess(bytes, password)
            } finally {
                bytes.fill(0)
                password = ""
            }
        }
    }
    Text(fileWorkflowTitle(state.workflow), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(fileWorkflowHelp(state.workflow))
    if (needsPassword) {
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text(if (state.workflow == NativeFileWorkflow.RecoveryMaterial) "Optional BIP39 passphrase" else "Backup password") },
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
        )
    }
    if (needsFile) {
        Button(onClick = { launcher.launch(arrayOf("*/*")) }, modifier = Modifier.fillMaxWidth()) { Text("Choose File") }
    } else {
        Button(onClick = { onProcess(ByteArray(0), password); password = "" }, modifier = Modifier.fillMaxWidth()) { Text("Create Backup") }
    }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

private fun fileWorkflowTitle(workflow: NativeFileWorkflow): String = when (workflow) {
    NativeFileWorkflow.RecoveryMaterial -> "Recovery Material"
    NativeFileWorkflow.WalletBackup -> "Seed / XPrv Backup"
    NativeFileWorkflow.PortableBackup -> "Encrypted Wallet Backup"
    NativeFileWorkflow.StegoBackup -> "Steganographic Restore"
    NativeFileWorkflow.Transaction -> "Transaction"
    NativeFileWorkflow.Kpub -> "kpub (Watch-Only)"
    NativeFileWorkflow.MultisigAddress -> "Multisig Address"
    NativeFileWorkflow.MultisigDescriptor -> "Multisig Descriptor"
    NativeFileWorkflow.CovenantRestore -> "Covenant Restore"
    NativeFileWorkflow.PortableExport -> "Encrypted Wallet Backup"
    NativeFileWorkflow.PortableXprvExport -> "Encrypted XPrv Backup"
    NativeFileWorkflow.StegoExport -> "Steganographic Backup"
}

private fun fileWorkflowHelp(workflow: NativeFileWorkflow): String = when (workflow) {
    NativeFileWorkflow.StegoExport -> "Choose a baseline JPEG carrier. The shared Rust steganographic codec will embed an authenticated encrypted backup."
    NativeFileWorkflow.Transaction -> "Choose a transaction file. The Rust Vault runtime will parse it and enter the same review state used by QR scanning."
    NativeFileWorkflow.Kpub, NativeFileWorkflow.MultisigAddress, NativeFileWorkflow.MultisigDescriptor, NativeFileWorkflow.CovenantRestore ->
        "Choose the corresponding M5-compatible file. It will be validated by the shared Rust runtime before display or import."
    NativeFileWorkflow.PortableExport, NativeFileWorkflow.PortableXprvExport ->
        "Create a portable authenticated encrypted backup for local file storage. This is not presented as the M5 device-bound SD secret."
    else -> "Choose the M5-compatible recovery/backup material and complete the explicit import operation."
}

@Composable
internal fun ExportFileScreen(state: VaultScreenState.ExportFile, onDone: () -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument(state.mimeType)) { uri ->
        if (uri != null) {
            context.contentResolver.openOutputStream(uri, "w")?.use { it.write(state.data) }
            onDone()
        }
    }
    Text(state.title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("The backup bytes were created inside the shared Rust Vault runtime. Save them to the location you control.")
    Button(onClick = { launcher.launch(state.filename) }, modifier = Modifier.fillMaxWidth()) { Text("Save File") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Cancel") }
}

@Composable
internal fun InfoWorkflowScreen(state: VaultScreenState.Info, onBack: () -> Unit) {
    Text(state.title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(state.body)
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

private fun readBounded(input: java.io.InputStream): ByteArray {
    val output = ByteArrayOutputStream()
    val buffer = ByteArray(8192)
    var total = 0
    while (true) {
        val read = input.read(buffer)
        if (read < 0) break
        total += read
        require(total <= MAX_NATIVE_FILE_BYTES) { "Selected file exceeds the 32 MiB Vault import limit" }
        output.write(buffer, 0, read)
    }
    buffer.fill(0)
    return output.toByteArray()
}

@Composable
internal fun AdvancedToolsMenuScreen(
    onBip85: () -> Unit,
    onSignMessage: () -> Unit,
    onCommitSecret: () -> Unit,
    onDecryptSecret: () -> Unit,
    onBack: () -> Unit,
) {
    Text("Advanced", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    listOf(
        "BIP85 Child Wallet" to onBip85,
        "Sign Message" to onSignMessage,
        "Commit Secret" to onCommitSecret,
        "Decrypt Secret" to onDecryptSecret,
    ).forEach { (label, action) ->
        OutlinedButton(onClick = action, modifier = Modifier.fillMaxWidth()) { Text(label) }
    }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

@Composable
internal fun SettingsWorkflowMenuScreen(
    onSecurity: () -> Unit,
    onStorage: () -> Unit,
    onAbout: () -> Unit,
    onLock: () -> Unit,
    onDelete: () -> Unit,
    onBack: () -> Unit,
) {
    Text("Settings", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    listOf(
        "Security" to onSecurity,
        "Storage" to onStorage,
        "About" to onAbout,
    ).forEach { (label, action) ->
        OutlinedButton(onClick = action, modifier = Modifier.fillMaxWidth()) { Text(label) }
    }
    OutlinedButton(onClick = onLock, modifier = Modifier.fillMaxWidth()) { Text("Lock Vault") }
    OutlinedButton(onClick = onDelete, modifier = Modifier.fillMaxWidth()) { Text("Delete local Vault wallet") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}
@Composable
internal fun XprvExportMenuScreen(onShowQr: () -> Unit, onEncrypt: () -> Unit, onBack: () -> Unit) {
    Text("XPrv Backup", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    OutlinedButton(onClick = onShowQr, modifier = Modifier.fillMaxWidth()) { Text("Show as QR") }
    OutlinedButton(onClick = onEncrypt, modifier = Modifier.fillMaxWidth()) { Text("Encrypt to SD") }
    OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}

