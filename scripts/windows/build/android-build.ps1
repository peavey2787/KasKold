param([ValidateSet('debug','release','test')][string]$Mode='debug')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
$android = Join-Path $root 'apps/kaskold-companion-android'
$vault = Join-Path $root 'apps/kaskold-vault-android'
$wrapperProperties = Join-Path $android 'gradle/wrapper/gradle-wrapper.properties'
$daemonJvmProperties = Join-Path $android 'gradle/gradle-daemon-jvm.properties'
$products = if ($env:KASKOLD_ANDROID_PRODUCTS) { $env:KASKOLD_ANDROID_PRODUCTS.ToLowerInvariant() } else { 'all' }
if ($products -notin @('all','companion','vault')) { throw "Unknown KASKOLD_ANDROID_PRODUCTS=$products (expected all, companion, or vault)." }
$buildCompanion = $products -in @('all','companion')
$buildVault = $products -in @('all','vault')

Import-KasKoldToolchains $root
if (-not (Test-Path -LiteralPath $daemonJvmProperties -PathType Leaf)) { throw "Missing canonical Gradle Daemon JVM criteria: $daemonJvmProperties" }
$daemonJvm = @{}
foreach ($line in Get-Content -LiteralPath $daemonJvmProperties) {
    if ($line -match '^([^=]+)=(.*)$') { $daemonJvm[$Matches[1].Trim()] = $Matches[2].Trim() }
}
$centralJavaText = [string]$env:KASKOLD_ANDROID_JDK
$daemonJavaText = [string]$daemonJvm['toolchainVersion']
if ($centralJavaText -notmatch '^\d+$') { throw 'KASKOLD_ANDROID_JDK is missing or invalid in qa/config/toolchains.env.' }
if ($daemonJavaText -notmatch '^\d+$') { throw 'toolchainVersion is missing or invalid in gradle-daemon-jvm.properties.' }
$requiredJava = [int]$centralJavaText
if ([int]$daemonJavaText -ne $requiredJava) { throw "Gradle Daemon JVM criteria ($daemonJavaText) does not match central Android JDK pin ($requiredJava)." }

function Get-JavaMajor([string]$Java) {
    if (-not $Java) { return 0 }
    if (-not (Test-Path -LiteralPath $Java -PathType Leaf)) { return 0 }
    try {
        $probe = Invoke-KasKoldCapture -Command $Java -Arguments @('-version')
    } catch {
        # A stale or partially prepared managed JDK must be treated as absent
        # so Install-ManagedJava can replace it. Do not let a broken java.exe
        # candidate abort Android QA before the repair path runs.
        return 0
    }
    if ($probe.ExitCode -ne 0) { return 0 }
    if ($probe.Output -match 'version\s+"(?:(?:1\.)?)(\d+)') { return [int]$Matches[1] }
    return 0
}
function Add-JavaCandidate([Collections.Generic.List[string]]$Candidates, [string]$Java) {
    if (-not $Java) { return }
    if (-not (Test-Path -LiteralPath $Java -PathType Leaf)) { return }
    $full = [IO.Path]::GetFullPath($Java)
    foreach ($existing in $Candidates) {
        if ([string]::Equals($existing, $full, [StringComparison]::OrdinalIgnoreCase)) { return }
    }
    $Candidates.Add($full)
}
function Install-ManagedJava([int]$RequiredMajor) {
    $target = Join-Path $HOME ".kaskold/tools/jdk-$RequiredMajor"
    $targetJava = Join-Path $target 'bin/java.exe'
    if ((Get-JavaMajor $targetJava) -eq $RequiredMajor) { return $targetJava }

    $arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'ARM64' { 'aarch64' } default { 'x64' } }
    $metaUri = "https://api.adoptium.net/v3/assets/latest/$RequiredMajor/hotspot?architecture=$arch&image_type=jdk&os=windows&vendor=eclipse"
    Write-Host "==> Pinned JDK $RequiredMajor is not installed; downloading and verifying it under $target"
    $assets = Invoke-RestMethod -Uri $metaUri -UseBasicParsing
    if (-not $assets -or -not $assets[0].binary.package) { throw "Adoptium did not return a Windows JDK $RequiredMajor package for $arch." }
    $package = $assets[0].binary.package
    $link = [string]$package.link
    $checksum = [string]$package.checksum
    if (-not $link -or $checksum -notmatch '^[0-9A-Fa-f]{64}$') { throw "Adoptium JDK $RequiredMajor metadata is incomplete." }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-android-jdk-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        $archive = Join-Path $tmp 'jdk.zip'
        Invoke-WebRequest -UseBasicParsing -Uri $link -OutFile $archive
        $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $checksum.ToLowerInvariant()) { throw "JDK SHA-256 mismatch: expected $checksum, got $actual" }
        $unpack = Join-Path $tmp 'unpack'
        Expand-Archive -LiteralPath $archive -DestinationPath $unpack -Force
        $source = Get-ChildItem -LiteralPath $unpack -Directory | Select-Object -First 1
        if (-not $source) { throw 'JDK archive did not contain a top-level directory.' }
        Remove-KasKoldPath $target
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        Move-Item -LiteralPath $source.FullName -Destination $target
    } finally {
        Remove-KasKoldPath $tmp
    }
    if ((Get-JavaMajor $targetJava) -ne $RequiredMajor) { throw "Prepared JDK does not report required major ${RequiredMajor}: $targetJava" }
    return $targetJava
}

function Resolve-Java([int]$RequiredMajor) {
    $candidates = [Collections.Generic.List[string]]::new()
    if ($env:KASKOLD_ANDROID_JDK) {
        Add-JavaCandidate $candidates (Join-Path $HOME ".kaskold/tools/jdk-$($env:KASKOLD_ANDROID_JDK)/bin/java.exe")
    }
    if ($env:JAVA_HOME) { Add-JavaCandidate $candidates (Join-Path $env:JAVA_HOME 'bin/java.exe') }
    if ($env:ProgramFiles) { Add-JavaCandidate $candidates (Join-Path $env:ProgramFiles 'Android/Android Studio/jbr/bin/java.exe') }
    if ($env:LOCALAPPDATA) { Add-JavaCandidate $candidates (Join-Path $env:LOCALAPPDATA 'Programs/Android Studio/jbr/bin/java.exe') }
    $pathJava = Get-Command java.exe -ErrorAction SilentlyContinue
    if (-not $pathJava) { $pathJava = Get-Command java -ErrorAction SilentlyContinue }
    if ($pathJava) { Add-JavaCandidate $candidates $pathJava.Source }

    foreach ($candidate in $candidates) {
        $major = Get-JavaMajor $candidate
        if ($major -eq $RequiredMajor) {
            return [pscustomobject]@{ Path = $candidate; Major = $major }
        }
    }

    $managed = Install-ManagedJava $RequiredMajor
    return [pscustomobject]@{ Path = $managed; Major = $RequiredMajor }
}

$java = Resolve-Java $requiredJava
$javaBin = Split-Path -Parent $java.Path
$env:JAVA_HOME = Split-Path -Parent $javaBin
$env:PATH = $javaBin + [IO.Path]::PathSeparator + $env:PATH

function Read-LocalSdk {
    $properties = Join-Path $android 'local.properties'
    if (-not (Test-Path -LiteralPath $properties -PathType Leaf)) { return $null }
    foreach ($line in Get-Content -LiteralPath $properties) {
        if ($line -match '^\s*sdk\.dir\s*=\s*(.+?)\s*$') {
            return ($Matches[1] -replace '\\\\','\' -replace '\\:',':' -replace '\\ ',' ').Trim()
        }
    }
    return $null
}
function Looks-LikeAndroidSdk([string]$Path) {
    return $Path -and (Test-Path -LiteralPath $Path -PathType Container) -and (
        (Test-Path -LiteralPath (Join-Path $Path 'platforms') -PathType Container) -or
        (Test-Path -LiteralPath (Join-Path $Path 'platform-tools') -PathType Container) -or
        (Test-Path -LiteralPath (Join-Path $Path 'cmdline-tools') -PathType Container)
    )
}
function Find-AndroidSdk {
    $candidates = [Collections.Generic.List[string]]::new()
    foreach ($candidate in @($env:KASKOLD_ANDROID_SDK_ROOT, (Read-LocalSdk), $env:ANDROID_SDK_ROOT, $env:ANDROID_HOME)) {
        if ($candidate) { $candidates.Add($candidate) }
    }
    if ($env:LOCALAPPDATA) { $candidates.Add((Join-Path $env:LOCALAPPDATA 'Android/Sdk')) }
    $candidates.Add((Join-Path $HOME 'AppData/Local/Android/Sdk'))
    $candidates.Add((Join-Path $HOME 'Android/Sdk'))
    foreach ($candidate in $candidates) {
        if (Looks-LikeAndroidSdk $candidate) { return [IO.Path]::GetFullPath($candidate) }
    }
    return $null
}
function Find-AndroidPlatformJar([string]$Sdk, [int]$RequiredApi) {
    $platforms = Join-Path $Sdk 'platforms'
    if (-not (Test-Path -LiteralPath $platforms -PathType Container)) { return $null }
    foreach ($directory in Get-ChildItem -LiteralPath $platforms -Directory -ErrorAction SilentlyContinue | Sort-Object Name) {
        $jar = Join-Path $directory.FullName 'android.jar'
        if (-not (Test-Path -LiteralPath $jar -PathType Leaf)) { continue }
        $api = $null
        $source = Join-Path $directory.FullName 'source.properties'
        if (Test-Path -LiteralPath $source -PathType Leaf) {
            $text = Get-Content -LiteralPath $source -Raw
            if ($text -match '(?m)^AndroidVersion\.ApiLevel\s*=\s*(\d+)\s*$') { $api = [int]$Matches[1] }
        }
        if ($null -eq $api -and $directory.Name -match '^android-(\d+)(?:\.\d+)?$') { $api = [int]$Matches[1] }
        if ($api -eq $RequiredApi) { return $jar }
    }
    return $null
}

$sdk = Find-AndroidSdk
if (-not $sdk) { throw 'Android SDK was not found. Configure apps/kaskold-companion-android/local.properties sdk.dir, ANDROID_SDK_ROOT/ANDROID_HOME, or KASKOLD_ANDROID_SDK_ROOT.' }
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_HOME = $sdk

# KasKold Vault carries a Rust static library built with cargo-ndk. Prepare
# the centrally pinned Rust/Android native toolchain here so `make android-qa` remains
# self-contained: missing Rust Android targets, cargo-ndk, or the pinned NDK
# are installed and then re-verified before Gradle starts. Companion-only
# builds intentionally skip this Vault-specific bootstrap.
function Ensure-RustAndroidTargets([string]$Toolchain) {
    $installed = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('target','list','--toolchain',$Toolchain,'--installed')
    if ($installed.ExitCode -ne 0) { throw "Unable to inspect Rust targets for $Toolchain." }
    $installedNames = @($installed.Output -split "`r?`n" | Where-Object { $_ })
    foreach ($rustTarget in @('aarch64-linux-android','x86_64-linux-android')) {
        if ($installedNames -contains $rustTarget) { continue }
        Write-Host "==> Installing pinned Rust target $rustTarget for $Toolchain"
        Invoke-KasKoldCommand -Command 'rustup' -Arguments @('target','add',$rustTarget,'--toolchain',$Toolchain) | Out-Null
    }
    $verified = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('target','list','--toolchain',$Toolchain,'--installed')
    if ($verified.ExitCode -ne 0) { throw "Unable to verify Rust targets for $Toolchain after installation." }
    $verifiedNames = @($verified.Output -split "`r?`n" | Where-Object { $_ })
    foreach ($rustTarget in @('aarch64-linux-android','x86_64-linux-android')) {
        if ($verifiedNames -notcontains $rustTarget) { throw "Rust target $rustTarget could not be prepared for $Toolchain." }
    }
}

function Ensure-CargoNdk([string]$Toolchain, [string]$Version) {
    $probe = Invoke-KasKoldCapture -Command 'cargo' -Arguments @("+$Toolchain",'ndk','--version')
    $installedVersion = $null
    if ($probe.ExitCode -eq 0 -and $probe.Output -match 'cargo-ndk[^0-9]*([0-9]+\.[0-9]+\.[0-9]+)') { $installedVersion = $Matches[1] }
    if ($installedVersion -ne $Version) {
        Write-Host "==> Installing pinned cargo-ndk $Version for Rust $Toolchain"
        Invoke-KasKoldCommand -Command 'cargo' -Arguments @("+$Toolchain",'install','cargo-ndk','--version',$Version,'--locked','--force') | Out-Null
    }
    $verified = Invoke-KasKoldCapture -Command 'cargo' -Arguments @("+$Toolchain",'ndk','--version')
    if ($verified.ExitCode -ne 0 -or $verified.Output -notmatch 'cargo-ndk[^0-9]*([0-9]+\.[0-9]+\.[0-9]+)' -or $Matches[1] -ne $Version) {
        throw "Pinned cargo-ndk $Version could not be prepared for Rust $Toolchain."
    }
}

function Ensure-AndroidCommandLineTools([string]$Sdk) {
    $tools = Join-Path $Sdk 'cmdline-tools/latest'
    $sdkmanager = Join-Path $tools 'bin/sdkmanager.bat'
    if (Test-Path -LiteralPath $sdkmanager -PathType Leaf) { return $sdkmanager }
    $revision = [string]$env:KASKOLD_ANDROID_CMDLINE_TOOLS
    $expectedSha = [string]$env:KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256
    if (-not $revision -or $expectedSha -notmatch '^[0-9A-Fa-f]{64}$') {
        throw 'Pinned Android command-line tools metadata is missing or invalid in qa/config/toolchains.env.'
    }
    Write-Host "==> Android command-line tools $revision are not installed; downloading and verifying them under $Sdk"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-android-tools-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        $archive = Join-Path $tmp 'cmdline-tools.zip'
        $url = "https://dl.google.com/android/repository/commandlinetools-win-$($revision)_latest.zip"
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
        $actualSha = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualSha -ne $expectedSha.ToLowerInvariant()) { throw "Android command-line tools SHA-256 mismatch: expected $expectedSha, got $actualSha" }
        $unpack = Join-Path $tmp 'unpack'
        Expand-Archive -LiteralPath $archive -DestinationPath $unpack -Force
        $source = Join-Path $unpack 'cmdline-tools'
        if (-not (Test-Path -LiteralPath (Join-Path $source 'bin/sdkmanager.bat') -PathType Leaf)) { throw 'Android command-line tools archive is missing sdkmanager.bat.' }
        Remove-KasKoldPath $tools
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $tools) | Out-Null
        Move-Item -LiteralPath $source -Destination $tools
    } finally {
        Remove-KasKoldPath $tmp
    }
    if (-not (Test-Path -LiteralPath $sdkmanager -PathType Leaf)) { throw 'Pinned Android command-line tools could not be prepared.' }
    return $sdkmanager
}

function Convert-ToLegacyAndroidPackage([string]$Package) {
    $slash = $Package.IndexOf('/')
    if ($slash -lt 1) { return $Package }
    return $Package.Substring(0, $slash) + ';' + $Package.Substring($slash + 1)
}

function Install-AndroidSdkPackages([string]$Sdk, [string[]]$Packages) {
    if (-not $Packages -or $Packages.Count -eq 0) { return }
    $sdkmanager = Ensure-AndroidCommandLineTools $Sdk
    $androidCli = Join-Path $Sdk 'cmdline-tools/latest/bin/android.bat'
    if (Test-Path -LiteralPath $androidCli -PathType Leaf) {
        $cliArgs = @("--sdk=$Sdk", 'sdk', 'install', '--canary') + @($Packages)
        1..100 | ForEach-Object { 'y' } | & $androidCli @cliArgs
        if ($LASTEXITCODE -eq 0) { return }
        Write-Warning 'Android CLI package installation failed; retrying with sdkmanager preview-channel compatibility mode.'
    }

    1..100 | ForEach-Object { 'y' } | & $sdkmanager --sdk_root=$Sdk --licenses | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Android SDK license acceptance failed.' }
    $legacyPackages = @($Packages | ForEach-Object { Convert-ToLegacyAndroidPackage $_ })
    & $sdkmanager "--sdk_root=$Sdk" '--channel=3' @legacyPackages
    if ($LASTEXITCODE -ne 0) { throw "Android SDK package installation failed: $($Packages -join ', ')" }
}

function Ensure-AndroidSdkBuildPackages([string]$Sdk, [int]$Api, [string]$BuildToolsVersion) {
    if ($Api -le 0) { throw "Android API pin is invalid: $Api" }
    if (-not $BuildToolsVersion) { throw 'KASKOLD_ANDROID_BUILD_TOOLS is missing from qa/config/toolchains.env.' }
    $platformJar = Find-AndroidPlatformJar $Sdk $Api
    $buildTools = Join-Path $Sdk "build-tools/$BuildToolsVersion"
    $buildToolsReady = Test-Path -LiteralPath (Join-Path $buildTools 'aapt2.exe') -PathType Leaf
    if ($platformJar -and $buildToolsReady) { return $platformJar }

    Write-Host "==> Preparing pinned Android SDK API $Api / build-tools $BuildToolsVersion under $Sdk"
    $platformCandidates = @("platforms/android-$Api", "platforms/android-$Api.0")
    $lastFailure = $null
    foreach ($platformPackage in $platformCandidates) {
        $packages = [Collections.Generic.List[string]]::new()
        if (-not $platformJar) { $packages.Add($platformPackage) }
        if (-not $buildToolsReady) { $packages.Add("build-tools/$BuildToolsVersion") }
        try {
            Install-AndroidSdkPackages $Sdk @($packages)
        } catch {
            $lastFailure = $_
            continue
        }
        $platformJar = Find-AndroidPlatformJar $Sdk $Api
        $buildToolsReady = Test-Path -LiteralPath (Join-Path $buildTools 'aapt2.exe') -PathType Leaf
        if ($platformJar -and $buildToolsReady) { return $platformJar }
    }

    if ($lastFailure) { throw $lastFailure }
    if (-not $platformJar) { throw "Android SDK platform API $Api could not be prepared under $Sdk\platforms." }
    throw "Android build-tools $BuildToolsVersion could not be prepared under $Sdk\build-tools."
}

function Ensure-AndroidNdk([string]$Sdk, [string]$Version) {
    $ndk = Join-Path $Sdk "ndk/$Version"
    if (-not (Test-Path -LiteralPath $ndk -PathType Container)) {
        Write-Host "==> Installing pinned Android NDK $Version under $Sdk"
        Install-AndroidSdkPackages $Sdk @("ndk/$Version")
    }
    if (-not (Test-Path -LiteralPath $ndk -PathType Container)) { throw "Pinned Android NDK $Version could not be prepared under $Sdk\ndk." }
    $properties = Join-Path $ndk 'source.properties'
    if (Test-Path -LiteralPath $properties -PathType Leaf) {
        $text = Get-Content -LiteralPath $properties -Raw
        if ($text -match '(?m)^Pkg\.Revision\s*=\s*([^\r\n]+)') {
            $actualVersion = $Matches[1].Trim()
            if ($actualVersion -ne $Version) { throw "Android NDK directory $ndk reports revision $actualVersion; expected $Version." }
        }
    }
    return $ndk
}

if ($buildVault) {
    $stableRust = [string]$env:KASKOLD_STABLE_RUST
    $cargoNdkVersion = [string]$env:KASKOLD_CARGO_NDK_VERSION
    $androidNdkVersion = [string]$env:KASKOLD_ANDROID_NDK
    if (-not $stableRust) { throw 'KASKOLD_STABLE_RUST is missing from qa/config/toolchains.env.' }
    if (-not $cargoNdkVersion) { throw 'KASKOLD_CARGO_NDK_VERSION is missing from qa/config/toolchains.env.' }
    if (-not $androidNdkVersion) { throw 'KASKOLD_ANDROID_NDK is missing from qa/config/toolchains.env.' }
    Ensure-RustAndroidTargets $stableRust
    Ensure-CargoNdk $stableRust $cargoNdkVersion
    $ndkPath = Ensure-AndroidNdk $sdk $androidNdkVersion
}
$androidApiText = [string]$env:KASKOLD_ANDROID_API
$androidBuildTools = [string]$env:KASKOLD_ANDROID_BUILD_TOOLS
if ($androidApiText -notmatch '^\d+$') { throw 'KASKOLD_ANDROID_API is missing or invalid in qa/config/toolchains.env.' }
$androidApi = [int]$androidApiText
$androidPlatformJar = Ensure-AndroidSdkBuildPackages $sdk $androidApi $androidBuildTools

if (-not (Test-Path -LiteralPath $wrapperProperties -PathType Leaf)) { throw "Missing Gradle wrapper metadata: $wrapperProperties" }
$wrapper = @{}
foreach ($line in Get-Content -LiteralPath $wrapperProperties) {
    if ($line -match '^([^=]+)=(.*)$') { $wrapper[$Matches[1].Trim()] = ($Matches[2].Trim() -replace '\\:', ':') }
}
$url = [string]$wrapper['distributionUrl']
$sha = [string]$wrapper['distributionSha256Sum']
if ($url -notmatch '/gradle-([0-9]+(?:\.[0-9]+)*)-(?:bin|all)\.zip(?:[?#].*)?$') { throw 'Could not determine the pinned Gradle version from distributionUrl.' }
$version = $Matches[1]
if ($sha -notmatch '^[0-9A-Fa-f]{64}$') { throw 'distributionSha256Sum is missing or invalid in Gradle wrapper metadata.' }

function Gradle-Version([string]$Command) {
    if (-not $Command) { return $null }
    $probe = Invoke-KasKoldCapture -Command $Command -Arguments @('--version')
    if ($probe.ExitCode -ne 0) { return $null }
    if ($probe.Output -match '(?m)^Gradle\s+([^\s]+)') { return $Matches[1] }
    return $null
}

$gradle = $null
if ($env:GRADLE_BIN) {
    $command = Get-Command $env:GRADLE_BIN -ErrorAction SilentlyContinue
    if (-not $command -and (Test-Path -LiteralPath $env:GRADLE_BIN -PathType Leaf)) { $gradle = $env:GRADLE_BIN }
    elseif ($command) { $gradle = $command.Source }
    if (-not $gradle) { throw "GRADLE_BIN=$($env:GRADLE_BIN) was requested but is not executable or on PATH." }
    $foundVersion = Gradle-Version $gradle
    if ($foundVersion -ne $version) { throw "Pinned Gradle $version is required; GRADLE_BIN provides $foundVersion." }
} else {
    $command = Get-Command gradle.exe -ErrorAction SilentlyContinue
    if (-not $command) { $command = Get-Command gradle.bat -ErrorAction SilentlyContinue }
    if ($command -and (Gradle-Version $command.Source) -eq $version) { $gradle = $command.Source }
}

$gradleHome = if ($env:GRADLE_USER_HOME) { $env:GRADLE_USER_HOME } else { Join-Path $HOME '.gradle' }
$env:GRADLE_USER_HOME = $gradleHome
if (-not $gradle) {
    $distRoot = Join-Path $gradleHome 'kaskold/distributions'
    $zip = Join-Path $distRoot "gradle-$version-distribution.zip"
    $extracted = Join-Path $distRoot "gradle-$version"
    $gradle = Join-Path $extracted 'bin/gradle.bat'
    New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
    $archiveValid = (Test-Path -LiteralPath $zip -PathType Leaf) -and ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant() -eq $sha.ToLowerInvariant())
    if (-not $archiveValid) {
        Write-Host "==> Pinned Gradle $version is not installed; downloading and verifying it under $distRoot"
        $temporary = "$zip.tmp"
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
        try { Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $temporary }
        catch { Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue; throw "Unable to download pinned Gradle from $url`: $($_.Exception.Message)" }
        $actual = (Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $sha.ToLowerInvariant()) {
            Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
            throw "Gradle SHA-256 mismatch: expected $sha, got $actual"
        }
        Move-Item -LiteralPath $temporary -Destination $zip -Force
    }
    if (-not (Test-Path -LiteralPath $gradle -PathType Leaf)) {
        Remove-KasKoldPath $extracted
        Expand-Archive -LiteralPath $zip -DestinationPath $distRoot -Force
    }
}
if (-not (Test-Path -LiteralPath $gradle -PathType Leaf)) { throw "Pinned Gradle $version could not be prepared." }
$foundVersion = Gradle-Version $gradle
if ($foundVersion -ne $version) { throw "Pinned Gradle $version is required; prepared $foundVersion." }

$companionTasks = switch ($Mode) { 'debug' {@('assembleDebug')} 'release' {@('assembleRelease')} 'test' {@('testDebugUnitTest')} }
$vaultTasks = switch ($Mode) { 'debug' {@('assembleDebug')} 'release' {@('assembleRelease')} 'test' {@('assembleDebug','testDebugUnitTest')} }
$python = Get-KasKoldPython
$gradleEnvironment = @{ 'PYTHON' = $python }
Write-Host "==> KasKold Android product family - $Mode (API $androidApi; products=$products)"
Write-Host "==> Android SDK: $sdk"
if ($buildVault) {
    Write-Host "==> Android NDK: $ndkPath"
    Write-Host "==> cargo-ndk: $cargoNdkVersion"
}
Write-Host "==> Java: $($java.Path) (major $($java.Major))"
Write-Host "==> Python: $python"
Write-Host "==> Gradle: $gradle"
$productDirs = [Collections.Generic.List[string]]::new()
if ($buildCompanion) {
    Write-Host '==> Building KasKold Companion Android'
    Invoke-KasKoldCommand -Command $gradle -Arguments (@('--project-dir',$android,'--no-daemon') + $companionTasks) -WorkingDirectory $root -Environment $gradleEnvironment | Out-Null
    $productDirs.Add($android)
}
if ($buildVault) {
    Write-Host '==> Building KasKold Vault Android'
    Invoke-KasKoldCommand -Command $gradle -Arguments (@('--project-dir',$vault,'--no-daemon') + $vaultTasks) -WorkingDirectory $root -Environment $gradleEnvironment | Out-Null
    $productDirs.Add($vault)
}
Write-Host "KasKold Android product family - $Mode complete."

if ($Mode -in @('debug','release')) {
    foreach ($productDir in $productDirs) {
        $variantDir = Join-Path $productDir "app/build/outputs/apk/$Mode"
        $artifacts = @(Get-ChildItem -LiteralPath $variantDir -Filter '*.apk' -File -ErrorAction SilentlyContinue | Sort-Object FullName)
        if ($artifacts.Count -eq 0) { throw "Android $Mode build completed but no APK was found under $variantDir." }
        Write-Host $(if ($artifacts.Count -eq 1) { 'Built artifact:' } else { 'Built artifacts:' })
        foreach ($artifact in $artifacts) { Write-Host "  $($artifact.FullName)" }
    }
} elseif ($Mode -eq 'test') {
    foreach ($productDir in $productDirs) {
        $report = Join-Path $productDir 'app/build/reports/tests/testDebugUnitTest/index.html'
        if (Test-Path -LiteralPath $report -PathType Leaf) {
            Write-Host 'Test report:'
            Write-Host "  $report"
        }
    }
}
