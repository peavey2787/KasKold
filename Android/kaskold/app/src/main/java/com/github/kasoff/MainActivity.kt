package com.github.kasoff

import android.app.KeyguardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.webkit.WebSettings
import android.webkit.WebView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import fi.iki.elonen.NanoHTTPD
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import android.webkit.PermissionRequest
import android.webkit.WebChromeClient

class MainActivity : AppCompatActivity() {

    // Folder name to copy your web app assets into
    private val OFFLINE_WEB_DIR = "offlineweb"
    // Port number for your local HTTP server
    private val SERVER_PORT = 8080
    // Our embedded HTTP server
    private var server: LocalHttpServer? = null
    // WebView to display our served content
    private lateinit var webView: WebView

    // Request code for device credential authentication
    private val REQUEST_CODE_CONFIRM_CREDENTIALS = 1

    // Track when the app was last paused
    private var lastPauseTime: Long = 0
    // Define the threshold (in milliseconds) after which authentication is required.
    // For example, 30 seconds.
    private val AUTHENTICATION_TIMEOUT = 15000L // 15 seconds

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Request camera permission at runtime for Android 6.0+ if not already granted
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M) {
            if (checkSelfPermission(android.Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(arrayOf(android.Manifest.permission.CAMERA), 1001)
            }
        }

        // Set up the WebView that will load the HTTP-served content.
        webView = WebView(this)
        setContentView(webView)

        // Configure the WebView settings
        webView.settings.apply {
            javaScriptEnabled = true
            loadWithOverviewMode = true
            useWideViewPort = true
            domStorageEnabled = true // Enable DOM storage
            databaseEnabled = true    // Enable database storage
            allowFileAccess = true    // Allow file access
            allowContentAccess = true // Allow content access
            allowFileAccessFromFileURLs = true  // Enable for ES6 modules
            allowUniversalAccessFromFileURLs = true // Enable for ES6 modules
            mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW // Allow mixed content
            cacheMode = WebSettings.LOAD_DEFAULT  // Enable caching
        }

        // Set a custom WebChromeClient to handle permission requests (e.g., for camera access)
        webView.webChromeClient = object : WebChromeClient() {
            override fun onPermissionRequest(request: PermissionRequest?) {
                request?.let {
                    runOnUiThread {
                        // Grant all requested permissions (consider adding more logic to check each one)
                        it.grant(it.resources)
                    }
                }
            }
        }

        // Copy assets from assets/webapp/ into our app-specific directory
        val offlineWebDir = File(getExternalFilesDir(null), OFFLINE_WEB_DIR)
        copyAssets("webapp", offlineWebDir)

        // Start the local HTTP server serving content from offlineWebDir
        server = LocalHttpServer(offlineWebDir, SERVER_PORT)
        try {
            server?.start()
        } catch (e: Exception) {
            Toast.makeText(this, "Server failed to start: ${e.message}", Toast.LENGTH_LONG).show()
            e.printStackTrace()
        }

        // Load the web app by accessing it via HTTP (e.g., http://localhost:8080/index.html)
        webView.loadUrl("http://localhost:$SERVER_PORT/index.html")
    }

    override fun onResume() {
        super.onResume()
        // Only prompt for authentication if the time since onPause exceeds the threshold.
        if (System.currentTimeMillis() - lastPauseTime >= AUTHENTICATION_TIMEOUT) {
            promptDeviceAuthentication()
        }
    }

    override fun onPause() {
        super.onPause()
        // Record the time when the app is paused.
        lastPauseTime = System.currentTimeMillis()
    }

    // Uses the KeyguardManager to display the system lock screen (PIN, pattern, or biometric)
    private fun promptDeviceAuthentication() {
        val keyguardManager = getSystemService(KEYGUARD_SERVICE) as KeyguardManager
        if (keyguardManager.isKeyguardSecure) {
            val intent = keyguardManager.createConfirmDeviceCredentialIntent(
                "Unlock Wallet",
                "Please verify your device credentials to continue."
            )
            if (intent != null) {
                startActivityForResult(intent, REQUEST_CODE_CONFIRM_CREDENTIALS)
            } else {
                Toast.makeText(this, "Unable to launch secure authentication. Exiting...", Toast.LENGTH_LONG).show()
                finish()
            }
        } else {
            Toast.makeText(
                this,
                "Secure lock screen is not set up. For added security, please enable it.",
                Toast.LENGTH_LONG
            ).show()
        }
    }

    // Handle the result from the device authentication intent.
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_CODE_CONFIRM_CREDENTIALS) {
            if (resultCode == RESULT_OK) {
                // Authentication successful. Continue with app functionality.
            } else {
                // Authentication failed or was canceled.
                Toast.makeText(this, "Authentication required. Exiting app...", Toast.LENGTH_SHORT).show()
                finish()
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        // Stop the server when the Activity is destroyed.
        server?.stop()
    }

    /**
     * Recursively copy assets from the given assets folder to a target directory on the file system.
     * For example, copy "webapp" (located in assets/webapp) into your offlineWebDir.
     */
    private fun copyAssets(assetPath: String, targetDir: File) {
        try {
            val assetManager = assets
            val files = assetManager.list(assetPath)
            if (!targetDir.exists()) {
                targetDir.mkdirs()
            }
            if (files != null && files.isNotEmpty()) {
                for (file in files) {
                    val assetSubPath = if (assetPath.isEmpty()) file else "$assetPath/$file"
                    val subFiles = assetManager.list(assetSubPath)
                    val targetFile = File(targetDir, file)
                    if (subFiles != null && subFiles.isNotEmpty()) {
                        // It's a directory – recurse.
                        copyAssets(assetSubPath, targetFile)
                    } else {
                        // It's a file – copy it.
                        val inputStream = assetManager.open(assetSubPath)
                        val outputStream = FileOutputStream(targetFile)
                        inputStream.copyTo(outputStream)
                        inputStream.close()
                        outputStream.close()
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    /**
     * A simple embedded HTTP server based on NanoHTTPD.
     * This server serves files from the provided rootDir on the specified port.
     */
    class LocalHttpServer(private val rootDir: File, port: Int) : NanoHTTPD(port) {
        override fun serve(session: IHTTPSession?): Response {
            val uri = session?.uri ?: "/"
            val filePath = if (uri == "/") "index.html" else uri
            val requestedFile = File(rootDir, filePath)

            return if (requestedFile.exists() && requestedFile.isFile) {
                val mimeType = when {
                    requestedFile.name.endsWith(".html") -> "text/html"
                    requestedFile.name.endsWith(".js") -> "application/javascript"
                    requestedFile.name.endsWith(".css") -> "text/css"
                    requestedFile.name.endsWith(".wasm") -> "application/wasm"
                    else -> "application/octet-stream"
                }

                val response = newChunkedResponse(Response.Status.OK, mimeType, FileInputStream(requestedFile))
                response.addHeader("Access-Control-Allow-Origin", "*")
                response.addHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                response.addHeader("Access-Control-Allow-Headers", "Content-Type")
                response.addHeader("Cross-Origin-Embedder-Policy", "require-corp")
                response.addHeader("Cross-Origin-Opener-Policy", "same-origin")
                response
            } else {
                newFixedLengthResponse(Response.Status.NOT_FOUND, "text/plain", "404 Not Found")
            }
        }
    }
}
