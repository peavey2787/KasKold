import Foundation

struct CreationFlowConfig {
    let diceTargets: [Int]
    let touchTarget: Int
}

struct TouchEntropySample {
    let time: UInt32
    let x: UInt16
    let y: UInt16
}

final class CreationSession {
    let addToInventory: Bool
    let returnTo: VaultViewModel.Screen
    var walletName: String
    var wordCount = 24
    var diceRolls: [UInt8] = []
    var diceTarget = 0
    var touchSamples: [TouchEntropySample] = []
    var passphrase = ""

    init(addToInventory: Bool, returnTo: VaultViewModel.Screen, walletName: String) {
        self.addToInventory = addToInventory
        self.returnTo = returnTo
        self.walletName = walletName
    }
}

@MainActor
extension VaultViewModel {
    func beginCreation(addToInventory: Bool = false, returnTo: Screen = .locked) {
        perform(returnTo: returnTo) {
            let result = try runtime.workflowText("wallets")
            let count = (result["wallets"] as? [[String: Any]])?.count ?? 0
            let name = "Wallet \(count + 1)"
            creationSession = CreationSession(addToInventory: addToInventory, returnTo: returnTo, walletName: name)
            return .creationName(name)
        }
    }

    func creationSetName(_ name: String) {
        guard let session = creationSession else { return fail("Wallet creation is not active.", returnTo: .locked) }
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        let bytes = Array(trimmed.utf8)
        guard !trimmed.isEmpty, bytes.count <= 64,
              trimmed.unicodeScalars.allSatisfy({ $0.value >= 0x20 && $0.value != 0x7f }) else {
            return fail("Wallet name must be 1–64 UTF-8 bytes with no control characters.", returnTo: session.returnTo)
        }
        session.walletName = trimmed
        screen = .creationWordCount
    }

    func creationSetWordCount(_ count: Int) {
        guard let session = creationSession, count == 12 || count == 24 else {
            return fail("Wallet word count must be 12 or 24.", returnTo: creationSession?.returnTo ?? .locked)
        }
        session.wordCount = count
        screen = .creationDiceChoice
    }

    func creationChooseDice(_ addDice: Bool) {
        guard let session = creationSession else { return }
        session.diceRolls.removeAll(keepingCapacity: false)
        session.diceTarget = 0
        screen = addDice ? .creationDiceCount(creationFlow.diceTargets) : .creationTouchChoice
    }

    func creationSetDiceTarget(_ target: Int) {
        guard let session = creationSession, creationFlow.diceTargets.contains(target) else {
            return fail("Unsupported dice-roll target.", returnTo: creationSession?.returnTo ?? .locked)
        }
        session.diceTarget = target
        session.diceRolls.removeAll(keepingCapacity: true)
        screen = .creationDiceRoll(0, target)
    }

    func creationAddDice(_ value: Int) {
        guard let session = creationSession, (1...6).contains(value), session.diceTarget > 0 else { return }
        if session.diceRolls.count < session.diceTarget { session.diceRolls.append(UInt8(value)) }
        screen = session.diceRolls.count == session.diceTarget
            ? .creationTouchChoice
            : .creationDiceRoll(session.diceRolls.count, session.diceTarget)
    }

    func creationUndoDice() {
        guard let session = creationSession else { return }
        if !session.diceRolls.isEmpty { session.diceRolls.removeLast() }
        screen = .creationDiceRoll(session.diceRolls.count, session.diceTarget)
    }

    func creationResetDice() {
        guard let session = creationSession else { return }
        session.diceRolls.removeAll(keepingCapacity: true)
        screen = .creationDiceRoll(0, session.diceTarget)
    }

    func creationChooseTouch(_ addTouch: Bool) {
        guard let session = creationSession else { return }
        session.touchSamples.removeAll(keepingCapacity: false)
        screen = addTouch ? .creationTouch(0, creationFlow.touchTarget) : .creationPassphraseChoice
    }

    func creationAddTouch(time: UInt64, x: Int, y: Int) {
        guard let session = creationSession, session.touchSamples.count < creationFlow.touchTarget else { return }
        let sample = TouchEntropySample(
            time: UInt32(truncatingIfNeeded: time),
            x: UInt16(clamping: x),
            y: UInt16(clamping: y)
        )
        if session.touchSamples.last.map({ $0.x == sample.x && $0.y == sample.y }) != true {
            session.touchSamples.append(sample)
        }
        screen = session.touchSamples.count >= creationFlow.touchTarget
            ? .creationPassphraseChoice
            : .creationTouch(session.touchSamples.count, creationFlow.touchTarget)
    }

    func creationResetTouch() {
        guard let session = creationSession else { return }
        session.touchSamples.removeAll(keepingCapacity: true)
        screen = .creationTouch(0, creationFlow.touchTarget)
    }

    func creationChoosePassphrase(_ usePassphrase: Bool) {
        if usePassphrase { screen = .creationPassphrase }
        else { createFromCreationSession(passphrase: "") }
    }

    func creationSubmitPassphrase(_ value: String, confirmation: String) {
        guard value == confirmation else {
            return fail("BIP39 passphrases do not match.", returnTo: .creationPassphrase)
        }
        createFromCreationSession(passphrase: value)
    }

    func acknowledgeCreationBackup() { screen = .creationStorageFinalize }

    func creationStorageChoice(save: Bool) {
        if save { screen = .creationStorageProtection }
        else {
            automaticPersistence = false
            finishCreationDestination()
        }
    }

    func creationUseDeviceProtection() {
        do {
            automaticPersistence = true
            try persistUnlocked(force: true)
            finishCreationDestination()
        } catch {
            fail(error.localizedDescription, returnTo: .creationStorageProtection)
        }
    }

    func creationNoProtection() {
        automaticPersistence = false
        finishCreationDestination()
    }

    func cancelCreation() {
        let destination = creationSession?.returnTo ?? .locked
        clearCreationSession()
        screen = destination
    }

    func clearCreationSession() {
        creationSession?.diceRolls.removeAll(keepingCapacity: false)
        creationSession?.touchSamples.removeAll(keepingCapacity: false)
        creationSession?.passphrase = ""
        creationSession = nil
    }

    private func createFromCreationSession(passphrase: String) {
        guard let session = creationSession else { return }
        session.passphrase = passphrase
        perform(returnTo: session.returnTo) {
            var touch = encodeTouchTranscript(session.touchSamples)
            defer {
                touch.secureErase()
                session.passphrase = ""
            }
            let operation = session.addToInventory ? "add_create_with_entropy" : "create_with_entropy"
            let dice = session.diceRolls.map(String.init).joined()
            let result = try runtime.workflowTextWithBytes(
                operation,
                input: ["wordCount": session.wordCount, "dice": dice, "passphrase": session.passphrase],
                data: &touch
            )
            guard let phrase = result["recoveryPhrase"] as? String else { throw VaultBridgeError.malformedResponse }
            let inventory = try runtime.workflowText("wallets")
            if let wallets = inventory["wallets"] as? [[String: Any]],
               let active = wallets.first(where: { ($0["active"] as? Bool) == true }),
               let index = active["index"] as? Int {
                _ = try runtime.workflowText("set_wallet_name", input: ["index": index, "name": session.walletName])
            }
            creationPhrase = phrase
            return .backup(phrase)
        }
    }

    private func encodeTouchTranscript(_ samples: [TouchEntropySample]) -> Data {
        var data = Data(capacity: samples.count * 8)
        for sample in samples {
            var time = sample.time.littleEndian
            var x = sample.x.littleEndian
            var y = sample.y.littleEndian
            withUnsafeBytes(of: &time) { data.append(contentsOf: $0) }
            withUnsafeBytes(of: &x) { data.append(contentsOf: $0) }
            withUnsafeBytes(of: &y) { data.append(contentsOf: $0) }
        }
        return data
    }

    private func finishCreationDestination() {
        let addToInventory = creationSession?.addToInventory == true
        creationPhrase = nil
        clearCreationSession()
        if addToInventory { walletInventory() }
        else { screen = .mainMenu }
    }
}
