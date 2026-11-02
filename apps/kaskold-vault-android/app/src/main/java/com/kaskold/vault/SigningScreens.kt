package com.kaskold.vault

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

@Composable
internal fun ScanningScreen(state: VaultScreenState.Scanning, onFrame: (ByteArray) -> Unit, onCancel: () -> Unit, onError: (String) -> Unit) {
    Text(if (state.total > 0) "Scanning request · ${state.received}/${state.total} frames" else "Scanning signing request")
    QrScannerView(onFrame = onFrame, onError = onError)
    OutlinedButton(onClick = onCancel, modifier = Modifier.fillMaxWidth()) { Text("Cancel") }
}

@Composable
internal fun ReviewScreen(review: VaultReview, onApprove: () -> Unit, onReject: () -> Unit) {
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
internal fun ResponseScreen(frames: List<ByteArray>, onDone: () -> Unit) {
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
internal fun ErrorScreen(message: String, onBack: () -> Unit) {
    Text("Vault operation failed", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(message)
    Button(onClick = onBack, modifier = Modifier.fillMaxWidth()) { Text("Back") }
}
