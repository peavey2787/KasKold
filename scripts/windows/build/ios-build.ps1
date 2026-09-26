param([ValidateSet('build','release','test')][string]$Mode='build')
throw "KasKold iOS applications require macOS with Xcode. Run 'make ios', 'make ios-vault', 'make ios-release', or 'make ios-test' on a macOS/Xcode host."
