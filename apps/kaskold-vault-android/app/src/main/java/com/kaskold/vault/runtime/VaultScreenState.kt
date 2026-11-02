package com.kaskold.vault

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
