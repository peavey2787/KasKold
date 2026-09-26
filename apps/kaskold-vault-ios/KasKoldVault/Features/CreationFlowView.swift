import Foundation
import SwiftUI

struct CreationNameView: View {
    let onContinue: (String) -> Void
    let onCancel: () -> Void
    @State private var name: String

    init(suggestedName: String, onContinue: @escaping (String) -> Void, onCancel: @escaping () -> Void) {
        self.onContinue = onContinue
        self.onCancel = onCancel
        _name = State(initialValue: suggestedName)
    }

    var body: some View {
        Group {
            Text("Wallet Name").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            TextField("Wallet name", text: $name).textFieldStyle(.roundedBorder)
            Button("Continue") { onContinue(name) }.buttonStyle(.borderedProminent)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationWordCountView: View {
    let onSelect: (Int) -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("Recovery Words").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Button("24 Words") { onSelect(24) }.buttonStyle(.borderedProminent)
            Button("12 Words") { onSelect(12) }.buttonStyle(.bordered)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationDiceChoiceView: View {
    let onChoose: (Bool) -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("Add Dice").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("Apple platform CSPRNG entropy is always used. Dice can add extra user entropy.")
            Button("No Dice") { onChoose(false) }.buttonStyle(.borderedProminent)
            Button("Add Dice Rolls") { onChoose(true) }.buttonStyle(.bordered)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationDiceCountView: View {
    let targets: [Int]
    let onSelect: (Int) -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("How Many Dice Rolls?").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            ForEach(targets, id: \.self) { target in
                Button("\(target) Rolls") { onSelect(target) }.buttonStyle(.bordered)
            }
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationDiceRollView: View {
    let collected: Int
    let target: Int
    let onRoll: (Int) -> Void
    let onUndo: () -> Void
    let onReset: () -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("Dice Rolls").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("\(collected)/\(target) rolls collected.")
            HStack {
                ForEach(1...3, id: \.self) { value in
                    Button("\(value)") { onRoll(value) }.buttonStyle(.borderedProminent)
                }
            }
            HStack {
                ForEach(4...6, id: \.self) { value in
                    Button("\(value)") { onRoll(value) }.buttonStyle(.borderedProminent)
                }
            }
            HStack {
                Button("Undo", action: onUndo).buttonStyle(.bordered)
                Button("Reset", action: onReset).buttonStyle(.bordered)
            }
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationTouchChoiceView: View {
    let onChoose: (Bool) -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("Add Touch").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("Touch is optional additive entropy; the platform CSPRNG remains mandatory.")
            Button("No Touch Entropy") { onChoose(false) }.buttonStyle(.borderedProminent)
            Button("Add Touch Entropy") { onChoose(true) }.buttonStyle(.bordered)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationTouchView: View {
    let collected: Int
    let target: Int
    let onSample: (UInt64, Int, Int) -> Void
    let onReset: () -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("Touch Entropy").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("\(collected)/\(target) movement samples collected. Move your finger continuously below.")
            GeometryReader { geometry in
                RoundedRectangle(cornerRadius: 16)
                    .fill(.secondary.opacity(0.18))
                    .overlay(Text("Move finger here").foregroundStyle(.secondary))
                    .contentShape(Rectangle())
                    .gesture(
                        DragGesture(minimumDistance: 0).onChanged { value in
                            let width = max(1, geometry.size.width)
                            let height = max(1, geometry.size.height)
                            let x = min(65535, max(0, Int(value.location.x * 65535 / width)))
                            let y = min(65535, max(0, Int(value.location.y * 65535 / height)))
                            let time = UInt64(ProcessInfo.processInfo.systemUptime * 1_000_000)
                            onSample(time, x, y)
                        }
                    )
            }
            .frame(height: 240)
            Button("Reset", action: onReset).buttonStyle(.bordered)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationPassphraseChoiceView: View {
    let onChoose: (Bool) -> Void
    let onCancel: () -> Void

    var body: some View {
        Group {
            Text("BIP39 Pass.").font(.title2.bold())
            Text("Optional extra secret for your words. It creates a different wallet and is required for recovery.")
            Button("No Passphrase") { onChoose(false) }.buttonStyle(.borderedProminent)
            Button("Use Passphrase") { onChoose(true) }.buttonStyle(.bordered)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationPassphraseView: View {
    let onSubmit: (String, String) -> Void
    let onCancel: () -> Void
    @State private var passphrase = ""
    @State private var confirmation = ""

    var body: some View {
        Group {
            Text("Passphrase").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            SecureField("BIP39 passphrase", text: $passphrase).textFieldStyle(.roundedBorder)
            SecureField("Confirm passphrase", text: $confirmation).textFieldStyle(.roundedBorder)
            Button("Continue") { onSubmit(passphrase, confirmation) }.buttonStyle(.borderedProminent)
            Button("Back", action: onCancel).buttonStyle(.bordered)
        }
    }
}

struct CreationBackupView: View {
    let phrase: String
    let onAdvanced: () -> Void
    let onDone: () -> Void
    @State private var index = 0
    @State private var showAll = false

    private var words: [String] {
        phrase.split(whereSeparator: \.isWhitespace).map(String.init)
    }

    var body: some View {
        Group {
            Text("Recovery Words").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("Write down each recovery word in order and store the backup privately and securely.")
            if showAll {
                Text(words.enumerated().map { "\($0.offset + 1). \($0.element)" }.joined(separator: "\n"))
                    .font(.body.monospaced())
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 12))
                    .textSelection(.enabled)
            } else {
                Text("Word \(index + 1) of \(words.count)").foregroundStyle(.secondary)
                Text(words.indices.contains(index) ? words[index] : "")
                    .font(.largeTitle.monospaced())
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 12))
                    .textSelection(.enabled)
            }
            Button(showAll ? "Show One at a Time" : "Show All") { showAll.toggle() }.buttonStyle(.bordered)
            HStack {
                Button("Previous") { if index > 0 { index -= 1 } }.buttonStyle(.bordered).disabled(showAll || index == 0)
                Button(showAll || index + 1 >= words.count ? "Continue" : "Next") {
                    if showAll || index + 1 >= words.count { onDone() } else { index += 1 }
                }.buttonStyle(.borderedProminent)
            }
            Button("Advanced backup", action: onAdvanced).buttonStyle(.bordered)
        }
    }
}

struct CreationRecoveryAcknowledgementView: View {
    let onDone: () -> Void

    var body: some View {
        Group {
            Text("Recovery Backup").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Text("Your recovery words are the master backup for this wallet.")
            Text("Write them down in order and keep them private, offline, and in a secure location.")
            Text("Anyone who has these words can restore the wallet and spend its funds.")
            Text("Verify your recovery backup before relying on encrypted device storage.")
            Button("I Backed Up My Words", action: onDone).buttonStyle(.borderedProminent)
        }
    }
}

struct CreationStorageFinalizeView: View {
    let onSave: () -> Void
    let onSession: () -> Void

    var body: some View {
        Group {
            Text("Storage").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Button("Save Securely on Device", action: onSave).buttonStyle(.borderedProminent)
            Text("iOS Vault seals the shared Rust wallet inventory with a Keychain-protected device wrapping key.")
            Button("Use for This Session Only", action: onSession).buttonStyle(.bordered)
        }
    }
}

struct CreationStorageProtectionView: View {
    let onProtect: () -> Void
    let onSession: () -> Void

    var body: some View {
        Group {
            Text("PROTECT THIS WALLET?").font(.title2.bold()).frame(maxWidth: .infinity, alignment: .center).multilineTextAlignment(.center)
            Button("Use iOS Device Protection", action: onProtect).buttonStyle(.borderedProminent)
            Text("KasKold stores only sealed ciphertext and relies on the platform Keychain/device security boundary for the wrapping key.")
            Button("Session Only", action: onSession).buttonStyle(.bordered)
        }
    }
}
