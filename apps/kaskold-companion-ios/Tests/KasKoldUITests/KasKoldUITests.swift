import XCTest

final class KasKoldUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testSharedCompanionSurfaceRendersStyledAtMobileScale() {
        let app = XCUIApplication()
        app.launchArguments += ["-ui-testing"]
        app.launch()

        let webView = app.webViews.firstMatch
        XCTAssertTrue(webView.waitForExistence(timeout: 20))
        XCTAssertFalse(app.staticTexts["Companion failed to render"].exists)

        let welcome = webView.staticTexts["KasKold Companion"]
        XCTAssertTrue(welcome.waitForExistence(timeout: 20))

        let loadWallet = webView.buttons["Manage Wallets"]
        XCTAssertTrue(loadWallet.waitForExistence(timeout: 20))
        XCTAssertGreaterThan(loadWallet.frame.width, webView.frame.width * 0.55)

        XCTAssertFalse(webView.staticTexts["Verify Address"].exists)
        XCTAssertFalse(webView.staticTexts["Transaction History"].exists)
    }

    func testApplicationRelaunchSurvivesProcessRecreation() {
        let app = XCUIApplication()
        app.launchArguments += ["-ui-testing"]
        app.launch()
        assertCompanionRendered(app)
        app.terminate()
        XCTAssertTrue(app.wait(for: .notRunning, timeout: 30))
        app.launch()
        assertCompanionRendered(app)
    }

    /// Waits for the companion surface so lifecycle actions never race a cold WebView boot.
    private func assertCompanionRendered(_ app: XCUIApplication) {
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 30))
        let welcome = app.webViews.firstMatch.staticTexts["KasKold Companion"]
        XCTAssertTrue(welcome.waitForExistence(timeout: 30))
    }
}
