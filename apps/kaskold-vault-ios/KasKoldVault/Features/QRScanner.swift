import AVFoundation
import Foundation
import SwiftUI
import UIKit

struct QRScanner: UIViewControllerRepresentable {
    let onFrame: (Data) -> Void
    let onError: (String) -> Void

    func makeUIViewController(context: Context) -> ScannerController {
        let controller = ScannerController()
        controller.onFrame = onFrame
        controller.onError = onError
        return controller
    }
    func updateUIViewController(_ controller: ScannerController, context: Context) {
        controller.onFrame = onFrame
        controller.onError = onError
    }
}

final class ScannerController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var onFrame: ((Data) -> Void)?
    var onError: ((String) -> Void)?
    private let session = AVCaptureSession()
    private var previewLayer: AVCaptureVideoPreviewLayer?
    private var lastPayload: Data?
    private var lastPayloadAt = Date.distantPast

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        configureCamera()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        previewLayer?.frame = view.bounds
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in self?.session.startRunning() }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        session.stopRunning()
    }

    private func configureCamera() {
        guard let camera = AVCaptureDevice.default(for: .video) else { onError?("No camera is available"); return }
        do {
            let input = try AVCaptureDeviceInput(device: camera)
            guard session.canAddInput(input) else { onError?("Camera input is unavailable"); return }
            session.addInput(input)
            let output = AVCaptureMetadataOutput()
            guard session.canAddOutput(output) else { onError?("QR scanner output is unavailable"); return }
            session.addOutput(output)
            output.setMetadataObjectsDelegate(self, queue: .main)
            output.metadataObjectTypes = [.qr]
            let layer = AVCaptureVideoPreviewLayer(session: session)
            layer.videoGravity = .resizeAspectFill
            view.layer.addSublayer(layer)
            previewLayer = layer
        } catch { onError?(error.localizedDescription) }
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard let code = metadataObjects.compactMap({ $0 as? AVMetadataMachineReadableCodeObject }).first,
              let text = code.stringValue,
              let data = text.data(using: .isoLatin1), !data.isEmpty else { return }
        let now = Date()
        if data == lastPayload && now.timeIntervalSince(lastPayloadAt) < 0.6 { return }
        lastPayload = data
        lastPayloadAt = now
        onFrame?(data)
    }
}
