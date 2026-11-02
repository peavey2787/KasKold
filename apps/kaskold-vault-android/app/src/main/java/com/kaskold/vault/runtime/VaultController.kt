package com.kaskold.vault

import android.content.Context
import android.security.keystore.UserNotAuthenticatedException
import com.kaskold.vault.security.VaultBlobStore
import com.kaskold.vault.security.VaultWrappingKeyStore
import org.json.JSONObject

internal class VaultController(context: Context) : AutoCloseable {
    private val runtime = RuntimeBridge()
    private val blobStore = VaultBlobStore(context.applicationContext)
    private val keyStore = VaultWrappingKeyStore(context.applicationContext)
    private val creation = WalletCreationFlow(runtime)
    private val tools = NativeToolRunner(runtime)
    private val files = NativeFileWorkflows(runtime) { persistUnlocked() }
    private var automaticPersistence = true
    private var homeReached = false
    var persistencePending = false
        private set
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
    ): VaultScreenState = guarded(previous) { creation.begin(addToInventory, previous) }

    fun creationSetName(name: String): VaultScreenState = inCreation { creation.setName(name) }
    fun creationSetWordCount(wordCount: Int): VaultScreenState = inCreation { creation.setWordCount(wordCount) }
    fun creationChooseDice(addDice: Boolean): VaultScreenState = inCreation { creation.chooseDice(addDice) }
    fun creationSetDiceTarget(target: Int): VaultScreenState = inCreation { creation.setDiceTarget(target) }
    fun creationAddDice(value: Int): VaultScreenState = inCreation { creation.addDice(value) }
    fun creationUndoDice(): VaultScreenState = inCreation { creation.undoDice() }
    fun creationResetDice(): VaultScreenState = inCreation { creation.resetDice() }
    fun creationChooseTouch(addTouch: Boolean): VaultScreenState = inCreation { creation.chooseTouch(addTouch) }
    fun creationAddTouch(time: Long, x: Int, y: Int): VaultScreenState = inCreation { creation.addTouch(time, x, y) }
    fun creationResetTouch(): VaultScreenState = inCreation { creation.resetTouch() }
    fun creationChoosePassphrase(usePassphrase: Boolean): VaultScreenState = inCreation { creation.choosePassphrase(usePassphrase) }
    fun creationSubmitPassphrase(value: String, confirmation: String): VaultScreenState =
        inCreation { creation.submitPassphrase(value, confirmation) }

    fun finishBackup(): VaultScreenState = if (creation.active) {
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

    fun creationUseDeviceProtection(): VaultScreenState = inCreation {
        automaticPersistence = true
        persistUnlocked(force = true)
        finishCreationDestinationValue()
    }

    fun creationNoProtection(): VaultScreenState {
        automaticPersistence = false
        return finishCreationDestination()
    }

    fun cancelCreation(): VaultScreenState {
        val previous = creation.previous ?: if (runtimeUnlocked()) VaultScreenState.MainMenu else VaultScreenState.Locked
        creation.clear()
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

    fun walletInventory(): VaultScreenState = guarded(state) {
        walletInventoryValue()
    }

    fun switchWallet(index: Int): VaultScreenState = guarded(state) {
        runtime.workflowText("switch_wallet", JSONObject().put("index", index))
        persistUnlocked()
        VaultScreenState.WalletMenu
    }

    fun submitTool(tool: NativeTool, values: List<String>, previous: VaultScreenState): VaultScreenState = guarded(previous) {
        if (tool == NativeTool.Receive) {
            return@guarded receive(values.getOrNull(0) == "change", values.getOrNull(1)?.toIntOrNull() ?: 0)
        }
        val outcome = tools.run(tool, values)
        if (outcome.changedCustody) persistUnlocked()
        VaultScreenState.SecretText(outcome.title, outcome.warning, outcome.text, previous)
    }

    fun importFile(workflow: NativeFileWorkflow, data: ByteArray, password: String, previous: VaultScreenState): VaultScreenState =
        guarded(previous) { files.process(workflow, data, password, previous) }

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

    /** Retries the save deferred by [persistUnlocked] once the device credential is confirmed. */
    fun completePendingPersistence(): VaultScreenState = guarded(state) {
        persistUnlocked(force = true)
        state
    }

    fun lock(): VaultScreenState {
        persistencePending = false
        runtime.lock()
        creation.clear()
        return set(VaultScreenState.Locked)
    }

    fun deleteWallet(): VaultScreenState {
        persistencePending = false
        runtime.lock()
        creation.clear()
        blobStore.delete()
        keyStore.delete()
        return set(VaultScreenState.Locked)
    }

    fun setState(value: VaultScreenState): VaultScreenState = set(value)

    private inline fun inCreation(operation: () -> VaultScreenState): VaultScreenState =
        guarded(creation.previous ?: state, operation)

    private fun finishCreationDestination(): VaultScreenState = set(finishCreationDestinationValue())

    private fun finishCreationDestinationValue(): VaultScreenState {
        val addToInventory = creation.addsToInventory
        creation.clear()
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

    private fun runtimeUnlocked(): Boolean = runCatching {
        runtime.workflowText("wallets").getJSONArray("wallets").length() > 0
    }.getOrDefault(false)

    private fun persistUnlocked(force: Boolean = false) {
        if (!force && !automaticPersistence) return
        val key = try {
            keyStore.loadOrCreate()
        } catch (_: UserNotAuthenticatedException) {
            persistencePending = true
            return
        }
        persistencePending = false
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
