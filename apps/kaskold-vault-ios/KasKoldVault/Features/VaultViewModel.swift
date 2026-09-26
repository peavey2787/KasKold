import Foundation
import LocalAuthentication
import SwiftUI

struct NativeWalletSummary: Identifiable {
    let index: Int
    let name: String
    let active: Bool
    let kind: String
    let kpub: String?
    var id: Int { index }
}

enum NativeTool: Equatable {
    case importRawKey, importXprv, multisigCreate, multisigImport, bip85, signMessage, commitSecret, decryptSecret, signingPolicy
}

enum NativeFileWorkflow: Equatable {
    case recoveryMaterial, walletBackup, portableBackup, stegoBackup, transaction, kpub, multisigAddress, multisigDescriptor, covenantRestore, portableExport, portableXprvExport, stegoExport
}

@MainActor
final class VaultViewModel: ObservableObject {
    enum Screen {
        case locked
        case mainMenu
        case creationName(String)
        case creationWordCount
        case creationDiceChoice
        case creationDiceCount([Int])
        case creationDiceRoll(Int, Int)
        case creationTouchChoice
        case creationTouch(Int, Int)
        case creationPassphraseChoice
        case creationPassphrase
        case creationRecoveryAcknowledgement
        case creationStorageFinalize
        case creationStorageProtection
        case connect(String)
        case walletMenu
        case backup(String)
        case backupMethods
        case recoveryWords(String)
        case seedQR(String, Data)
        case advancedBackup
        case xprvExport
        case secretText(String, String, String)
        case exportKey
        case walletDetails(String)
        case walletAdvanced
        case settings
        case recoveryMenu
        case multisigMenu
        case receive(String, Bool, Int)
        case walletInventory([NativeWalletSummary])
        case toolForm(NativeTool)
        case fileWorkflow(NativeFileWorkflow)
        case exportFile(String, String, String, Data)
        case info(String, String)
        case restore
        case scanning(Int, Int)
        case review(VaultReview)
        case response([Data])
        case failure(String)
    }

    @Published private(set) var homeReached = false
    @Published var screen: Screen = .locked {
        didSet {
            if case .mainMenu = screen { homeReached = true }
            if case .locked = screen { homeReached = false }
        }
    }
    @Published var restorePhrase = ""
    @Published var restorePassphrase = ""
    @Published var exportKeyIndex = "0"
    @Published var toolValue1 = ""
    @Published var toolValue2 = ""
    @Published var workflowPassword = ""
    @Published var receiveIndex = "0"

    let runtime: VaultRuntimeBridge
    let blobStore = VaultBlobStore()
    let keyStore = VaultWrappingKeyStore()
    let creationFlow: CreationFlowConfig
    var creationSession: CreationSession?
    var automaticPersistence = true
    var creationPhrase: String?
    private var advancedReturnsToCreation = false
    private var seedQRReturnsToAdvanced = false
    private var failureReturn: Screen = .mainMenu
    var workflowReturn: Screen = .walletMenu
    private var restoreAddsWallet = false
    private var restoreReturn: Screen = .locked

    init() {
        do {
            let runtime = try VaultRuntimeBridge()
            self.runtime = runtime
            let flow = try runtime.workflowText("creation_flow")
            guard let targets = flow["diceRollTargets"] as? [Int],
                  let touchTarget = flow["touchEntropyTarget"] as? Int,
                  !targets.isEmpty, targets.allSatisfy({ $0 > 0 }), touchTarget > 0 else {
                throw VaultBridgeError.malformedResponse
            }
            creationFlow = CreationFlowConfig(diceTargets: targets, touchTarget: touchTarget)
        } catch {
            fatalError("KasKold Vault runtime initialization failed: \(error.localizedDescription)")
        }
    }

    func unlockSaved() {
        failureReturn = .locked
        Task { @MainActor in
            let context = LAContext()
            context.localizedCancelTitle = "Cancel"
            do {
                guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: nil) else {
                    screen = .failure("Device owner authentication is required to unlock a saved KasKold Vault.")
                    return
                }
                let authenticated = try await context.evaluatePolicy(
                    .deviceOwnerAuthentication,
                    localizedReason: "Authenticate to unlock KasKold Vault private key custody."
                )
                guard authenticated else {
                    screen = .failure("Vault authentication failed.")
                    return
                }
                perform(returnTo: .locked) {
                    guard var sealed = try blobStore.readSealedWallet() else { return .locked }
                    var key = try keyStore.loadOrCreate()
                    defer { sealed.secureErase(); key.secureErase() }
                    _ = try runtime.unlockSealedWallet(&sealed, wrappingKey: &key)
                    automaticPersistence = true
                    return .mainMenu
                }
            } catch {
                screen = .failure("Vault authentication failed: \(error.localizedDescription)")
            }
        }
    }

    func restore() { perform(returnTo: .restore) {
        let phrase = restorePhrase.trimmingCharacters(in: .whitespacesAndNewlines)
        if restoreAddsWallet {
            _ = try runtime.workflowText("add_restore", input: ["phrase": phrase, "passphrase": restorePassphrase])
        } else {
            _ = try runtime.restoreWallet(phrase: phrase, passphrase: restorePassphrase)
        }
        restorePhrase = ""
        restorePassphrase = ""
        try persistUnlocked()
        let destination: Screen = restoreAddsWallet ? .walletMenu : .mainMenu
        restoreAddsWallet = false
        return destination
    } }

    func finishBackup() {
        advancedReturnsToCreation = false
        if creationSession != nil {
            screen = .creationRecoveryAcknowledgement
        } else {
            creationPhrase = nil
            screen = .mainMenu
        }
    }

    func mainMenu() { screen = .mainMenu }
    func walletMenu() { screen = .walletMenu }
    func backupMethods() { screen = .backupMethods }
    func walletAdvanced() { screen = .walletAdvanced }
    func settings() { screen = .settings }
    var homeShortcutVisible: Bool {
        guard homeReached else { return false }
        switch screen {
        case .mainMenu, .locked: return false
        default: return true
        }
    }
    func showRestore(addToInventory: Bool = false, returnTo: Screen = .locked) {
        restoreAddsWallet = addToInventory
        restoreReturn = returnTo
        screen = .restore
    }

    func cancelRestore() {
        restorePhrase = ""
        restorePassphrase = ""
        restoreAddsWallet = false
        screen = restoreReturn
    }

    func connect() { perform { .connect(try runtime.exportPublicAccount()) } }
    func walletDetails() { perform(returnTo: .walletMenu) { .walletDetails(try runtime.exportPublicAccount()) } }
    func revealWords() { perform(returnTo: .backupMethods) { .recoveryWords(try runtime.backupWords()) } }

    func advancedBackup(fromCreation: Bool) {
        advancedReturnsToCreation = fromCreation
        screen = .advancedBackup
    }

    func backFromAdvanced() {
        if advancedReturnsToCreation, let creationPhrase { screen = .backup(creationPhrase) }
        else { screen = .backupMethods }
    }

    func seedQR(compact: Bool, title: String, returnToAdvanced: Bool) {
        seedQRReturnsToAdvanced = returnToAdvanced
        perform(returnTo: returnToAdvanced ? .advancedBackup : .backupMethods) {
            .seedQR(title, try runtime.backupSeedQR(compact: compact))
        }
    }

    func plainSeedQR() {
        seedQRReturnsToAdvanced = true
        perform(returnTo: .advancedBackup) { .seedQR("Plain-text SeedQR", Data(try runtime.backupWords().utf8)) }
    }

    func backFromSeedQR() { screen = seedQRReturnsToAdvanced ? .advancedBackup : .backupMethods }

    func xprvBackup() { screen = .xprvExport }

    func showXprvQR() {
        workflowReturn = .xprvExport
        perform(returnTo: .xprvExport) {
            .secretText(
                "XPrv Backup",
                "This XPrv can control the wallet account. Keep it private and offline.",
                try runtime.backupXPrv()
            )
        }
    }


    func exportReceiveKey() {
        workflowReturn = .exportKey
        guard let index = Int(exportKeyIndex), (0...65535).contains(index) else {
            fail("Address index must be between 0 and 65535.", returnTo: .exportKey)
            return
        }
        perform(returnTo: .exportKey) {
            .secretText(
                "Export Key",
                "This private key can spend funds controlled by its receive address. Keep it private and offline.",
                try runtime.exportReceiveKey(index: index)
            )
        }
    }

    func finishWorkflow() { screen = workflowReturn }
    func info(_ title: String, body: String, returnTo: Screen) { workflowReturn = returnTo; screen = .info(title, body) }

    func prettyJSON(_ value: [String: Any]) throws -> String {
        let data = try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys])
        guard let text = String(data: data, encoding: .utf8) else { throw VaultBridgeError.malformedResponse }
        return text
    }

    func review(from object: [String: Any]) throws -> VaultReview {
        try runtime.parseReview(object)
    }

    func beginScan() { perform { try runtime.beginScan(); return .scanning(0, 0) } }

    func accept(frame: Data) {
        do {
            let result = try runtime.accept(frame: frame)
            if let review = result.review { screen = .review(review) }
            else if let progress = result.progress { screen = .scanning(progress.0, progress.1) }
        } catch { fail(error.localizedDescription, returnTo: .mainMenu) }
    }

    func approve() { perform(returnTo: .mainMenu) { .response(try runtime.approve()) } }
    func reject() { runtime.reject(); screen = .mainMenu }
    func finishResponse() { reject() }
    func lock() {
        runtime.lock()
        creationPhrase = nil
        clearCreationSession()
        screen = .locked
    }

    func failFromScanner(_ message: String) { fail(message, returnTo: .mainMenu) }
    func returnFromFailure() { screen = failureReturn }

    func deleteWallet() {
        do {
            runtime.lock()
            clearCreationSession()
            try blobStore.delete()
            try keyStore.delete()
            creationPhrase = nil
            screen = .locked
        } catch { fail(error.localizedDescription, returnTo: .settings) }
    }

    func persistUnlocked(force: Bool = false) throws {
        if !force && !automaticPersistence { return }
        var key = try keyStore.loadOrCreate()
        defer { key.secureErase() }
        var sealed = try runtime.sealWallet(wrappingKey: &key)
        defer { sealed.secureErase() }
        try blobStore.writeSealedWallet(sealed)
    }

    func perform(returnTo: Screen = .mainMenu, _ operation: () throws -> Screen) {
        do { screen = try operation() }
        catch { fail(error.localizedDescription, returnTo: returnTo) }
    }

    func fail(_ message: String, returnTo: Screen) {
        failureReturn = returnTo
        screen = .failure(message)
    }
}
