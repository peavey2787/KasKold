[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$SkipAndroidEmulator
)
# KasKold native Windows developer bootstrap.
# After this completes, a new developer should be able to run qa/windows/run-all.ps1.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Use this installer on native Windows only.' }

$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
Import-KasKoldToolchains $root

$localBin = Join-Path $HOME '.kaskold/bin'
$toolRoot = Join-Path $HOME '.kaskold/tools'
$androidSdk = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'Android/Sdk' } else { Join-Path $HOME 'AppData/Local/Android/Sdk' }
New-Item -ItemType Directory -Force -Path $localBin,$toolRoot,$androidSdk | Out-Null

function Refresh-KasKoldPath {
    $machine = [Environment]::GetEnvironmentVariable('Path','Machine')
    $user = [Environment]::GetEnvironmentVariable('Path','User')
    $extras = @(
        $localBin,
        (Join-Path $HOME '.cargo/bin'),
        (Join-Path $androidSdk 'platform-tools'),
        (Join-Path $androidSdk 'cmdline-tools/latest/bin')
    )
    $env:PATH = (($extras + @($machine,$user) | Where-Object { $_ }) -join [IO.Path]::PathSeparator)
}

function Add-KasKoldUserPath([string]$Path) {
    $current = [Environment]::GetEnvironmentVariable('Path','User')
    $parts = @($current -split [regex]::Escape([IO.Path]::PathSeparator) | Where-Object { $_ })
    if ($parts -notcontains $Path) {
        [Environment]::SetEnvironmentVariable('Path', (($parts + $Path) -join [IO.Path]::PathSeparator), 'User')
    }
}

function Test-KasKoldCommand([string]$Name) { return [bool](Get-Command $Name -ErrorAction SilentlyContinue) }

function Get-KasKoldChrome {
    foreach ($name in @('google-chrome.exe','chrome.exe','chromium.exe')) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    $candidates = New-Object System.Collections.Generic.List[string]
    if ($env:ProgramFiles) { $candidates.Add((Join-Path $env:ProgramFiles 'Google/Chrome/Application/chrome.exe')) }
    if (${env:ProgramFiles(x86)}) { $candidates.Add((Join-Path ${env:ProgramFiles(x86)} 'Google/Chrome/Application/chrome.exe')) }
    if ($env:LOCALAPPDATA) { $candidates.Add((Join-Path $env:LOCALAPPDATA 'Google/Chrome/Application/chrome.exe')) }
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) { return $candidate }
    }
    return $null
}

function Invoke-WingetInstall([string]$Id, [string[]]$Extra = @()) {
    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) { throw 'Windows Package Manager (winget) is required. Install App Installer from Microsoft, then rerun install.ps1.' }
    Write-Host "Installing/repairing $Id..."
    & $winget.Source install --id $Id --exact --accept-package-agreements --accept-source-agreements @Extra
    if ($LASTEXITCODE -ne 0) { throw "winget failed for $Id with exit code $LASTEXITCODE" }
    Refresh-KasKoldPath
}

function Ensure-WingetCommand([string]$Command, [string]$Id) {
    if (-not (Test-KasKoldCommand $Command)) {
        Invoke-WingetInstall $Id
    }
    Refresh-KasKoldPath
    if (-not (Test-KasKoldCommand $Command)) {
        # GnuWin32 Make is not consistently added to PATH by older manifests.
        if ($Command -eq 'make') {
            $makeCandidates = New-Object System.Collections.Generic.List[string]
            if (${env:ProgramFiles(x86)}) { $makeCandidates.Add((Join-Path ${env:ProgramFiles(x86)} 'GnuWin32/bin')) }
            if ($env:ProgramFiles) { $makeCandidates.Add((Join-Path $env:ProgramFiles 'GnuWin32/bin')) }
            foreach ($candidate in $makeCandidates) {
                if ($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'make.exe'))) {
                    Add-KasKoldUserPath $candidate; Refresh-KasKoldPath; break
                }
            }
        }
    }
    if (-not (Test-KasKoldCommand $Command)) { throw "Required command is still missing after installation: $Command ($Id)" }
}

function Ensure-Chrome {
    if (-not (Get-KasKoldChrome)) { Invoke-WingetInstall 'Google.Chrome' }
    $chrome = Get-KasKoldChrome
    if (-not $chrome) { throw 'Google Chrome was installed but chrome.exe could not be located.' }
    $shim = Join-Path $localBin 'google-chrome.cmd'
    [IO.File]::WriteAllText($shim, "@echo off`r`n`"$chrome`" %*`r`n", [Text.UTF8Encoding]::new($false))
    Add-KasKoldUserPath $localBin; Refresh-KasKoldPath
}

function Ensure-MsvcFuzzToolchain {
    if (-not ${env:ProgramFiles(x86)}) { throw 'ProgramFiles(x86) is unavailable; 64-bit Windows is required.' }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $ready = $false
    if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
        $installation = & $vswhere -latest -products Microsoft.VisualStudio.Product.BuildTools `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 Microsoft.VisualStudio.Component.VC.ASAN `
            -property installationPath 2>$null | Select-Object -First 1
        $ready = [bool]$installation
    }
    if (-not $ready) {
        Invoke-WingetInstall 'Microsoft.VisualStudio.2022.BuildTools' @(
            '--force', '--override', '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --add Microsoft.VisualStudio.Component.VC.ASAN'
        )
        if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) { throw 'Visual Studio Build Tools installed, but vswhere.exe is unavailable.' }
        $installation = & $vswhere -latest -products Microsoft.VisualStudio.Product.BuildTools `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 Microsoft.VisualStudio.Component.VC.ASAN `
            -property installationPath 2>$null | Select-Object -First 1
        if (-not $installation) { throw 'Visual Studio C++ tools with AddressSanitizer are required for native Windows fuzzing.' }
    }
}

function Ensure-Rust {
    if (-not (Test-KasKoldCommand rustup) -or -not (Test-KasKoldCommand cargo)) {
        Invoke-WingetInstall 'Rustlang.Rustup'
        Refresh-KasKoldPath
    }
    Require-KasKoldCommand rustup 'Restart PowerShell if rustup was just installed and is not visible.' | Out-Null
    Require-KasKoldCommand cargo 'Restart PowerShell if cargo was just installed and is not visible.' | Out-Null
    Invoke-KasKoldCommand -Command 'rustup' -Arguments @('toolchain','install',$env:KASKOLD_STABLE_RUST,'--profile','minimal') | Out-Null
    Invoke-KasKoldCommand -Command 'rustup' -Arguments @('toolchain','install',$env:KASKOLD_BRANCH_RUST,'--profile','minimal','--component','llvm-tools-preview') | Out-Null
    $targets = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('target','list','--toolchain',$env:KASKOLD_STABLE_RUST,'--installed')
    foreach ($target in @('wasm32-unknown-unknown','aarch64-linux-android','x86_64-linux-android')) {
        if (($targets.Output -split "`r?`n") -notcontains $target) {
            Invoke-KasKoldCommand -Command 'rustup' -Arguments @('target','add',$target,'--toolchain',$env:KASKOLD_STABLE_RUST) | Out-Null
        }
    }
}

function Ensure-CargoTool([string]$Subcommand, [string]$Package, [string]$Version, [string]$Toolchain = '') {
    if (-not $Toolchain) { $Toolchain = $env:KASKOLD_STABLE_RUST }
    $probe = Invoke-KasKoldCapture -Command 'cargo' -Arguments @("+$Toolchain",$Subcommand,'--version')
    if ($probe.ExitCode -ne 0 -or $probe.Output -notlike "*$Version*") {
        Invoke-KasKoldCommand -Command 'cargo' -Arguments @("+$($env:KASKOLD_STABLE_RUST)",'install',$Package,'--version',$Version,'--locked','--force') | Out-Null
    }
}

function Ensure-RustTools {
    Ensure-CargoTool 'mutants' 'cargo-mutants' $env:KASKOLD_CARGO_MUTANTS_VERSION
    Ensure-CargoTool 'fuzz' 'cargo-fuzz' $env:KASKOLD_CARGO_FUZZ_VERSION $env:KASKOLD_BRANCH_RUST
    Ensure-CargoTool 'ndk' 'cargo-ndk' $env:KASKOLD_CARGO_NDK_VERSION
    & (Join-Path $root 'scripts/windows/quality/branch-coverage-setup.ps1')
    if ($LASTEXITCODE) { throw "branch coverage setup failed with exit code $LASTEXITCODE" }

    $espup = Invoke-KasKoldCapture -Command 'espup' -Arguments @('--version')
    if ($espup.ExitCode -ne 0 -or $espup.Output -notlike "*$($env:KASKOLD_ESPUP_VERSION)*") {
        Invoke-KasKoldCommand -Command 'cargo' -Arguments @("+$($env:KASKOLD_STABLE_RUST)",'install','espup','--version',$env:KASKOLD_ESPUP_VERSION,'--locked','--force') | Out-Null
    }
    $esp = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run','esp','rustc','--version')
    if ($esp.ExitCode -ne 0) {
        Invoke-KasKoldCommand -Command 'espup' -Arguments @('install','--toolchain-version',$env:KASKOLD_ESP_RUST) | Out-Null
    }
    $espflash = Invoke-KasKoldCapture -Command 'espflash' -Arguments @('--version')
    if ($espflash.ExitCode -ne 0 -or $espflash.Output -notlike "*$($env:KASKOLD_ESPFLASH_VERSION)*") {
        Invoke-KasKoldCommand -Command 'cargo' -Arguments @("+$($env:KASKOLD_STABLE_RUST)",'install','espflash','--version',$env:KASKOLD_ESPFLASH_VERSION,'--locked','--force') | Out-Null
    }

    $cacheBase = if ($env:KASKOLD_TOOL_CACHE_DIR) { $env:KASKOLD_TOOL_CACHE_DIR } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'KasKold/tools' } else { Join-Path $HOME '.cache/kaskold/tools' }
    $wasmRoot = Join-Path $cacheBase "wasm-bindgen-cli-$($env:KASKOLD_WASM_BINDGEN_CLI_VERSION)"
    $wasm = Join-Path $wasmRoot 'bin/wasm-bindgen.exe'
    $wasmProbe = if (Test-Path -LiteralPath $wasm -PathType Leaf) { Invoke-KasKoldCapture -Command $wasm -Arguments @('--version') } else { [pscustomobject]@{ExitCode=1;Output=''} }
    if ($wasmProbe.ExitCode -ne 0 -or $wasmProbe.Output.Trim() -ne "wasm-bindgen $($env:KASKOLD_WASM_BINDGEN_CLI_VERSION)") {
        Remove-KasKoldPath $wasmRoot
        Invoke-KasKoldCommand -Command 'cargo' -Arguments @("+$($env:KASKOLD_STABLE_RUST)",'install','wasm-bindgen-cli','--version',$env:KASKOLD_WASM_BINDGEN_CLI_VERSION,'--locked','--root',$wasmRoot) | Out-Null
    }
}

function Ensure-Jdk25 {
    $required = [int]$env:KASKOLD_ANDROID_JDK
    $target = Join-Path $toolRoot "jdk-$required"
    $targetJava = Join-Path $target 'bin/java.exe'
    $managedMajor = 0
    if (Test-Path -LiteralPath $targetJava -PathType Leaf) {
        $probe = Invoke-KasKoldCapture -Command $targetJava -Arguments @('-version')
        if ($probe.ExitCode -eq 0 -and $probe.Output -match 'version "(?<major>\d+)') { $managedMajor = [int]$Matches.major }
    }
    if ($managedMajor -ne $required) {
        $arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'ARM64' {'aarch64'} default {'x64'} }
        $metaUri = "https://api.adoptium.net/v3/assets/latest/$required/hotspot?architecture=$arch&image_type=jdk&os=windows&vendor=eclipse"
        $asset = (Invoke-RestMethod -Uri $metaUri -UseBasicParsing)[0].binary.package
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-jdk-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            $archive = Join-Path $tmp 'jdk.zip'
            Invoke-WebRequest -Uri $asset.link -OutFile $archive -UseBasicParsing
            $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
            if ($actual -ne ([string]$asset.checksum).ToLowerInvariant()) { throw 'JDK download checksum mismatch.' }
            Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $tmp 'unpack') -Force
            $source = Get-ChildItem -LiteralPath (Join-Path $tmp 'unpack') -Directory | Select-Object -First 1
            if (-not $source) { throw 'JDK archive did not contain a top-level directory.' }
            Remove-KasKoldPath $target; Move-Item -LiteralPath $source.FullName -Destination $target
        } finally { Remove-KasKoldPath $tmp }
    }
    $env:JAVA_HOME = $target
    [Environment]::SetEnvironmentVariable('JAVA_HOME',$target,'User')
    Add-KasKoldUserPath (Join-Path $target 'bin'); Refresh-KasKoldPath
}

function Ensure-Gradle {
    $version = $env:KASKOLD_GRADLE_VERSION
    $target = Join-Path $toolRoot "gradle-$version"
    $gradleProbe = if (Test-KasKoldCommand gradle) { Invoke-KasKoldCapture -Command 'gradle' -Arguments @('--version') } else { [pscustomobject]@{ExitCode=1;Output=''} }
    if ($gradleProbe.ExitCode -ne 0 -or $gradleProbe.Output -notmatch [regex]::Escape("Gradle $version")) {
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-gradle-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            $archive = Join-Path $tmp 'gradle.zip'
            $base = "https://services.gradle.org/distributions/gradle-$version-bin.zip"
            Invoke-WebRequest -Uri $base -OutFile $archive -UseBasicParsing
            $expected = ((Invoke-WebRequest -Uri "$base.sha256" -UseBasicParsing).Content).Trim().ToLowerInvariant()
            $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
            if ($actual -ne $expected) { throw 'Gradle download checksum mismatch.' }
            Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $tmp 'unpack') -Force
            Remove-KasKoldPath $target
            Move-Item -LiteralPath (Join-Path $tmp "unpack/gradle-$version") -Destination $target
        } finally { Remove-KasKoldPath $tmp }
    }
    Add-KasKoldUserPath (Join-Path $target 'bin'); Refresh-KasKoldPath
}

function Ensure-Kotlin {
    $version = $env:KASKOLD_KOTLIN_CLI_VERSION
    $target = Join-Path $toolRoot "kotlin-$version"
    $kotlinProbe = if (Test-KasKoldCommand kotlinc) { Invoke-KasKoldCapture -Command 'kotlinc' -Arguments @('-version') } else { [pscustomobject]@{ExitCode=1;Output=''} }
    if ($kotlinProbe.ExitCode -ne 0 -or $kotlinProbe.Output -notmatch [regex]::Escape($version)) {
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-kotlin-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            $archive = Join-Path $tmp 'kotlin.zip'
            Invoke-WebRequest -Uri "https://github.com/JetBrains/kotlin/releases/download/v$version/kotlin-compiler-$version.zip" -OutFile $archive -UseBasicParsing
            Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $tmp 'unpack') -Force
            Remove-KasKoldPath $target
            Move-Item -LiteralPath (Join-Path $tmp 'unpack/kotlinc') -Destination $target
        } finally { Remove-KasKoldPath $tmp }
    }
    Add-KasKoldUserPath (Join-Path $target 'bin'); Refresh-KasKoldPath
}

function Ensure-AndroidSdk {
    $tools = Join-Path $androidSdk 'cmdline-tools/latest'
    $sdkmanager = Join-Path $tools 'bin/sdkmanager.bat'
    if (-not (Test-Path -LiteralPath $sdkmanager -PathType Leaf)) {
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('companion-android-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            $archive = Join-Path $tmp 'cmdline-tools.zip'
            Invoke-WebRequest -Uri "https://dl.google.com/android/repository/commandlinetools-win-$($env:KASKOLD_ANDROID_CMDLINE_TOOLS)_latest.zip" -OutFile $archive -UseBasicParsing
            $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
            if ($actual -ne ([string]$env:KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256).ToLowerInvariant()) { throw 'Android command-line tools checksum mismatch.' }
            Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $tmp 'unpack') -Force
            Remove-KasKoldPath $tools
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $tools) | Out-Null
            Move-Item -LiteralPath (Join-Path $tmp 'unpack/cmdline-tools') -Destination $tools
        } finally { Remove-KasKoldPath $tmp }
    }
    $env:ANDROID_SDK_ROOT = $androidSdk; $env:ANDROID_HOME = $androidSdk
    [Environment]::SetEnvironmentVariable('ANDROID_SDK_ROOT',$androidSdk,'User')
    [Environment]::SetEnvironmentVariable('ANDROID_HOME',$androidSdk,'User')
    Add-KasKoldUserPath (Join-Path $androidSdk 'platform-tools')
    Add-KasKoldUserPath (Join-Path $androidSdk 'cmdline-tools/latest/bin')
    Refresh-KasKoldPath
    1..100 | ForEach-Object { 'y' } | & $sdkmanager --sdk_root=$androidSdk --licenses | Out-Null
    & $sdkmanager --sdk_root=$androidSdk 'platform-tools' "platforms;android-$($env:KASKOLD_ANDROID_API)" "build-tools;$($env:KASKOLD_ANDROID_BUILD_TOOLS)" "ndk;$($env:KASKOLD_ANDROID_NDK)" 'extras;google;usb_driver'
    if ($LASTEXITCODE -ne 0) { throw 'Android SDK package installation failed.' }
    if (-not $SkipAndroidEmulator) {
        $imageArch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64-v8a' } else { 'x86_64' }
        & $sdkmanager --sdk_root=$androidSdk 'emulator' "system-images;android-$($env:KASKOLD_ANDROID_API);google_apis;$imageArch"
        if ($LASTEXITCODE -ne 0) { throw 'Android emulator/system image installation failed.' }
    }
}

function Test-Environment {
    $missing = New-Object System.Collections.Generic.List[string]
    Write-Host 'KasKold native Windows environment check:'
    foreach ($name in @('make','git','node','npm','rustup','cargo','java','kotlinc','gradle','espup','espflash')) {
        if (Test-KasKoldCommand $name) { Write-Host "  OK   $name" } else { Write-Host "  MISS $name"; $missing.Add($name) }
    }
    try { $python = Get-KasKoldPython; Write-Host "  OK   python ($python)" } catch { Write-Host '  MISS python'; $missing.Add('python') }
    if (Get-KasKoldChrome) { Write-Host '  OK   Chrome' } else { Write-Host '  MISS Chrome'; $missing.Add('Chrome') }

    $managedJava = Join-Path $toolRoot "jdk-$($env:KASKOLD_ANDROID_JDK)/bin/java.exe"
    if (Test-Path -LiteralPath $managedJava -PathType Leaf) {
        $javaProbe = Invoke-KasKoldCapture -Command $managedJava -Arguments @('-version')
        if ($javaProbe.ExitCode -eq 0 -and $javaProbe.Output -match 'version "(?<major>\d+)' -and [int]$Matches.major -eq [int]$env:KASKOLD_ANDROID_JDK) {
            Write-Host "  OK   JDK $($env:KASKOLD_ANDROID_JDK)"
        } else { Write-Host "  MISS JDK $($env:KASKOLD_ANDROID_JDK)"; $missing.Add('JDK') }
    } else { Write-Host "  MISS JDK $($env:KASKOLD_ANDROID_JDK)"; $missing.Add('JDK') }
    if (Test-KasKoldCommand gradle) {
        $gradleProbe = Invoke-KasKoldCapture -Command 'gradle' -Arguments @('--version')
        if ($gradleProbe.ExitCode -eq 0 -and $gradleProbe.Output -match [regex]::Escape("Gradle $($env:KASKOLD_GRADLE_VERSION)")) { Write-Host "  OK   Gradle $($env:KASKOLD_GRADLE_VERSION)" }
        else { Write-Host "  MISS Gradle $($env:KASKOLD_GRADLE_VERSION)"; $missing.Add('Gradle version') }
    }
    if (Test-KasKoldCommand kotlinc) {
        $kotlinProbe = Invoke-KasKoldCapture -Command 'kotlinc' -Arguments @('-version')
        if ($kotlinProbe.ExitCode -eq 0 -and $kotlinProbe.Output -match [regex]::Escape($env:KASKOLD_KOTLIN_CLI_VERSION)) { Write-Host "  OK   Kotlin $($env:KASKOLD_KOTLIN_CLI_VERSION)" }
        else { Write-Host "  MISS Kotlin $($env:KASKOLD_KOTLIN_CLI_VERSION)"; $missing.Add('Kotlin version') }
    }

    $androidJar = Join-Path $androidSdk "platforms/android-$($env:KASKOLD_ANDROID_API)/android.jar"
    if (Test-Path -LiteralPath $androidJar -PathType Leaf) { Write-Host "  OK   Android SDK API $($env:KASKOLD_ANDROID_API)" } else { Write-Host "  MISS Android SDK API $($env:KASKOLD_ANDROID_API)"; $missing.Add('Android SDK') }
    $buildTools = Join-Path $androidSdk "build-tools/$($env:KASKOLD_ANDROID_BUILD_TOOLS)"
    if (Test-Path -LiteralPath $buildTools -PathType Container) { Write-Host "  OK   Android build-tools $($env:KASKOLD_ANDROID_BUILD_TOOLS)" } else { Write-Host "  MISS Android build-tools $($env:KASKOLD_ANDROID_BUILD_TOOLS)"; $missing.Add('Android build-tools') }
    $ndk = Join-Path $androidSdk "ndk/$($env:KASKOLD_ANDROID_NDK)"
    if (Test-Path -LiteralPath $ndk -PathType Container) { Write-Host "  OK   Android NDK $($env:KASKOLD_ANDROID_NDK)" } else { Write-Host "  MISS Android NDK $($env:KASKOLD_ANDROID_NDK)"; $missing.Add('Android NDK') }

    if (Test-KasKoldCommand rustup) {
        foreach ($tc in @($env:KASKOLD_STABLE_RUST,$env:KASKOLD_BRANCH_RUST,'esp')) {
            $probe = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run',$tc,'rustc','--version')
            if ($probe.ExitCode -eq 0) { Write-Host "  OK   Rust $tc" } else { Write-Host "  MISS Rust $tc"; $missing.Add("Rust $tc") }
        }
    }
    if (Test-KasKoldCommand cargo) {
        $cargoTools = @(
            [pscustomobject]@{ Toolchain=$env:KASKOLD_STABLE_RUST; Subcommand='mutants'; Version=$env:KASKOLD_CARGO_MUTANTS_VERSION; Name='cargo-mutants' },
            [pscustomobject]@{ Toolchain=$env:KASKOLD_BRANCH_RUST; Subcommand='fuzz'; Version=$env:KASKOLD_CARGO_FUZZ_VERSION; Name='cargo-fuzz' },
            [pscustomobject]@{ Toolchain=$env:KASKOLD_BRANCH_RUST; Subcommand='llvm-cov'; Version=$env:KASKOLD_CARGO_LLVM_COV_VERSION; Name='cargo-llvm-cov' },
            [pscustomobject]@{ Toolchain=$env:KASKOLD_BRANCH_RUST; Subcommand='crap'; Version=$env:KASKOLD_CARGO_CRAP_VERSION; Name='cargo-crap' }
        )
        foreach ($tool in $cargoTools) {
            $probe = Invoke-KasKoldCapture -Command 'cargo' -Arguments @("+$($tool.Toolchain)",$tool.Subcommand,'--version')
            if ($probe.ExitCode -eq 0 -and $probe.Output -like "*$($tool.Version)*") { Write-Host "  OK   $($tool.Name) $($tool.Version)" }
            else { Write-Host "  MISS $($tool.Name) $($tool.Version)"; $missing.Add($tool.Name) }
        }
    }

    $qemuRoot = Join-Path $HOME '.espressif/tools/qemu-xtensa'
    $qemu = if (Test-Path -LiteralPath $qemuRoot) { Get-ChildItem -LiteralPath $qemuRoot -Recurse -File -Filter 'qemu-system-xtensa.exe' -ErrorAction SilentlyContinue | Select-Object -First 1 } else { $null }
    if ($qemu -or (Get-Command qemu-system-xtensa.exe -ErrorAction SilentlyContinue)) { Write-Host '  OK   Espressif QEMU' }
    else { Write-Host '  MISS Espressif QEMU'; $missing.Add('Espressif QEMU') }

    if ($missing.Count) { throw ('Missing native Windows prerequisites: ' + ($missing -join ', ')) }
    Write-Host 'PASS: native Windows prerequisites are installed.'
}

Refresh-KasKoldPath
if ($CheckOnly) { Test-Environment; exit 0 }

Write-Host '==> Installing native Windows host tools'
Ensure-WingetCommand 'git' 'Git.Git'
try { Get-KasKoldPython | Out-Null } catch { Invoke-WingetInstall 'Python.Python.3.12' }
Refresh-KasKoldPath; Get-KasKoldPython | Out-Null
Ensure-WingetCommand 'make' 'GnuWin32.Make'
Ensure-WingetCommand 'node' 'OpenJS.NodeJS.LTS'
if (-not (Test-KasKoldCommand npm)) { throw 'npm is missing after Node.js installation.' }
Ensure-Chrome
Ensure-MsvcFuzzToolchain

Write-Host '==> Installing pinned Rust and QA toolchains'
Ensure-Rust
Ensure-RustTools
Write-Host "==> Installing JDK $($env:KASKOLD_ANDROID_JDK), Kotlin $($env:KASKOLD_KOTLIN_CLI_VERSION), and Gradle $($env:KASKOLD_GRADLE_VERSION)"
Ensure-Jdk25
Ensure-Kotlin
Ensure-Gradle
Write-Host "==> Installing Android SDK API $($env:KASKOLD_ANDROID_API)"
Ensure-AndroidSdk
Write-Host '==> Installing Espressif QEMU'
& (Join-Path $root 'scripts/windows/qemu/setup.ps1')
if ($LASTEXITCODE) { throw "Espressif QEMU setup failed with exit code $LASTEXITCODE" }
Write-Host '==> Priming pinned TypeScript compiler'
$python = Get-KasKoldPython
Invoke-KasKoldCommand -Command $python -Arguments @((Join-Path $root 'qa/checks/web/typescript_toolchain.py')) -WorkingDirectory $root | Out-Null

Add-KasKoldUserPath $localBin
Refresh-KasKoldPath
Test-Environment
Write-Host "`nPASS: KasKold native Windows developer environment is ready."
Write-Host "Next: cd `"$root`"; .\qa\windows\run-all.ps1"
