# Native Windows QEMU setup helpers.
$script:QemuScriptDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../qemu'))
$script:RootDir = [IO.Path]::GetFullPath((Join-Path $script:QemuScriptDir '../../..'))
. (Join-Path $script:RootDir 'scripts/windows/lib/common.ps1')
Import-KasKoldToolchains $script:RootDir
$script:QemuStateDir = if ($env:KASKOLD_QEMU_HOME) { $env:KASKOLD_QEMU_HOME } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'KasKold/qemu' } else { Join-Path $HOME '.kaskold/qemu' }
$script:ManagedIdfPath = Join-Path $script:QemuStateDir "esp-idf-$($env:KASKOLD_ESP_IDF_VERSION)"

function Install-KasKoldRustupIfMissing {
    if ((Get-Command cargo -ErrorAction SilentlyContinue) -and (Get-Command rustup -ErrorAction SilentlyContinue)) { return }
    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) { throw 'Rustup is required. Install Rust for Windows from rustup.rs, then reopen PowerShell.' }
    Write-Host 'Rustup is missing; installing Rustup natively with winget.'
    & $winget.Source install --id Rustlang.Rustup --exact --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) { throw "winget failed to install Rustup (exit $LASTEXITCODE)" }
    $cargoBin = Join-Path $HOME '.cargo/bin'
    if ($env:PATH -notlike "*$cargoBin*") { $env:PATH = $cargoBin + [IO.Path]::PathSeparator + $env:PATH }
    Require-KasKoldCommand rustup 'Restart PowerShell if winget installed Rustup but PATH has not refreshed.' | Out-Null
    Require-KasKoldCommand cargo 'Restart PowerShell if winget installed Rustup but PATH has not refreshed.' | Out-Null
}

function Install-KasKoldEspRustToolchain {
    Require-KasKoldCommand cargo | Out-Null; Require-KasKoldCommand rustup | Out-Null
    $hostCargo = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run',$env:KASKOLD_STABLE_RUST,'cargo','--version')
    if ($hostCargo.ExitCode -ne 0) {
        Write-Host "Pinned host Rust $($env:KASKOLD_STABLE_RUST) is missing; installing it before ESP tooling."
        Invoke-KasKoldStreamingCommand -Command 'rustup' -Arguments @('toolchain','install',$env:KASKOLD_STABLE_RUST,'--profile','minimal')
    }
    $probe = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run','esp','rustc','--version')
    if ($probe.ExitCode -ne 0) {
        Write-Host "ESP Rust toolchain is missing; provisioning pinned ESP Rust $($env:KASKOLD_ESP_RUST)."
        $espup = Invoke-KasKoldCapture -Command 'espup' -Arguments @('--version')
        if ($espup.ExitCode -ne 0 -or $espup.Output -notlike "*$($env:KASKOLD_ESPUP_VERSION)*") {
            Write-Host "Installing pinned espup $($env:KASKOLD_ESPUP_VERSION)."
            Invoke-KasKoldStreamingCommand -Command 'rustup' -Arguments @('run',$env:KASKOLD_STABLE_RUST,'cargo','install','espup','--version',$env:KASKOLD_ESPUP_VERSION,'--locked','--force')
        }
        # espup installs a rustup toolchain named `esp` on Windows. Its POSIX export
        # file is intentionally not sourced; Cargo/rustup selection is native.
        Invoke-KasKoldStreamingCommand -Command 'espup' -Arguments @('install','--toolchain-version',$env:KASKOLD_ESP_RUST)
        $probe = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run','esp','rustc','--version')
        if ($probe.ExitCode -ne 0) { throw 'espup completed, but no usable ESP Rust toolchain named esp was found.' }
    }
    $cargoProbe = Invoke-KasKoldCapture -Command 'cargo' -Arguments @('--version') -WorkingDirectory (Join-Path $script:RootDir 'apps/kaskold-hardware')
    if ($cargoProbe.ExitCode -ne 0) { throw "Cargo cannot activate the ESP toolchain selected by apps/kaskold-hardware/rust-toolchain.toml.`n$($cargoProbe.Output)" }
    Write-Host "ESP Rust toolchain ready: $($probe.Output.Trim())"
}

function Install-KasKoldEspflash {
    $actual = Invoke-KasKoldCapture -Command 'espflash' -Arguments @('--version')
    if ($actual.ExitCode -ne 0 -or $actual.Output -notlike "*$($env:KASKOLD_ESPFLASH_VERSION)*") {
        Invoke-KasKoldStreamingCommand -Command 'cargo' -Arguments @('install','espflash','--version',$env:KASKOLD_ESPFLASH_VERSION,'--locked','--force')
    }
    Require-KasKoldCommand espflash | Out-Null
}

function Resolve-KasKoldIdfPath {
    if ($env:KASKOLD_IDF_PATH) {
        $candidate = [IO.Path]::GetFullPath($env:KASKOLD_IDF_PATH)
        if (-not (Test-Path -LiteralPath (Join-Path $candidate 'tools/idf_tools.py') -PathType Leaf)) { throw "invalid KASKOLD_IDF_PATH: $candidate" }
        return $candidate
    }
    New-Item -ItemType Directory -Force -Path $script:QemuStateDir | Out-Null
    if (-not (Test-Path -LiteralPath (Join-Path $script:ManagedIdfPath '.git') -PathType Container)) {
        Remove-KasKoldPath $script:ManagedIdfPath
        Require-KasKoldCommand git 'Git for Windows is required for QEMU setup.' | Out-Null
        Invoke-KasKoldStreamingCommand -Command 'git' -Arguments @('clone','--filter=blob:none','--depth','1','--branch',$env:KASKOLD_ESP_IDF_VERSION,'https://github.com/espressif/esp-idf.git',$script:ManagedIdfPath)
    }
    return $script:ManagedIdfPath
}

function Find-KasKoldMsys2Root {
    $candidates = @()
    if ($env:MSYS2_ROOT) { $candidates += $env:MSYS2_ROOT }
    $candidates += 'C:\msys64'
    if ($env:LOCALAPPDATA) {
        $candidates += (Join-Path $env:LOCALAPPDATA 'Programs/msys64')
        $candidates += (Join-Path $env:LOCALAPPDATA 'msys64')
    }
    foreach ($candidate in $candidates) {
        if (-not $candidate) { continue }
        $root = [IO.Path]::GetFullPath($candidate)
        if ((Test-Path -LiteralPath (Join-Path $root 'usr/bin/pacman.exe') -PathType Leaf) -and
            (Test-Path -LiteralPath (Join-Path $root 'mingw64/bin') -PathType Container)) {
            return $root
        }
    }
    return $null
}

function Ensure-KasKoldQemuWindowsRuntime {
    $root = Find-KasKoldMsys2Root
    if (-not $root) {
        $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
        if (-not $winget) {
            throw 'Espressif QEMU for Windows needs its MinGW runtime DLLs. MSYS2 was not found and winget is unavailable to install it.'
        }
        Write-Host '==> QEMU setup: installing MSYS2 runtime host for Espressif QEMU DLL dependencies'
        Invoke-KasKoldStreamingCommand -Command $winget.Source -Arguments @(
            'install','--id','MSYS2.MSYS2','--exact','--source','winget','--silent',
            '--accept-package-agreements','--accept-source-agreements','--disable-interactivity'
        )
        $root = Find-KasKoldMsys2Root
        if (-not $root) { throw 'winget reported MSYS2 installed, but its installation root could not be located.' }
    }

    $mingwBin = Join-Path $root 'mingw64/bin'
    $requiredDlls = @('libglib-2.0-0.dll','libpixman-1-0.dll','libgcrypt-20.dll','libslirp-0.dll','SDL2.dll')
    $missing = @($requiredDlls | Where-Object { -not (Test-Path -LiteralPath (Join-Path $mingwBin $_) -PathType Leaf) })
    if ($missing.Count -gt 0) {
        Write-Host ('==> QEMU setup: installing missing MinGW runtime libraries: ' + ($missing -join ', '))
        $packages = @(
            'mingw-w64-x86_64-glib2',
            'mingw-w64-x86_64-pixman',
            'mingw-w64-x86_64-libgcrypt',
            'mingw-w64-x86_64-libslirp',
            'mingw-w64-x86_64-SDL2'
        )
        $pacmanArgs = @('-S','--needed','--noconfirm') + $packages
        Invoke-KasKoldStreamingCommand -Command (Join-Path $root 'usr/bin/pacman.exe') -Arguments $pacmanArgs
        $missing = @($requiredDlls | Where-Object { -not (Test-Path -LiteralPath (Join-Path $mingwBin $_) -PathType Leaf) })
        if ($missing.Count -gt 0) {
            throw ('MSYS2 finished, but QEMU runtime DLLs are still missing: ' + ($missing -join ', '))
        }
    }

    if ($env:PATH -notlike "*$mingwBin*") {
        $env:PATH = $mingwBin + [IO.Path]::PathSeparator + $env:PATH
    }
    Write-Host "==> QEMU setup: MinGW runtime DLL path ready: $mingwBin"
    return $mingwBin
}

function Find-KasKoldQemuBinary {
    $toolsRoot = if ($env:IDF_TOOLS_PATH) { $env:IDF_TOOLS_PATH } else { Join-Path $HOME '.espressif' }
    $search = Join-Path $toolsRoot 'tools/qemu-xtensa'
    if (Test-Path -LiteralPath $search) {
        $candidate = Get-ChildItem -LiteralPath $search -Recurse -File -Filter 'qemu-system-xtensa.exe' -ErrorAction SilentlyContinue | Sort-Object FullName | Select-Object -Last 1
        if ($candidate) { return $candidate.FullName }
    }
    $cmd = Get-Command qemu-system-xtensa.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

function Install-KasKoldEspressifQemu {
    Write-Host '==> QEMU setup: resolving pinned Espressif QEMU'
    $python = Get-KasKoldPython
    $idf = Resolve-KasKoldIdfPath
    $env:IDF_TOOLS_PATH = if ($env:IDF_TOOLS_PATH) { $env:IDF_TOOLS_PATH } else { Join-Path $HOME '.espressif' }
    # The official win64 QEMU artifact is a MinGW build. Its version probe exits
    # with NTSTATUS 0xC0000135 (decimal 3221225781) when the dependent MinGW
    # runtime DLLs are not on PATH. Prepare that runtime before idf_tools.py
    # performs its post-extraction version check, otherwise idf_tools removes the
    # otherwise-correct QEMU installation as unusable.
    Ensure-KasKoldQemuWindowsRuntime | Out-Null
    Write-Host "==> QEMU setup: installing/verifying qemu-xtensa under $($env:IDF_TOOLS_PATH)"
    Invoke-KasKoldStreamingCommand -Command $python -Arguments @((Join-Path $idf 'tools/idf_tools.py'),'install','qemu-xtensa') -WorkingDirectory $script:RootDir
    $qemu = Find-KasKoldQemuBinary
    if (-not $qemu) { throw "Espressif QEMU was installed, but qemu-system-xtensa.exe could not be located under $($env:IDF_TOOLS_PATH)." }
    Write-Host "==> QEMU setup: probing $qemu"
    $versionProbe = Invoke-KasKoldCapture -Command $qemu -Arguments @('--version')
    if ($versionProbe.ExitCode -ne 0) { throw "Espressif QEMU exists but cannot run: $qemu`n$($versionProbe.Output)" }
    $machineProbe = Invoke-KasKoldCapture -Command $qemu -Arguments @('-machine','help')
    if ($machineProbe.ExitCode -ne 0 -or $machineProbe.Output -notmatch '(?m)^\s*esp32s3\s') {
        throw "Selected qemu-system-xtensa does not advertise the required esp32s3 machine: $qemu`n$($machineProbe.Output)"
    }
    $env:IDF_PATH = $idf
    $env:QEMU_SYSTEM_XTENSA = $qemu
    $qemuDir = Split-Path -Parent $qemu
    if ($env:PATH -notlike "*$qemuDir*") { $env:PATH = $qemuDir + [IO.Path]::PathSeparator + $env:PATH }
    Write-Host "Espressif QEMU ready: $qemu"
    $versionLine = @($versionProbe.Output -split "`r?`n")[0]
    Write-Host "QEMU version: $versionLine"
}

function Initialize-KasKoldQemuEnvironment {
    Get-KasKoldPython | Out-Null
    Require-KasKoldCommand git 'Install Git for Windows and ensure git.exe is on PATH.' | Out-Null
    Install-KasKoldRustupIfMissing
    Install-KasKoldEspRustToolchain
    Install-KasKoldEspflash
    Install-KasKoldEspressifQemu
    Write-Host "QEMU environment ready: $($env:QEMU_SYSTEM_XTENSA)"
}

function Invoke-KasKoldQemuHarness {
    param([switch]$KeepRunning)

    [Console]::Out.WriteLine('==> QEMU stage 1/3: initialize native Windows emulator environment')
    Initialize-KasKoldQemuEnvironment

    [Console]::Out.WriteLine('==> QEMU stage 2/3: build ESP32-S3 test image')
    & (Join-Path $script:RootDir 'tools/firmware/qemu/build.ps1')
    if (-not $?) { throw 'QEMU firmware build script failed.' }

    $python = Get-KasKoldPython
    $arguments = @(
        (Join-Path $script:RootDir 'qa/checks/firmware/qemu/run.py'),
        '--qemu', $env:QEMU_SYSTEM_XTENSA,
        '--image', (Join-Path $script:RootDir 'target/qemu/kaskold-qemu-flash.bin')
    )
    if ($KeepRunning) { $arguments += '--keep-running' }

    [Console]::Out.WriteLine('==> QEMU stage 3/3: launch host harness and stream guest UART')
    $display = @($python) + $arguments | ForEach-Object {
        if ($_ -match '[\s"]') { '"' + ($_ -replace '"','\"') + '"' } else { $_ }
    }
    [Console]::Out.WriteLine('  + ' + ($display -join ' '))

    Push-Location -LiteralPath $script:RootDir
    $savedPreference = $ErrorActionPreference
    try {
        # Windows PowerShell 5.1 can promote native stderr into a terminating
        # NativeCommandError when the repository-wide preference is Stop. The
        # Python harness owns the native exit status and its own diagnostics.
        $ErrorActionPreference = 'Continue'
        & $python @arguments
        $status = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    } finally {
        $ErrorActionPreference = $savedPreference
        Pop-Location
    }
    if ($status -ne 0) {
        throw "QEMU host harness exited with status $status."
    }
}

