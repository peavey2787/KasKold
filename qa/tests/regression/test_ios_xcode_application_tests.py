import importlib.util
import pathlib
import unittest
ROOT=pathlib.Path(__file__).resolve().parents[3]
class IOSXcodeApplicationTests(unittest.TestCase):
    def test_real_xcode_test_targets_and_scheme_are_wired(self):
        pbx=(ROOT/'apps/kaskold-companion-ios/KasKold.xcodeproj/project.pbxproj').read_text()
        scheme=(ROOT/'apps/kaskold-companion-ios/KasKold.xcodeproj/xcshareddata/xcschemes/KasKold.xcscheme').read_text()
        self.assertIn('com.apple.product-type.bundle.unit-test',pbx)
        self.assertIn('com.apple.product-type.bundle.ui-testing',pbx)
        self.assertIn('KasKoldAppTests.xctest',scheme)
        self.assertIn('KasKoldUITests.xctest',scheme)
        debug_start = pbx.index('B167C47E5CA0508B907D5DAB /* Debug */')
        release_start = pbx.index('FFDA01A37DFC562DBB92FAEB /* Release */')
        self.assertIn(
            'ENABLE_TESTABILITY = YES;',
            pbx[debug_start:release_start],
            'KasKold Debug target must enable testability for @testable XCTest imports',
        )
        app=(ROOT/'apps/kaskold-companion-ios/Tests/KasKoldAppTests/KasKoldAppTests.swift').read_text()
        ui=(ROOT/'apps/kaskold-companion-ios/Tests/KasKoldUITests/KasKoldUITests.swift').read_text()
        self.assertIn('@testable import KasKold',app)
        self.assertIn('testAppLockImmediateBackgroundTransitionRelocks',app)
        self.assertIn('testNativeAppearancePreferenceRemainsShellOnly',app)
        self.assertIn('sceneDidEnterBackground()',app)
        lock=(ROOT/'apps/kaskold-companion-ios/KasKold/Infrastructure/Security/AppLockService.swift').read_text()
        self.assertIn('authenticationSessionFactory: (() -> AuthenticationSession)? = nil', lock)
        self.assertIn(
            'self.authenticationSessionFactory = authenticationSessionFactory ?? AppLockService.liveAuthenticationSession',
            lock,
        )
        self.assertNotIn(
            'authenticationSessionFactory: @escaping () -> AuthenticationSession = AppLockService.liveAuthenticationSession',
            lock,
            'main-actor live authentication must not be invoked from a nonisolated default argument',
        )
        self.assertIn('XCUIApplication()',ui)
        self.assertIn('testSharedCompanionSurfaceRendersStyledAtMobileScale',ui)
        self.assertIn('webView.buttons["Manage Wallets"]',ui)
        self.assertIn('Companion failed to render',ui)
        self.assertIn('XCTAssertGreaterThan(loadWallet.frame.width, webView.frame.width * 0.55)',ui)
        self.assertIn('app.terminate()',ui)
        web=(ROOT/'apps/kaskold-companion-ios/KasKold/Features/Root/Components/CompanionWebView.swift').read_text()
        loopback=(ROOT/'apps/kaskold-companion-ios/KasKold/Features/Root/Components/CompanionLoopbackServer.swift').read_text()
        self.assertNotIn('mobileViewportBootstrap', web)
        self.assertNotIn('webView.pageZoom', web)
        self.assertNotIn('scrollView.minimumZoomScale', web)
        self.assertNotIn('scrollView.maximumZoomScale', web)
        self.assertIn('renderHealthCheck', web)
        self.assertIn('reportLoadState', web)
        self.assertIn('httpResponseHeader', loopback)
        self.assertIn('joined(separator: "\\r\\n")', loopback)
        self.assertIn('contentContext: .finalMessage', loopback)
        self.assertIn('isComplete: true', loopback)
        self.assertIn('"",\n            "",', loopback)

    def test_ios_test_destination_is_selected_from_available_iphone_simulators(self):
        build=(ROOT/'scripts/mac/build/ios-build.sh').read_text()
        self.assertIn('destination="${KASKOLD_IOS_TEST_DESTINATION:-}"', build)
        self.assertIn('tools/build/ios/select_simulator.py', build)
        self.assertNotIn('KASKOLD_IOS_TEST_DESTINATION:-platform=iOS Simulator,name=iPhone 16 Pro', build)

        selector_path=ROOT/'tools/build/ios/select_simulator.py'
        spec=importlib.util.spec_from_file_location('kaskold_ios_simulator_selector', selector_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module=importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        payload={
            'devices': {
                'com.apple.CoreSimulator.SimRuntime.iOS-26-2': [
                    {'name': 'iPhone 16e', 'udid': 'IPHONE16E-262', 'isAvailable': True},
                    {'name': 'iPhone 17 Pro', 'udid': 'IPHONE17PRO-262', 'isAvailable': True},
                ],
                'com.apple.CoreSimulator.SimRuntime.iOS-26-5': [
                    {'name': 'iPhone 17 Pro', 'udid': 'IPHONE17PRO-265', 'isAvailable': True},
                    {'name': 'iPhone 17', 'udid': 'IPHONE17-265', 'isAvailable': True},
                ],
                'com.apple.CoreSimulator.SimRuntime.visionOS-26-5': [
                    {'name': 'Apple Vision Pro', 'udid': 'VISION-265', 'isAvailable': True},
                ],
            }
        }
        self.assertEqual(
            module.select_destination(payload),
            'platform=iOS Simulator,id=IPHONE17PRO-265',
        )

if __name__=='__main__': unittest.main()
