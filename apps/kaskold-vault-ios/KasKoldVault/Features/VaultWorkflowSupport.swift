import CoreImage
import CoreImage.CIFilterBuiltins
import Foundation
import SwiftUI
import UIKit
import UniformTypeIdentifiers

func toolTitle(_ tool: NativeTool) -> String {
    switch tool {
    case .importRawKey: "Import Raw Key"
    case .importXprv: "Import XPrv"
    case .multisigCreate: "Create Multisig"
    case .multisigImport: "Import Multisig"
    case .bip85: "BIP85 Child Wallet"
    case .signMessage: "Sign Message"
    case .commitSecret: "Commit Secret"
    case .decryptSecret: "Decrypt Secret"
    case .signingPolicy: "Security"
    }
}

func toolFieldOne(_ tool: NativeTool) -> String {
    switch tool {
    case .importRawKey: "Raw private key (64 hex)"
    case .importXprv: "Account XPrv"
    case .multisigCreate: "Threshold (M)"
    case .multisigImport: "Multisig descriptor"
    case .bip85: "Word count (12/24)"
    case .signMessage: "Message"
    case .commitSecret: "Secret"
    case .decryptSecret: "Encrypted payload hex"
    case .signingPolicy: "No-sign-before UTC"
    }
}

func toolFieldTwo(_ tool: NativeTool) -> String? {
    switch tool {
    case .multisigCreate: "Other participant kpubs — one per line"
    case .bip85: "Child index"
    case .signingPolicy: "Weekly windows"
    default: nil
    }
}

func fileWorkflowTitle(_ workflow: NativeFileWorkflow) -> String {
    switch workflow {
    case .recoveryMaterial: "Recovery Material"
    case .walletBackup: "Seed / XPrv Backup"
    case .portableBackup: "Encrypted Wallet Backup"
    case .stegoBackup: "Steganographic Restore"
    case .transaction: "Transaction"
    case .kpub: "kpub (Watch-Only)"
    case .multisigAddress: "Multisig Address"
    case .multisigDescriptor: "Multisig Descriptor"
    case .covenantRestore: "Covenant Restore"
    case .portableExport: "Encrypted Wallet Backup"
    case .portableXprvExport: "Encrypted XPrv Backup"
    case .stegoExport: "Steganographic Backup"
    }
}

func workflowNeedsPassword(_ workflow: NativeFileWorkflow) -> Bool {
    switch workflow {
    case .recoveryMaterial, .walletBackup, .portableBackup, .stegoBackup, .portableExport, .portableXprvExport, .stegoExport: true
    default: false
    }
}

func workflowNeedsFile(_ workflow: NativeFileWorkflow) -> Bool {
    switch workflow {
    case .portableExport, .portableXprvExport: false
    default: true
    }
}

func fileWorkflowHelp(_ workflow: NativeFileWorkflow) -> String {
    switch workflow {
    case .stegoExport:
        "Choose a baseline JPEG carrier. The shared Rust steganographic codec will embed an authenticated encrypted backup."
    case .transaction:
        "Choose a transaction file. The Rust Vault runtime will parse it and enter the same review state used by QR scanning."
    case .kpub, .multisigAddress, .multisigDescriptor, .covenantRestore:
        "Choose the corresponding M5-compatible file. It will be validated by the shared Rust runtime before display or import."
    case .portableExport, .portableXprvExport:
        "Create a portable authenticated encrypted backup for local file storage. This is not presented as the M5 device-bound SD secret."
    default:
        "Choose the M5-compatible recovery/backup material and complete the explicit import operation."
    }
}

struct VaultDocumentImportButton: View {
    let label: String
    let onData: (Data) -> Void
    @State private var presented = false

    var body: some View {
        Button(label) { presented = true }
            .buttonStyle(.borderedProminent)
            .fileImporter(isPresented: $presented, allowedContentTypes: [.item], allowsMultipleSelection: false) { result in
                guard case .success(let urls) = result, let url = urls.first else { return }
                let scoped = url.startAccessingSecurityScopedResource()
                defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                do {
                    let values = try url.resourceValues(forKeys: [.fileSizeKey])
                    if let size = values.fileSize, size > 32 * 1024 * 1024 { return }
                    onData(try Data(contentsOf: url, options: [.mappedIfSafe]))
                } catch { return }
            }
    }
}

struct VaultDataDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }
    let data: Data

    init(data: Data) { self.data = data }
    init(configuration: ReadConfiguration) throws {
        data = configuration.file.regularFileContents ?? Data()
    }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}

struct VaultDocumentExportButton: View {
    let data: Data
    let filename: String
    let onSaved: () -> Void
    @State private var presented = false

    var body: some View {
        Button("Save File") { presented = true }
            .buttonStyle(.borderedProminent)
            .fileExporter(
                isPresented: $presented,
                document: VaultDataDocument(data: data),
                contentType: .data,
                defaultFilename: filename
            ) { result in
                if case .success = result { onSaved() }
            }
    }
}

struct QRImage: View {
    let data: Data
    private let context = CIContext()

    var body: some View {
        if let image = image {
            Image(uiImage: image).resizable().interpolation(.none).scaledToFit().accessibilityLabel("KasKold QR code")
        } else { Text("QR generation failed") }
    }

    private var image: UIImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = data
        filter.correctionLevel = "M"
        guard let output = filter.outputImage?.transformed(by: CGAffineTransform(scaleX: 8, y: 8)),
              let cgImage = context.createCGImage(output, from: output.extent) else { return nil }
        return UIImage(cgImage: cgImage)
    }
}

struct AnimatedQRFrames: View {
    let frames: [Data]
    var body: some View {
        TimelineView(.periodic(from: .now, by: 0.8)) { context in
            let index = frames.isEmpty ? 0 : Int(context.date.timeIntervalSinceReferenceDate / 0.8) % frames.count
            VStack {
                if !frames.isEmpty { QRImage(data: frames[index]); Text("Frame \(index + 1)/\(frames.count)").font(.caption) }
            }
        }
    }
}
