package com.kaskold.vault

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Bitmap
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.common.InputImage
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import java.nio.charset.StandardCharsets
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

@Composable
internal fun QrScannerView(
    onFrame: (ByteArray) -> Unit,
    onError: (String) -> Unit,
) {
    val context = LocalContext.current
    var granted by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED,
        )
    }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
        granted = allowed
        if (!allowed) onError("Camera permission is required to scan a signing request")
    }
    LaunchedEffect(Unit) { if (!granted) launcher.launch(Manifest.permission.CAMERA) }
    if (!granted) return

    val lifecycleOwner = LocalLifecycleOwner.current
    val executor = remember { Executors.newSingleThreadExecutor() }
    val scanner = remember {
        BarcodeScanning.getClient(
            BarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build(),
        )
    }
    val busy = remember { AtomicBoolean(false) }

    AndroidView(
        modifier = Modifier.fillMaxWidth().height(420.dp),
        factory = { viewContext ->
            val previewView = PreviewView(viewContext)
            val cameraProviderFuture = ProcessCameraProvider.getInstance(viewContext)
            cameraProviderFuture.addListener({
                runCatching {
                    val provider = cameraProviderFuture.get()
                    val preview = Preview.Builder().build().also { it.surfaceProvider = previewView.surfaceProvider }
                    val analysis = ImageAnalysis.Builder()
                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                        .build()
                    analysis.setAnalyzer(executor) { imageProxy ->
                        val mediaImage = imageProxy.image
                        if (mediaImage == null || !busy.compareAndSet(false, true)) {
                            imageProxy.close()
                            return@setAnalyzer
                        }
                        val image = InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
                        scanner.process(image)
                            .addOnSuccessListener { barcodes ->
                                val qr = barcodes.firstOrNull { it.format == Barcode.FORMAT_QR_CODE }
                                val payload = qr?.rawBytes ?: qr?.rawValue?.toByteArray(StandardCharsets.ISO_8859_1)
                                if (payload != null && payload.isNotEmpty()) onFrame(payload)
                            }
                            .addOnFailureListener { error -> onError(error.message ?: "QR decoding failed") }
                            .addOnCompleteListener {
                                busy.set(false)
                                imageProxy.close()
                            }
                    }
                    provider.unbindAll()
                    provider.bindToLifecycle(lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
                }.onFailure { onError(it.message ?: "Camera initialization failed") }
            }, ContextCompat.getMainExecutor(viewContext))
            previewView
        },
    )

    DisposableEffect(Unit) {
        onDispose {
            scanner.close()
            executor.shutdownNow()
        }
    }
}

@Composable
internal fun QrCodeView(payload: ByteArray, modifier: Modifier = Modifier) {
    val bitmap = remember(payload.contentHashCode()) { encodeQr(payload) }
    Image(bitmap = bitmap.asImageBitmap(), contentDescription = "KasKold QR code", modifier = modifier)
}

private fun encodeQr(payload: ByteArray): Bitmap {
    val binaryText = String(payload, StandardCharsets.ISO_8859_1)
    val matrix = QRCodeWriter().encode(
        binaryText,
        BarcodeFormat.QR_CODE,
        720,
        720,
        mapOf(
            EncodeHintType.CHARACTER_SET to "ISO-8859-1",
            EncodeHintType.MARGIN to 2,
        ),
    )
    return Bitmap.createBitmap(matrix.width, matrix.height, Bitmap.Config.ARGB_8888).also { bitmap ->
        for (y in 0 until matrix.height) {
            for (x in 0 until matrix.width) {
                bitmap.setPixel(x, y, if (matrix[x, y]) android.graphics.Color.BLACK else android.graphics.Color.WHITE)
            }
        }
    }
}
