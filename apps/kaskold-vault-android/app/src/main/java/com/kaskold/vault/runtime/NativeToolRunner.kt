package com.kaskold.vault

import org.json.JSONArray
import org.json.JSONObject

internal data class ToolOutcome(
    val title: String,
    val warning: String,
    val text: String,
    val changedCustody: Boolean,
)

private class ToolSpec(
    val operation: String,
    val title: String,
    val warning: String,
    val changesCustody: Boolean = false,
    val describe: (JSONObject) -> JSONObject = { it },
    val input: (List<String>) -> JSONObject,
)

private fun List<String>.field(position: Int = 0): String = getOrNull(position).orEmpty()

private fun JSONObject.atFirstAddress(): JSONObject = put("network", "mainnet").put("chain", 0).put("index", 0)

private fun cosigners(lines: String): JSONArray {
    val keys = JSONArray()
    lines.lineSequence().map(String::trim).filter(String::isNotEmpty).forEach(keys::put)
    return keys
}

private val TOOL_SPECS = mapOf(
    NativeTool.ImportRawKey to ToolSpec(
        "import_raw_key", "Raw Private Key Imported",
        "The imported wallet is restricted to its matching single-key address.",
        changesCustody = true,
    ) { JSONObject().put("privateKey", it.field()) },
    NativeTool.ImportXprv to ToolSpec(
        "import_xprv", "XPrv Imported",
        "The imported account key is now held by the Rust Vault runtime.",
        changesCustody = true,
    ) { JSONObject().put("xprv", it.field()) },
    NativeTool.MultisigCreate to ToolSpec(
        "create_multisig", "Multisig",
        "Verify the descriptor and address with every participant before receiving funds.",
    ) {
        JSONObject().put("threshold", it.field(0).toIntOrNull() ?: 2).put("cosigners", cosigners(it.field(1))).atFirstAddress()
    },
    NativeTool.MultisigImport to ToolSpec(
        "import_multisig", "Multisig Descriptor",
        "Verify the derived address before use.",
    ) { JSONObject().put("descriptor", it.field()).atFirstAddress() },
    NativeTool.Bip85 to ToolSpec(
        "bip85", "BIP85 Child Wallet",
        "These derived recovery words control a child wallet. Keep them private.",
    ) { JSONObject().put("wordCount", it.field(0).toIntOrNull() ?: 12).put("index", it.field(1).toLongOrNull() ?: 0L) },
    NativeTool.SignMessage to ToolSpec(
        "sign_message", "Signed Message",
        "Verify the message before sharing its signature.",
    ) { JSONObject().put("message", it.field()) },
    NativeTool.CommitSecret to ToolSpec(
        "commit_secret", "Commit Secret",
        "Save the commitment/payload needed by the counterpart protocol.",
    ) { JSONObject().put("secret", it.field()) },
    NativeTool.DecryptSecret to ToolSpec(
        "decrypt_secret", "Decrypt Secret",
        "Decrypted secret material is shown only on this explicit screen.",
    ) { JSONObject().put("payloadHex", it.field()) },
    NativeTool.SigningPolicy to ToolSpec(
        "set_signing_policy", "Security",
        "Session policy is enforced at the Rust signing boundary; mobile system time is not an authenticated hardware RTC.",
        describe = { JSONObject().put("status", "Session signing policy enabled") },
    ) { JSONObject().put("notBeforeUtc", it.field(0)).put("weeklyWindows", it.field(1)) },
)

/** Runs a form tool through the Rust runtime; [NativeTool.Receive] opens its own screen instead. */
internal class NativeToolRunner(private val runtime: RuntimeBridge) {
    fun run(tool: NativeTool, values: List<String>): ToolOutcome {
        val spec = TOOL_SPECS.getValue(tool)
        val result = runtime.workflowText(spec.operation, spec.input(values))
        return ToolOutcome(spec.title, spec.warning, spec.describe(result).toString(2), spec.changesCustody)
    }
}
