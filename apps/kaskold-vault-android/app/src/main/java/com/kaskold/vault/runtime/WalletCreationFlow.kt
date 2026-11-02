package com.kaskold.vault

import org.json.JSONObject

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

/** The wallet-creation wizard: its entropy-bearing session and the screen each step leads to. */
internal class WalletCreationFlow(private val runtime: RuntimeBridge) {
    private val diceTargets: List<Int>
    private val touchTarget: Int
    private var session: CreationSession? = null

    init {
        val policy = runtime.workflowText("creation_flow")
        val targets = policy.getJSONArray("diceRollTargets")
        diceTargets = buildList {
            for (index in 0 until targets.length()) add(targets.getInt(index))
        }
        touchTarget = policy.getInt("touchEntropyTarget")
        require(diceTargets.isNotEmpty() && diceTargets.all { it > 0 } && touchTarget > 0) {
            "KasKold shared creation-flow policy is invalid"
        }
    }

    val active: Boolean get() = session != null
    val addsToInventory: Boolean get() = session?.addToInventory == true
    val previous: VaultScreenState? get() = session?.previous

    fun begin(addToInventory: Boolean, previous: VaultScreenState): VaultScreenState {
        val existingCount = runCatching { runtime.workflowText("wallets").getJSONArray("wallets").length() }
            .getOrDefault(0)
        val name = "Wallet ${existingCount + 1}"
        session = CreationSession(addToInventory, previous, name)
        return VaultScreenState.CreationName(name)
    }

    fun setName(name: String): VaultScreenState {
        val trimmed = name.trim()
        val bytes = trimmed.encodeToByteArray()
        require(trimmed.isNotEmpty() && bytes.size <= 64 && trimmed.none { it.code < 0x20 || it.code == 0x7f }) {
            "Wallet name must be 1–64 UTF-8 bytes with no control characters"
        }
        current().walletName = trimmed
        return VaultScreenState.CreationWordCount
    }

    fun setWordCount(wordCount: Int): VaultScreenState {
        require(wordCount == 12 || wordCount == 24) { "Wallet word count must be 12 or 24" }
        current().wordCount = wordCount
        return VaultScreenState.CreationDiceChoice
    }

    fun chooseDice(addDice: Boolean): VaultScreenState {
        val creation = current()
        creation.diceRolls.clear()
        creation.diceTarget = 0
        return if (addDice) VaultScreenState.CreationDiceCount(diceTargets)
        else VaultScreenState.CreationTouchChoice
    }

    fun setDiceTarget(target: Int): VaultScreenState {
        require(target in diceTargets) { "Unsupported dice-roll target" }
        val creation = current()
        creation.diceTarget = target
        creation.diceRolls.clear()
        return VaultScreenState.CreationDiceRoll(0, target)
    }

    fun addDice(value: Int): VaultScreenState {
        require(value in 1..6) { "Dice roll must be between 1 and 6" }
        val creation = current()
        require(creation.diceTarget > 0) { "Dice-roll target is not selected" }
        if (creation.diceRolls.size < creation.diceTarget) creation.diceRolls += value
        return if (creation.diceRolls.size == creation.diceTarget) VaultScreenState.CreationTouchChoice
        else VaultScreenState.CreationDiceRoll(creation.diceRolls.size, creation.diceTarget)
    }

    fun undoDice(): VaultScreenState {
        val creation = current()
        if (creation.diceRolls.isNotEmpty()) creation.diceRolls.removeAt(creation.diceRolls.lastIndex)
        return VaultScreenState.CreationDiceRoll(creation.diceRolls.size, creation.diceTarget)
    }

    fun resetDice(): VaultScreenState {
        val creation = current()
        creation.diceRolls.clear()
        return VaultScreenState.CreationDiceRoll(0, creation.diceTarget)
    }

    fun chooseTouch(addTouch: Boolean): VaultScreenState {
        current().touchSamples.clear()
        return if (addTouch) VaultScreenState.CreationTouch(0, touchTarget)
        else VaultScreenState.CreationPassphraseChoice
    }

    fun addTouch(time: Long, x: Int, y: Int): VaultScreenState {
        val creation = current()
        if (creation.touchSamples.size < touchTarget) {
            val sample = TouchEntropySample(time, x.coerceIn(0, 65535), y.coerceIn(0, 65535))
            if (creation.touchSamples.lastOrNull()?.let { it.x == sample.x && it.y == sample.y } != true) {
                creation.touchSamples += sample
            }
        }
        return if (creation.touchSamples.size >= touchTarget) VaultScreenState.CreationPassphraseChoice
        else VaultScreenState.CreationTouch(creation.touchSamples.size, touchTarget)
    }

    fun resetTouch(): VaultScreenState {
        current().touchSamples.clear()
        return VaultScreenState.CreationTouch(0, touchTarget)
    }

    fun choosePassphrase(usePassphrase: Boolean): VaultScreenState =
        if (usePassphrase) VaultScreenState.CreationPassphrase else createWallet("")

    fun submitPassphrase(value: String, confirmation: String): VaultScreenState {
        require(value == confirmation) { "BIP39 passphrases do not match" }
        return createWallet(value)
    }

    fun clear() {
        session?.diceRolls?.clear()
        session?.touchSamples?.clear()
        session?.passphrase = ""
        session = null
    }

    private fun current(): CreationSession =
        checkNotNull(session) { "Wallet creation is not active" }

    private fun createWallet(passphrase: String): VaultScreenState {
        val creation = current()
        creation.passphrase = passphrase
        val touch = encodeTouchTranscript(creation.touchSamples)
        val dice = creation.diceRolls.joinToString(separator = "")
        return try {
            val operation = if (creation.addToInventory) "add_create_with_entropy" else "create_with_entropy"
            val result = runtime.workflowTextWithBytes(
                operation,
                JSONObject()
                    .put("wordCount", creation.wordCount)
                    .put("dice", dice)
                    .put("passphrase", creation.passphrase),
                touch,
            )
            nameActiveWallet(creation.walletName)
            VaultScreenState.Backup(result.getString("recoveryPhrase"))
        } finally {
            touch.fill(0)
            creation.passphrase = ""
        }
    }

    private fun nameActiveWallet(name: String) {
        val wallets = runtime.workflowText("wallets").getJSONArray("wallets")
        for (index in 0 until wallets.length()) {
            val wallet = wallets.getJSONObject(index)
            if (wallet.getBoolean("active")) {
                runtime.workflowText(
                    "set_wallet_name",
                    JSONObject().put("index", wallet.getInt("index")).put("name", name),
                )
                return
            }
        }
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
