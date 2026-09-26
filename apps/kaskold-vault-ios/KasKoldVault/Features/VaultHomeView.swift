import Foundation
import SwiftUI

struct VaultHomeView: View {
    @StateObject private var model = VaultViewModel()

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 18) {
                    Image("AppIconPreview").resizable().scaledToFit().frame(width: 112, height: 112).accessibilityHidden(true)
                    Image("KasKoldWordmark").resizable().scaledToFit().frame(maxWidth: 420)
                    Text("Air-gapped signer · no Internet capability").font(.subheadline).foregroundStyle(.secondary)
                    Text("v 2.0.0").font(.caption.monospaced()).foregroundStyle(.secondary)
                    content
                }
                .padding(24)
            }
            .navigationTitle("Vault")
            .toolbar {
                if model.homeShortcutVisible {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button { model.mainMenu() } label: {
                            Image("HomeShortcut").resizable().scaledToFit().frame(width: 24, height: 24)
                        }
                        .accessibilityLabel("Home")
                    }
                }
            }
        }
    }

    @ViewBuilder private var content: some View {
        switch model.screen {
        case .locked:
            Text("Wallet secrets remain inside the native Rust signing runtime.").multilineTextAlignment(.center)
            Button("Unlock saved wallet") { model.unlockSaved() }.buttonStyle(.borderedProminent)
            Button("Create Wallet") { model.beginCreation() }.buttonStyle(.borderedProminent)
            Button("Restore wallet") { model.showRestore() }.buttonStyle(.bordered)
        case .creationName(let suggestedName):
            CreationNameView(
                suggestedName: suggestedName,
                onContinue: model.creationSetName,
                onCancel: model.cancelCreation
            )
        case .creationWordCount:
            CreationWordCountView(onSelect: model.creationSetWordCount, onCancel: model.cancelCreation)
        case .creationDiceChoice:
            CreationDiceChoiceView(onChoose: model.creationChooseDice, onCancel: model.cancelCreation)
        case .creationDiceCount(let targets):
            CreationDiceCountView(targets: targets, onSelect: model.creationSetDiceTarget, onCancel: model.cancelCreation)
        case .creationDiceRoll(let collected, let target):
            CreationDiceRollView(
                collected: collected,
                target: target,
                onRoll: model.creationAddDice,
                onUndo: model.creationUndoDice,
                onReset: model.creationResetDice,
                onCancel: model.cancelCreation
            )
        case .creationTouchChoice:
            CreationTouchChoiceView(onChoose: model.creationChooseTouch, onCancel: model.cancelCreation)
        case .creationTouch(let collected, let target):
            CreationTouchView(
                collected: collected,
                target: target,
                onSample: model.creationAddTouch,
                onReset: model.creationResetTouch,
                onCancel: model.cancelCreation
            )
        case .creationPassphraseChoice:
            CreationPassphraseChoiceView(onChoose: model.creationChoosePassphrase, onCancel: model.cancelCreation)
        case .creationPassphrase:
            CreationPassphraseView(onSubmit: model.creationSubmitPassphrase, onCancel: model.cancelCreation)
        case .creationRecoveryAcknowledgement:
            CreationRecoveryAcknowledgementView(onDone: model.acknowledgeCreationBackup)
        case .creationStorageFinalize:
            CreationStorageFinalizeView(
                onSave: { model.creationStorageChoice(save: true) },
                onSession: { model.creationStorageChoice(save: false) }
            )
        case .creationStorageProtection:
            CreationStorageProtectionView(
                onProtect: model.creationUseDeviceProtection,
                onSession: model.creationNoProtection
            )
        case .mainMenu:
            menuTitle("Main Menu")
            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
                homeMenuButton("Connect", image: "HomeConnect") { model.connect() }
                homeMenuButton("Scan QR", image: "HomeScan") { model.beginScan() }
                homeMenuButton("Wallet", image: "HomeWallet") { model.walletMenu() }
                homeMenuButton("Settings", image: "HomeSettings") { model.settings() }
            }
        case .connect(let kpub):
            menuTitle("Connect")
            Text(kpub).font(.caption.monospaced()).textSelection(.enabled)
            QRImage(data: Data(kpub.utf8)).frame(maxWidth: 360)
            Text("Scan this public account in KasKold Companion. It contains no private keys.").font(.caption)
            menuButton("Back") { model.mainMenu() }
        case .walletMenu:
            menuTitle("Wallet")
            menuButton("Receive") { model.receive() }
            menuButton("Backup") { model.backupMethods() }
            menuButton("Recovery") { model.recoveryMenu() }
            menuButton("Wallet Details") { model.walletDetails() }
            menuButton("Switch / Add Wallet") { model.walletInventory() }
            menuButton("Multisig") { model.multisigMenu() }
            menuButton("Advanced") { model.walletAdvanced() }
            menuButton("Back") { model.mainMenu() }
        case .backup(let phrase):
            CreationBackupView(
                phrase: phrase,
                onAdvanced: { model.advancedBackup(fromCreation: true) },
                onDone: model.finishBackup
            )
        case .backupMethods:
            menuTitle("Backup")
            menuButton("View Words") { model.revealWords() }
            menuButton("SeedQR Backup") { model.seedQR(compact: false, title: "SeedQR Backup", returnToAdvanced: false) }
            menuButton("Encrypted SD Card") { model.fileWorkflow(.portableExport, returnTo: .backupMethods) }
            menuButton("Advanced", prominent: true) { model.advancedBackup(fromCreation: false) }
            menuButton("Back") { model.walletMenu() }
        case .recoveryWords(let phrase):
            menuTitle("Recovery Words")
            Text("Anyone with these words can control this wallet. Keep them private and offline.")
            Text(phrase).padding().background(.thinMaterial, in: RoundedRectangle(cornerRadius: 12)).textSelection(.enabled)
            menuButton("Back") { model.backupMethods() }
        case .seedQR(let title, let payload):
            menuTitle(title)
            Text("This QR contains wallet recovery material. Keep it private.")
            QRImage(data: payload).frame(maxWidth: 360)
            menuButton("Back") { model.backFromSeedQR() }
        case .advancedBackup:
            menuTitle("Advanced Backup")
            menuButton("Compact SeedQR") { model.seedQR(compact: true, title: "Compact SeedQR", returnToAdvanced: true) }
            menuButton("Plain-text SeedQR") { model.plainSeedQR() }
            menuButton("Steganographic") { model.fileWorkflow(.stegoExport, returnTo: .advancedBackup) }
            menuButton("XPrv Backup") { model.xprvBackup() }
            menuButton("Export Key") { model.screen = .exportKey }
            menuButton("Back") { model.backFromAdvanced() }
        case .xprvExport:
            menuTitle("XPrv Backup")
            menuButton("Show as QR", prominent: true) { model.showXprvQR() }
            menuButton("Encrypt to SD") { model.fileWorkflow(.portableXprvExport, returnTo: .xprvExport) }
            menuButton("Back") { model.screen = .advancedBackup }
        case .secretText(let title, let warning, let value):
            menuTitle(title)
            Text(warning)
            Text(value).font(.caption.monospaced()).textSelection(.enabled)
            QRImage(data: Data(value.utf8)).frame(maxWidth: 320)
            menuButton("Back") { model.finishWorkflow() }
        case .exportKey:
            menuTitle("Export Key")
            Text("This exports one receive-chain private key. Anyone with it can spend funds controlled by that key.")
            TextField("Address index", text: $model.exportKeyIndex).keyboardType(.numberPad).textFieldStyle(.roundedBorder)
            menuButton("Show Private Key", prominent: true) { model.exportReceiveKey() }
            menuButton("Back") { model.screen = .advancedBackup }
        case .walletDetails(let kpub):
            menuTitle("Wallet Details")
            Text(kpub).font(.caption.monospaced()).textSelection(.enabled)
            menuButton("Back") { model.walletMenu() }
        case .walletAdvanced:
            menuTitle("Advanced")
            menuButton("BIP85 Child Wallet") { model.tool(.bip85, returnTo: .walletAdvanced) }
            menuButton("Sign Message") { model.tool(.signMessage, returnTo: .walletAdvanced) }
            menuButton("Commit Secret") { model.tool(.commitSecret, returnTo: .walletAdvanced) }
            menuButton("Decrypt Secret") { model.tool(.decryptSecret, returnTo: .walletAdvanced) }
            menuButton("Back") { model.walletMenu() }
        case .settings:
            menuTitle("Settings")
            menuButton("Security") { model.tool(.signingPolicy, returnTo: .settings) }
            menuButton("Storage") { model.info("Storage", body: "Wallet custody is sealed by the shared Rust KHV format and wrapped with a Keychain-protected platform key. Portable encrypted backup is available from Wallet → Backup.", returnTo: .settings) }
            menuButton("About") { model.info("About", body: "KasKold Vault 2.0.0 · native Rust custody runtime · offline application surface.", returnTo: .settings) }
            menuButton("Lock Vault") { model.lock() }
            Button("Delete local Vault wallet", role: .destructive) { model.deleteWallet() }.buttonStyle(.bordered)
            menuButton("Back") { model.mainMenu() }
        case .recoveryMenu:
            menuTitle("Recovery")
            menuButton("Seed / XPrv Backup") { model.fileWorkflow(.walletBackup, returnTo: .recoveryMenu) }
            menuButton("Transaction") { model.fileWorkflow(.transaction, returnTo: .recoveryMenu) }
            menuButton("kpub (Watch-Only)") { model.fileWorkflow(.kpub, returnTo: .recoveryMenu) }
            menuButton("Multisig Address") { model.fileWorkflow(.multisigAddress, returnTo: .recoveryMenu) }
            menuButton("Covenant Restore") { model.fileWorkflow(.covenantRestore, returnTo: .recoveryMenu) }
            menuButton("Import Raw Key") { model.tool(.importRawKey, returnTo: .recoveryMenu) }
            menuButton("Back") { model.walletMenu() }
        case .multisigMenu:
            menuTitle("Multisig")
            menuButton("Create Multisig", prominent: true) { model.tool(.multisigCreate, returnTo: .multisigMenu) }
            menuButton("Descriptors") { model.tool(.multisigImport, returnTo: .multisigMenu) }
            menuButton("kpub Multisig QR") { model.multisigKpub() }
            menuButton("Back") { model.walletMenu() }
        case .receive(let address, let change, let index):
            menuTitle("Receive")
            Text(change ? "Change address" : "Receive address")
            Text(address).font(.caption.monospaced()).textSelection(.enabled)
            QRImage(data: Data(address.utf8)).frame(maxWidth: 360)
            HStack {
                Button(change ? "Receive chain" : "Change chain") { model.receive(change: !change, index: index) }.buttonStyle(.bordered)
                Button("Previous") { model.receive(change: change, index: max(0, index - 1)) }.buttonStyle(.bordered)
                Button("Next") { model.receive(change: change, index: index + 1) }.buttonStyle(.bordered)
            }
            TextField("Address index", text: $model.receiveIndex).keyboardType(.numberPad).textFieldStyle(.roundedBorder)
            menuButton("Show Address", prominent: true) { model.receive(change: change, index: Int(model.receiveIndex) ?? index) }
            menuButton("Back") { model.walletMenu() }
        case .walletInventory(let wallets):
            menuTitle("Switch / Add Wallet")
            ForEach(wallets) { wallet in
                VStack(alignment: .leading, spacing: 6) {
                    Text("\(wallet.name)\(wallet.active ? " · active" : "")").bold()
                    Text(wallet.kind).font(.caption)
                    if let kpub = wallet.kpub { Text(kpub).font(.caption2.monospaced()).textSelection(.enabled) }
                    if !wallet.active { Button("Switch") { model.switchWallet(wallet.index) }.buttonStyle(.bordered) }
                }.frame(maxWidth: .infinity, alignment: .leading).padding().background(.thinMaterial, in: RoundedRectangle(cornerRadius: 12))
            }
            menuButton("Create Wallet") {
                model.beginCreation(addToInventory: true, returnTo: .walletInventory(wallets))
            }
            menuButton("Restore Words") { model.showRestore(addToInventory: true, returnTo: .walletMenu) }
            menuButton("Import XPrv") { model.tool(.importXprv, returnTo: .walletMenu) }
            menuButton("Import Raw Key") { model.tool(.importRawKey, returnTo: .walletMenu) }
            menuButton("Back") { model.walletMenu() }
        case .toolForm(let tool):
            menuTitle(toolTitle(tool))
            TextField(toolFieldOne(tool), text: $model.toolValue1, axis: .vertical).textFieldStyle(.roundedBorder).lineLimit(1...8)
            if let second = toolFieldTwo(tool) {
                TextField(second, text: $model.toolValue2, axis: .vertical).textFieldStyle(.roundedBorder).lineLimit(1...8)
            }
            menuButton("Continue", prominent: true) { model.submitTool(tool) }
            menuButton("Back") { model.finishWorkflow() }
        case .fileWorkflow(let workflow):
            menuTitle(fileWorkflowTitle(workflow))
            Text(fileWorkflowHelp(workflow)).font(.caption)
            if workflowNeedsPassword(workflow) { SecureField(workflow == .recoveryMaterial ? "Optional BIP39 passphrase" : "Backup password", text: $model.workflowPassword).textFieldStyle(.roundedBorder) }
            if workflowNeedsFile(workflow) {
                VaultDocumentImportButton(label: "Choose File") { data in
                    var mutable = data
                    model.processFileWorkflow(workflow, data: &mutable)
                    mutable.secureErase()
                }
            } else {
                menuButton("Create Backup", prominent: true) { var empty = Data(); model.processFileWorkflow(workflow, data: &empty); empty.secureErase() }
            }
            menuButton("Back") { model.finishWorkflow() }
        case .exportFile(let title, let filename, _, let data):
            menuTitle(title)
            Text("The backup bytes were created inside the shared Rust Vault runtime. Save them to a location you control.")
            VaultDocumentExportButton(data: data, filename: filename) { model.finishWorkflow() }
            menuButton("Cancel") { model.finishWorkflow() }
        case .info(let title, let body):
            menuTitle(title)
            Text(body)
            menuButton("Back") { model.finishWorkflow() }
        case .restore:
            TextField("Recovery phrase", text: $model.restorePhrase, axis: .vertical).textFieldStyle(.roundedBorder).lineLimit(4...8)
            SecureField("Optional BIP39 passphrase", text: $model.restorePassphrase).textFieldStyle(.roundedBorder)
            HStack {
                Button("Cancel") { model.cancelRestore() }.buttonStyle(.bordered)
                Button("Restore") { model.restore() }.buttonStyle(.borderedProminent).disabled(model.restorePhrase.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        case .scanning(let received, let total):
            Text(total > 0 ? "Scanning · \(received)/\(total) frames" : "Scan signing request").font(.headline)
            QRScanner { model.accept(frame: $0) } onError: { model.failFromScanner($0) }.frame(height: 420)
            Button("Cancel") { model.reject() }.buttonStyle(.bordered)
        case .review(let review):
            Text("Review transaction").font(.title2.bold())
            reviewRow("Network", review.network)
            reviewRow("Inputs", String(review.inputCount))
            reviewRow("Outputs", String(review.outputCount))
            reviewRow("Input total (sompi)", review.inputTotal)
            reviewRow("Output total (sompi)", review.outputTotal)
            reviewRow("Fee (sompi)", review.fee)
            ForEach(review.inputs, id: \.index) { input in
                VStack(alignment: .leading, spacing: 4) {
                    Text("Input \(input.index + 1) · \(input.scriptType)").font(.headline)
                    Text("Outpoint: \(input.outpoint)").font(.caption).textSelection(.enabled)
                    Text("Amount: \(input.amount) sompi")
                    Text("Address: \(input.address ?? "Unavailable")").font(.caption).textSelection(.enabled)
                }.frame(maxWidth: .infinity, alignment: .leading).padding(10)
            }
            ForEach(review.outputs, id: \.index) { output in
                VStack(alignment: .leading, spacing: 4) {
                    Text("Output \(output.index + 1) · \(output.ownership)").font(.headline)
                    Text("Amount: \(output.amount) sompi")
                    Text("Address: \(output.address ?? "Unavailable")").font(.caption).textSelection(.enabled)
                }.frame(maxWidth: .infinity, alignment: .leading).padding(10)
            }
            Text("Scanning never signs. Advanced hidden transaction semantics are rejected; only Approve & Sign authorizes this reviewed KSPT.").font(.caption)
            HStack {
                Button("Reject", role: .cancel) { model.reject() }.buttonStyle(.bordered)
                Button("Approve & Sign") { model.approve() }.buttonStyle(.borderedProminent)
            }
        case .response(let frames):
            Text("Signed response").font(.title2.bold())
            AnimatedQRFrames(frames: frames).frame(maxWidth: 380)
            Text("Scan the response with KasKold Companion.")
            Button("Done") { model.finishResponse() }.buttonStyle(.borderedProminent)
        case .failure(let message):
            Text("Vault operation failed").font(.headline)
            Text(message).foregroundStyle(.red)
            Button("Back") { model.returnFromFailure() }.buttonStyle(.bordered)
        }
    }

    private func menuTitle(_ value: String) -> some View {
        Text(value).font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
    }

    @ViewBuilder private func menuButton(_ label: String, prominent: Bool = false, action: @escaping () -> Void) -> some View {
        if prominent {
            Button(label, action: action).buttonStyle(.borderedProminent)
        } else {
            Button(label, action: action).buttonStyle(.bordered)
        }
    }

    @ViewBuilder private func homeMenuButton(_ label: String, image: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(image).resizable().scaledToFit().frame(width: 56, height: 56)
                Text(label).font(.headline)
            }
            .frame(maxWidth: .infinity, minHeight: 112)
        }
        .buttonStyle(.bordered)
    }

    private func reviewRow(_ label: String, _ value: String) -> some View {
        HStack { Text(label); Spacer(); Text(value).bold() }
    }
}
