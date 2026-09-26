package com.kaskold.vault

import android.content.Context
import com.kaskold.vault.security.VaultBlobStore
import com.kaskold.vault.security.VaultWrappingKeyStore
import org.json.JSONArray
import org.json.JSONObject

internal data class NativeWalletSummary(
    val index: Int, val name: String, val active: Boolean, val kind: String, val kpub: String?,
)

internal enum class NativeTool {
    Receive, ImportRawKey, ImportXprv, MultisigCreate, MultisigImport, Bip85,
    SignMessage, CommitSecret, DecryptSecret, SigningPolicy,
}

internal enum class NativeFileWorkflow {
    RecoveryMaterial, WalletBackup, PortableBackup, StegoBackup, Transaction, Kpub,
    MultisigAddress, MultisigDescriptor, CovenantRestore, PortableExport, PortableXprvExport, StegoExport,
}

internal data class CreationFlowConfig(
    val diceTargets: List<Int>,
    val touchTarget: Int,
)

private data class TouchEntropySample(val time: Long, val x: Int, val y: Int)

private data class CreationSession(
    val addToInventory: Boolean,
    val previous: VaultScreenState,
    var walletName: String,
    var wordCount: Int = 24,
    val diceRolls: MutableList<Int> = mutableListOf(),
    var diceTarget: Int = 0,
    val touchSamples: MutableList<TouchEntropySample> = mutableListOf(),
    var passphrase: String = "",
)

internal sealed interface VaultScreenState {
    data object Locked : VaultScreenState
    data object MainMenu : VaultScreenState
    data class CreationName(val suggestedName: String) : VaultScreenState
    data object CreationWordCount : VaultScreenState
    data object CreationDiceChoice : VaultScreenState
    data class CreationDiceCount(val targets: List<Int>) : VaultScreenState
    data class CreationDiceRoll(val collected: Int, val target: Int) : VaultScreenState
    data object CreationTouchChoice : VaultScreenState
    data class CreationTouch(val collected: Int, val target: Int) : VaultScreenState
    data object CreationPassphraseChoice : VaultScreenState
    data object CreationPassphrase : VaultScreenState
    data object CreationRecoveryAcknowledgement : VaultScreenState
    data object CreationStorageFinalize : VaultScreenState
    data object CreationStorageProtection : VaultScreenState
    data class Connect(val kpub: String) : VaultScreenState
    data object WalletMenu : VaultScreenState
    data class Backup(val phrase: String) : VaultScreenState
    data object BackupMethods : VaultScreenState
    data class RecoveryWords(val phrase: String) : VaultScreenState
    data class SeedQr(val title: String, val payload: ByteArray, val previous: VaultScreenState) : VaultScreenState
    data class AdvancedBackup(val previous: VaultScreenState) : VaultScreenState
    data object XprvExport : VaultScreenState
    data class SecretText(val title: String, val warning: String, val value: String, val previous: VaultScreenState) : VaultScreenState
    data class ExportKey(val previous: VaultScreenState) : VaultScreenState
    data class WalletDetails(val kpub: String) : VaultScreenState
    data object WalletAdvanced : VaultScreenState
    data object Settings : VaultScreenState
    data object RecoveryMenu : VaultScreenState
    data object MultisigMenu : VaultScreenState
    data class Receive(val address: String, val change: Boolean, val index: Int) : VaultScreenState
    data class WalletInventory(val wallets: List<NativeWalletSummary>) : VaultScreenState
    data class ToolForm(val tool: NativeTool, val previous: VaultScreenState) : VaultScreenState
    data class FileWorkflow(val workflow: NativeFileWorkflow, val previous: VaultScreenState) : VaultScreenState
    data class ExportFile(
        val title: String, val filename: String, val mimeType: String, val data: ByteArray, val previous: VaultScreenState,
    ) : VaultScreenState
    data class Info(val title: String, val body: String, val previous: VaultScreenState) : VaultScreenState
    data class Restore(
        val phrase: String = "", val passphrase: String = "", val addToInventory: Boolean = false,
        val previous: VaultScreenState = Locked,
    ) : VaultScreenState
    data class Scanning(val received: Int = 0, val total: Int = 0) : VaultScreenState
    data class Review(val review: VaultReview) : VaultScreenState
    data class Response(val frames: List<ByteArray>) : VaultScreenState
    data class Error(val message: String, val previous: VaultScreenState) : VaultScreenState
}

internal class VaultController(context: Context) : AutoCloseable {
    private val runtime = RuntimeBridge()
    private val blobStore = VaultBlobStore(context.applicationContext)
    private val keyStore = VaultWrappingKeyStore(context.applicationContext)
    private val creationFlow = loadCreationFlow()
    private var creationSession: CreationSession? = null
    private var automaticPersistence = true
    private var homeReached = false
    var state: VaultScreenState = VaultScreenState.Locked
        private set

    fun unlockPersisted(): VaultScreenState = guarded(state) {
        val sealed = blobStore.readSealedWallet() ?: return@guarded VaultScreenState.Locked
        val key = keyStore.loadOrCreate()
        try {
            runtime.unlockSealedWallet(sealed, key)
            automaticPersistence = true
            VaultScreenState.MainMenu
        } finally {
            key.fill(0)
            sealed.fill(0)
        }
    }

    fun beginCreation(
        addToInventory: Boolean = false,
        previous: VaultScreenState = VaultScreenState.Locked,
    ): VaultScreenState = guarded(previous) {
        val existingCount = runCatching { runtime.workflowText("wallets").getJSONArray("wallets").length() }
            .getOrDefault(0)
        val name = "Wallet ${existingCount + 1}"
        creationSession = CreationSession(addToInventory, previous, name)
        VaultScreenState.CreationName(name)
    }

    fun creationSetName(name: String): VaultScreenState = guarded(creationPrevious()) {
        val trimmed = name.trim()
        val bytes = trimmed.encodeToByteArray()
        require(trimmed.isNotEmpty() && bytes.size <= 64 && trimmed.none { it.code < 0x20 || it.code == 0x7f }) {
            "Wallet name must be 1–64 UTF-8 bytes with no control characters"
        }
        creation().walletName = trimmed
        VaultScreenState.CreationWordCount
    }

    fun creationSetWordCount(wordCount: Int): VaultScreenState = guarded(creationPrevious()) {
        require(wordCount == 12 || wordCount == 24) { "Wallet word count must be 12 or 24" }
        creation().wordCount = wordCount
        VaultScreenState.CreationDiceChoice
    }

    fun creationChooseDice(addDice: Boolean): VaultScreenState = guarded(creationPrevious()) {
        val session = creation()
        session.diceRolls.clear()
        session.diceTarget = 0
        if (addDice) VaultScreenState.CreationDiceCount(creationFlow.diceTargets)
        else VaultScreenState.CreationTouchChoice
    }

    fun creationSetDiceTarget(target: Int): VaultScreenState = guarded(creationPrevious()) {
        require(target in creationFlow.diceTargets) { "Unsupported dice-roll target" }
        val session = creation()
        session.diceTarget = target
        session.diceRolls.clear()
        VaultScreenState.CreationDiceRoll(0, target)
    }

    fun creationAddDice(value: Int): VaultScreenState = guarded(creationPrevious()) {
        require(value in 1..6) { "Dice roll must be between 1 and 6" }
        val session = creation()
        require(session.diceTarget > 0) { "Dice-roll target is not selected" }
        if (session.diceRolls.size < session.diceTarget) session.diceRolls += value
        if (session.diceRolls.size == session.diceTarget) VaultScreenState.CreationTouchChoice
        else VaultScreenState.CreationDiceRoll(session.diceRolls.size, session.diceTarget)
    }

    fun creationUndoDice(): VaultScreenState = guarded(creationPrevious()) {
        val session = creation()
        if (session.diceRolls.isNotEmpty()) session.diceRolls.removeAt(session.diceRolls.lastIndex)
        VaultScreenState.CreationDiceRoll(session.diceRolls.size, session.diceTarget)
    }

    fun creationResetDice(): VaultScreenState = guarded(creationPrevious()) {
        val session = creation()
        session.diceRolls.clear()
        VaultScreenState.CreationDiceRoll(0, session.diceTarget)
    }

    fun creationChooseTouch(addTouch: Boolean): VaultScreenState = guarded(creationPrevious()) {
        creation().touchSamples.clear()
        if (addTouch) VaultScreenState.CreationTouch(0, creationFlow.touchTarget)
        else VaultScreenState.CreationPassphraseChoice
    }

    fun creationAddTouch(time: Long, x: Int, y: Int): VaultScreenState = guarded(creationPrevious()) {
        val session = creation()
        if (session.touchSamples.size < creationFlow.touchTarget) {
            val sample = TouchEntropySample(time, x.coerceIn(0, 65535), y.coerceIn(0, 65535))
            if (session.touchSamples.lastOrNull()?.let { it.x == sample.x && it.y == sample.y } != true) {
                session.touchSamples += sample
            }
        }
        if (session.touchSamples.size >= creationFlow.touchTarget) VaultScreenState.CreationPassphraseChoice
        else VaultScreenState.CreationTouch(session.touchSamples.size, creationFlow.touchTarget)
    }

    fun creationResetTouch(): VaultScreenState = guarded(creationPrevious()) {
        creation().touchSamples.clear()
        VaultScreenState.CreationTouch(0, creationFlow.touchTarget)
    }

    fun creationChoosePassphrase(usePassphrase: Boolean): VaultScreenState = guarded(creationPrevious()) {
        if (usePassphrase) VaultScreenState.CreationPassphrase
        else createFromCreationSession("")
    }

    fun creationSubmitPassphrase(value: String, confirmation: String): VaultScreenState = guarded(creationPrevious()) {
        require(value == confirmation) { "BIP39 passphrases do not match" }
        createFromCreationSession(value)
    }

    fun finishBackup(): VaultScreenState = if (creationSession != null) {
        set(VaultScreenState.CreationRecoveryAcknowledgement)
    } else {
        set(VaultScreenState.MainMenu)
    }

    fun acknowledgeCreationBackup(): VaultScreenState = set(VaultScreenState.CreationStorageFinalize)

    fun creationStorageChoice(save: Boolean): VaultScreenState = if (save) {
        set(VaultScreenState.CreationStorageProtection)
    } else {
        automaticPersistence = false
        finishCreationDestination()
    }

    fun creationUseDeviceProtection(): VaultScreenState = guarded(creationPrevious()) {
        automaticPersistence = true
        persistUnlocked(force = true)
        finishCreationDestinationValue()
    }

    fun creationNoProtection(): VaultScreenState {
        automaticPersistence = false
        return finishCreationDestination()
    }

    fun cancelCreation(): VaultScreenState {
        val previous = creationSession?.previous ?: if (runtimeUnlocked()) VaultScreenState.MainMenu else VaultScreenState.Locked
        clearCreationSession()
        return set(previous)
    }

    fun mainMenu(): VaultScreenState = set(VaultScreenState.MainMenu)
    fun walletMenu(): VaultScreenState = set(VaultScreenState.WalletMenu)
    fun backupMethods(): VaultScreenState = set(VaultScreenState.BackupMethods)
    fun advancedBackup(previous: VaultScreenState): VaultScreenState = set(VaultScreenState.AdvancedBackup(previous))
    fun walletAdvanced(): VaultScreenState = set(VaultScreenState.WalletAdvanced)
    fun settings(): VaultScreenState = set(VaultScreenState.Settings)
    fun homeShortcutVisible(value: VaultScreenState): Boolean = homeReached && value != VaultScreenState.MainMenu && value != VaultScreenState.Locked

    fun connect(): VaultScreenState = guarded(state) {
        VaultScreenState.Connect(runtime.exportPublicAccount())
    }

    fun walletDetails(): VaultScreenState = guarded(state) {
        VaultScreenState.WalletDetails(runtime.exportPublicAccount())
    }

    fun revealWords(): VaultScreenState = guarded(state) {
        VaultScreenState.RecoveryWords(runtime.backupWords())
    }

    fun seedQr(compact: Boolean, title: String, previous: VaultScreenState): VaultScreenState = guarded(state) {
        VaultScreenState.SeedQr(title, runtime.backupSeedQr(compact), previous)
    }

    fun plainSeedQr(previous: VaultScreenState): VaultScreenState = guarded(state) {
        VaultScreenState.SeedQr("Plain-text SeedQR", runtime.backupWords().encodeToByteArray(), previous)
    }

    fun xprvBackup(): VaultScreenState = set(VaultScreenState.XprvExport)

    fun showXprvQr(): VaultScreenState = guarded(VaultScreenState.XprvExport) {
        VaultScreenState.SecretText(
            "XPrv Backup",
            "This XPrv can control the wallet account. Keep it private and offline.",
            runtime.backupXprv(),
            VaultScreenState.XprvExport,
        )
    }

    fun exportReceiveKey(index: Int): VaultScreenState = guarded(state) {
        VaultScreenState.SecretText(
            "Export Key",
            "This private key can spend funds controlled by its receive address. Keep it private and offline.",
            runtime.exportReceiveKey(index),
            state,
        )
    }

    fun receive(change: Boolean = false, index: Int = 0): VaultScreenState = guarded(state) {
        val json = runtime.workflowText(
            "receive_address",
            JSONObject().put("network", "mainnet").put("change", change).put("index", index),
        )
        VaultScreenState.Receive(json.getString("address"), change, index)
    }

    fun recoveryMenu(): VaultScreenState = set(VaultScreenState.RecoveryMenu)
    fun multisigMenu(): VaultScreenState = set(VaultScreenState.MultisigMenu)
    fun multisigKpub(): VaultScreenState = guarded(VaultScreenState.MultisigMenu) {
        val value = runtime.workflowText("multisig_kpub").getString("kpub")
        VaultScreenState.SecretText(
            "kpub Multisig QR",
            "Share this public BIP45 account key with the other multisig participants. It contains no private key material.",
            value,
            VaultScreenState.MultisigMenu,
        )
    }
    fun tool(tool: NativeTool, previous: VaultScreenState): VaultScreenState = set(VaultScreenState.ToolForm(tool, previous))
    fun fileWorkflow(workflow: NativeFileWorkflow, previous: VaultScreenState): VaultScreenState = set(VaultScreenState.FileWorkflow(workflow, previous))
    fun info(title: String, body: String, previous: VaultScreenState): VaultScreenState = set(VaultScreenState.Info(title, body, previous))

    fun portableBackup(password: String, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        require(password.length >= 8) { "Backup password must contain at least 8 characters" }
        val bytes = runtime.workflowBytes("portable_backup", JSONObject().put("password", password))
        VaultScreenState.ExportFile(
            "Encrypted Wallet Backup", "kaskold-vault-backup.kwp", "application/octet-stream", bytes, previous,
        )
    }

    fun portableXprvBackup(password: String, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        require(password.length >= 8) { "Backup password must contain at least 8 characters" }
        val bytes = runtime.workflowBytes("portable_xprv_backup", JSONObject().put("password", password))
        VaultScreenState.ExportFile(
            "Encrypted XPrv Backup", "kaskold-xprv-backup.kwp", "application/octet-stream", bytes, previous,
        )
    }

    fun stegoBackup(carrier: ByteArray, password: String, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        require(password.length >= 8) { "Backup password must contain at least 8 characters" }
        val bytes = runtime.workflowBytes("stego_backup", JSONObject().put("password", password), carrier)
        VaultScreenState.ExportFile(
            "Steganographic Backup", "kaskold-steganographic-backup.jpg", "image/jpeg", bytes, previous,
        )
    }

    fun walletInventory(): VaultScreenState = guarded(state) {
        walletInventoryValue()
    }

    fun switchWallet(index: Int): VaultScreenState = guarded(state) {
        runtime.workflowText("switch_wallet", JSONObject().put("index", index))
        persistUnlocked()
        VaultScreenState.WalletMenu
    }

    fun submitTool(tool: NativeTool, values: List<String>, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        val input = JSONObject()
        val title: String
        val warning: String
        val result = when (tool) {
            NativeTool.Receive -> return@guarded receive(values.getOrNull(0) == "change", values.getOrNull(1)?.toIntOrNull() ?: 0)
            NativeTool.ImportRawKey -> {
                input.put("privateKey", values.firstOrNull().orEmpty())
                title = "Raw Private Key Imported"; warning = "The imported wallet is restricted to its matching single-key address."
                runtime.workflowText("import_raw_key", input)
            }
            NativeTool.ImportXprv -> {
                input.put("xprv", values.firstOrNull().orEmpty())
                title = "XPrv Imported"; warning = "The imported account key is now held by the Rust Vault runtime."
                runtime.workflowText("import_xprv", input)
            }
            NativeTool.MultisigCreate -> {
                val cosigners = JSONArray()
                values.getOrNull(1).orEmpty().lineSequence().map(String::trim).filter(String::isNotEmpty).forEach(cosigners::put)
                input.put("threshold", values.getOrNull(0)?.toIntOrNull() ?: 2)
                    .put("cosigners", cosigners).put("network", "mainnet").put("chain", 0).put("index", 0)
                title = "Multisig"; warning = "Verify the descriptor and address with every participant before receiving funds."
                runtime.workflowText("create_multisig", input)
            }
            NativeTool.MultisigImport -> {
                input.put("descriptor", values.firstOrNull().orEmpty()).put("network", "mainnet").put("chain", 0).put("index", 0)
                title = "Multisig Descriptor"; warning = "Verify the derived address before use."
                runtime.workflowText("import_multisig", input)
            }
            NativeTool.Bip85 -> {
                input.put("wordCount", values.getOrNull(0)?.toIntOrNull() ?: 12).put("index", values.getOrNull(1)?.toLongOrNull() ?: 0L)
                title = "BIP85 Child Wallet"; warning = "These derived recovery words control a child wallet. Keep them private."
                runtime.workflowText("bip85", input)
            }
            NativeTool.SignMessage -> {
                input.put("message", values.firstOrNull().orEmpty())
                title = "Signed Message"; warning = "Verify the message before sharing its signature."
                runtime.workflowText("sign_message", input)
            }
            NativeTool.CommitSecret -> {
                input.put("secret", values.firstOrNull().orEmpty())
                title = "Commit Secret"; warning = "Save the commitment/payload needed by the counterpart protocol."
                runtime.workflowText("commit_secret", input)
            }
            NativeTool.DecryptSecret -> {
                input.put("payloadHex", values.firstOrNull().orEmpty())
                title = "Decrypt Secret"; warning = "Decrypted secret material is shown only on this explicit screen."
                runtime.workflowText("decrypt_secret", input)
            }
            NativeTool.SigningPolicy -> {
                input.put("notBeforeUtc", values.getOrNull(0).orEmpty()).put("weeklyWindows", values.getOrNull(1).orEmpty())
                runtime.workflowText("set_signing_policy", input)
                title = "Security"; warning = "Session policy is enforced at the Rust signing boundary; mobile system time is not an authenticated hardware RTC."
                JSONObject().put("status", "Session signing policy enabled")
            }
        }
        if (tool == NativeTool.ImportRawKey || tool == NativeTool.ImportXprv) persistUnlocked()
        VaultScreenState.SecretText(title, warning, result.toString(2), previous)
    }

    fun importFile(workflow: NativeFileWorkflow, data: ByteArray, password: String, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        when (workflow) {
            NativeFileWorkflow.RecoveryMaterial -> {
                runtime.workflowTextWithBytes("recover_material", JSONObject().put("passphrase", password), data)
                persistUnlocked(); VaultScreenState.WalletMenu
            }
            NativeFileWorkflow.WalletBackup -> {
                val text = data.toString(Charsets.UTF_8).trim()
                if (text.startsWith("kprv")) {
                    runtime.workflowText("import_xprv", JSONObject().put("xprv", text))
                } else {
                    require(password.isNotEmpty()) { "Enter the encrypted backup password" }
                    runtime.workflowTextWithBytes("restore_portable", JSONObject().put("password", password), data)
                }
                persistUnlocked(); VaultScreenState.WalletMenu
            }
            NativeFileWorkflow.PortableBackup -> {
                runtime.workflowTextWithBytes("restore_portable", JSONObject().put("password", password), data)
                persistUnlocked(); VaultScreenState.WalletMenu
            }
            NativeFileWorkflow.StegoBackup -> {
                runtime.workflowTextWithBytes("restore_stego", JSONObject().put("password", password), data)
                persistUnlocked(); VaultScreenState.WalletMenu
            }
            NativeFileWorkflow.Transaction -> {
                val review = runtime.workflowTextWithBytes("transaction_file", JSONObject(), data)
                VaultScreenState.Review(
                    VaultReview(
                        network = review.getString("network"), inputCount = review.getInt("inputCount"), outputCount = review.getInt("outputCount"),
                        inputTotal = review.getString("inputTotal"), outputTotal = review.getString("outputTotal"), fee = review.getString("fee"),
                    ),
                )
            }
            NativeFileWorkflow.Kpub -> {
                val value = runtime.workflowText("normalize_kpub", JSONObject().put("value", data.toString(Charsets.UTF_8).trim())).getString("value")
                VaultScreenState.SecretText("kpub (Watch-Only)", "Validated public account. This value contains no private keys.", value, previous)
            }
            NativeFileWorkflow.MultisigAddress -> {
                val value = runtime.workflowText("validate_address", JSONObject().put("value", data.toString(Charsets.UTF_8).trim())).getString("value")
                VaultScreenState.SecretText("Multisig Address", "Validated Kaspa address. Verify it with every participant before use.", value, previous)
            }
            NativeFileWorkflow.MultisigDescriptor -> {
                val descriptor = data.toString(Charsets.UTF_8).trim()
                val result = runtime.workflowText(
                    "import_multisig",
                    JSONObject().put("descriptor", descriptor).put("network", "mainnet").put("chain", 0).put("index", 0),
                )
                VaultScreenState.SecretText("Multisig Descriptor", "Descriptor validated and retained for multisig signing/change verification.", result.toString(2), previous)
            }
            NativeFileWorkflow.CovenantRestore -> {
                val normalized = runtime.workflowBytes("normalize_covenant_backup", JSONObject(), data)
                VaultScreenState.SecretText(
                    "Covenant Restore", "Validated against the shared M5 covenant-backup format.", normalized.toHex(), previous,
                )
            }
            NativeFileWorkflow.PortableExport -> portableBackup(password, previous)
            NativeFileWorkflow.PortableXprvExport -> portableXprvBackup(password, previous)
            NativeFileWorkflow.StegoExport -> stegoBackup(data, password, previous)
        }
    }


    fun showRestore(addToInventory: Boolean = false, previous: VaultScreenState = VaultScreenState.Locked): VaultScreenState =
        set(VaultScreenState.Restore(addToInventory = addToInventory, previous = previous))

    /** Restore from recovery words, mirroring the iOS Vault: a fresh vault restores its only
     *  wallet; from the wallet menu the restored wallet is added to the inventory. */
    fun restore(
        phrase: String,
        passphrase: String,
        addToInventory: Boolean,
        previous: VaultScreenState,
    ): VaultScreenState = guarded(VaultScreenState.Restore(addToInventory = addToInventory, previous = previous)) {
        val words = phrase.trim()
        if (addToInventory) {
            runtime.workflowText("add_restore", JSONObject().put("phrase", words).put("passphrase", passphrase))
        } else {
            runtime.restoreWallet(words, passphrase)
        }
        persistUnlocked()
        if (addToInventory) walletInventoryValue() else VaultScreenState.MainMenu
    }

    fun beginScan(): VaultScreenState = guarded(state) {
        runtime.beginScan()
        VaultScreenState.Scanning()
    }

    fun acceptFrame(frame: ByteArray): VaultScreenState = guarded(state) {
        when (val result = runtime.acceptQrFrame(frame)) {
            is QrAcceptResult.Progress -> VaultScreenState.Scanning(result.received, result.total)
            is QrAcceptResult.Ready -> VaultScreenState.Review(result.review)
        }
    }

    fun approve(): VaultScreenState = guarded(state) {
        VaultScreenState.Response(runtime.approve())
    }

    fun reject(): VaultScreenState = guarded(state) {
        runtime.reject()
        VaultScreenState.MainMenu
    }

    fun finishResponse(): VaultScreenState = guarded(state) {
        runtime.reject()
        VaultScreenState.MainMenu
    }

    fun lock(): VaultScreenState {
        runtime.lock()
        clearCreationSession()
        return set(VaultScreenState.Locked)
    }

    fun deleteWallet(): VaultScreenState {
        runtime.lock()
        clearCreationSession()
        blobStore.delete()
        keyStore.delete()
        return set(VaultScreenState.Locked)
    }

    fun setState(value: VaultScreenState): VaultScreenState = set(value)

    private fun loadCreationFlow(): CreationFlowConfig {
        val objectValue = runtime.workflowText("creation_flow")
        val targets = objectValue.getJSONArray("diceRollTargets")
        val diceTargets = buildList {
            for (index in 0 until targets.length()) add(targets.getInt(index))
        }
        val touchTarget = objectValue.getInt("touchEntropyTarget")
        require(diceTargets.isNotEmpty() && diceTargets.all { it > 0 } && touchTarget > 0) {
            "KasKold shared creation-flow policy is invalid"
        }
        return CreationFlowConfig(diceTargets, touchTarget)
    }

    private fun creation(): CreationSession =
        checkNotNull(creationSession) { "Wallet creation is not active" }

    private fun creationPrevious(): VaultScreenState = creationSession?.previous ?: state

    private fun createFromCreationSession(passphrase: String): VaultScreenState {
        val session = creation()
        session.passphrase = passphrase
        val touch = encodeTouchTranscript(session.touchSamples)
        val dice = session.diceRolls.joinToString(separator = "")
        return try {
            val operation = if (session.addToInventory) "add_create_with_entropy" else "create_with_entropy"
            val result = runtime.workflowTextWithBytes(
                operation,
                JSONObject()
                    .put("wordCount", session.wordCount)
                    .put("dice", dice)
                    .put("passphrase", session.passphrase),
                touch,
            )
            val wallets = runtime.workflowText("wallets").getJSONArray("wallets")
            for (index in 0 until wallets.length()) {
                val wallet = wallets.getJSONObject(index)
                if (wallet.getBoolean("active")) {
                    runtime.workflowText(
                        "set_wallet_name",
                        JSONObject().put("index", wallet.getInt("index")).put("name", session.walletName),
                    )
                    break
                }
            }
            VaultScreenState.Backup(result.getString("recoveryPhrase"))
        } finally {
            touch.fill(0)
            session.passphrase = ""
        }
    }

    private fun encodeTouchTranscript(samples: List<TouchEntropySample>): ByteArray {
        val output = ByteArray(samples.size * 8)
        samples.forEachIndexed { index, sample ->
            val offset = index * 8
            val time = sample.time.toInt()
            output[offset] = time.toByte()
            output[offset + 1] = (time ushr 8).toByte()
            output[offset + 2] = (time ushr 16).toByte()
            output[offset + 3] = (time ushr 24).toByte()
            output[offset + 4] = sample.x.toByte()
            output[offset + 5] = (sample.x ushr 8).toByte()
            output[offset + 6] = sample.y.toByte()
            output[offset + 7] = (sample.y ushr 8).toByte()
        }
        return output
    }

    private fun finishCreationDestination(): VaultScreenState = set(finishCreationDestinationValue())

    private fun finishCreationDestinationValue(): VaultScreenState {
        val addToInventory = creationSession?.addToInventory == true
        clearCreationSession()
        return if (addToInventory) walletInventoryValue() else VaultScreenState.MainMenu
    }

    private fun walletInventoryValue(): VaultScreenState {
        val array = runtime.workflowText("wallets").getJSONArray("wallets")
        val wallets = buildList {
            for (index in 0 until array.length()) {
                val item = array.getJSONObject(index)
                add(
                    NativeWalletSummary(
                        index = item.getInt("index"),
                        name = item.getString("name"),
                        active = item.getBoolean("active"),
                        kind = item.getString("kind"),
                        kpub = if (item.isNull("kpub")) null else item.getString("kpub"),
                    ),
                )
            }
        }
        return VaultScreenState.WalletInventory(wallets)
    }

    private fun clearCreationSession() {
        creationSession?.diceRolls?.clear()
        creationSession?.touchSamples?.clear()
        creationSession?.passphrase = ""
        creationSession = null
    }

    private fun runtimeUnlocked(): Boolean = runCatching {
        runtime.workflowText("wallets").getJSONArray("wallets").length() > 0
    }.getOrDefault(false)

    private fun persistUnlocked(force: Boolean = false) {
        if (!force && !automaticPersistence) return
        val key = keyStore.loadOrCreate()
        val sealed = try {
            runtime.sealWallet(key)
        } finally {
            key.fill(0)
        }
        try {
            blobStore.writeSealedWallet(sealed)
        } finally {
            sealed.fill(0)
        }
    }

    private inline fun guarded(previous: VaultScreenState, operation: () -> VaultScreenState): VaultScreenState =
        try { set(operation()) } catch (error: Throwable) {
            set(VaultScreenState.Error(error.message ?: error.javaClass.simpleName, previous))
        }

    private fun set(value: VaultScreenState): VaultScreenState {
        if (value == VaultScreenState.MainMenu) homeReached = true
        if (value == VaultScreenState.Locked) homeReached = false
        state = value
        return value
    }

    override fun close() = runtime.close()
}

private fun ByteArray.toHex(): String = joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }
