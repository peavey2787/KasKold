# Transactional host Cargo.lock reconciliation under the repository-pinned stable Cargo.
. (Join-Path $PSScriptRoot 'common.ps1')

function Get-KasKoldLockSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-KasKoldLockPackageCount {
    param([Parameter(Mandatory = $true)][string]$Path)
    # Cargo.lock writes every package as an array-of-tables entry. Counting
    # those headers avoids imposing the QA Python parser requirement on
    # standalone Rust commands such as `make sdk`.
    return @(Select-String -LiteralPath $Path -Pattern '^\s*\[\[package\]\]\s*$').Count
}

function Invoke-KasKoldHostCargoMetadata {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Manifest,
        [string[]]$ExtraArguments = @(),
        [switch]$Capture
    )
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
    $environment = @{
        'RUSTUP_TOOLCHAIN' = $env:KASKOLD_STABLE_RUST
        'CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS' = 'fallback'
        'PATH' = (Join-Path $cargoHome 'bin') + [IO.Path]::PathSeparator + $env:PATH
    }
    foreach ($name in @('RUSTC','RUSTDOC','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS')) {
        $environment[$name] = $null
    }
    $arguments = @('run', $env:KASKOLD_STABLE_RUST, 'cargo', 'metadata', '--manifest-path', (Join-Path $Root $Manifest), '--format-version', '1') + $ExtraArguments
    if ($Capture) { return Invoke-KasKoldCapture -Command 'rustup' -Arguments $arguments -WorkingDirectory $Root -Environment $environment }
    Invoke-KasKoldCommand -Command 'rustup' -Arguments $arguments -WorkingDirectory $Root -Environment $environment | Out-Null
}

function Repair-KasKoldOneHostLock {
    param([string]$Root,[string]$Label,[string]$Manifest,[string]$Lock)
    $lockPath = Join-Path $Root $Lock
    $verify = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--locked') -Capture
    if ($verify.ExitCode -eq 0) { return }
    if (-not (Test-Path -LiteralPath $lockPath -PathType Leaf)) { throw "Expected workspace lockfile is missing: $Lock" }
    $backup = [IO.Path]::GetTempFileName()
    Copy-Item -LiteralPath $lockPath -Destination $backup -Force
    try {
        $oldHash = Get-KasKoldLockSha256 $lockPath
        $oldCount = Get-KasKoldLockPackageCount $lockPath
        Write-Host "$Label Cargo.lock is stale under pinned Cargo $($env:KASKOLD_STABLE_RUST); reconciling transactionally."
        Write-Host "  Existing: sha256=$oldHash packages=$oldCount"
        $offline = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--offline') -Capture
        if ($offline.ExitCode -ne 0) {
            Copy-Item -LiteralPath $backup -Destination $lockPath -Force
            Write-Host '  Offline reconciliation was insufficient; retrying with registry access.'
            $online = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -Capture
            if ($online.ExitCode -ne 0) {
                Copy-Item -LiteralPath $backup -Destination $lockPath -Force
                throw "Cargo could not reconcile $Lock.`n$($online.Output)"
            }
        }
        $final = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--locked') -Capture
        if ($final.ExitCode -ne 0) {
            Copy-Item -LiteralPath $backup -Destination $lockPath -Force
            throw "Reconciled $Lock still fails Cargo --locked verification.`n$($final.Output)"
        }
        Write-Host "  Reconciled: sha256=$(Get-KasKoldLockSha256 $lockPath) packages=$(Get-KasKoldLockPackageCount $lockPath)"
    } finally { Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue }
}

function Test-KasKoldMetadataRustCompatibility {
    param([Parameter(Mandatory = $true)][string]$Json,[Parameter(Mandatory = $true)][string]$MaxRust)
    # Windows PowerShell 5.1 ConvertFrom-Json rejects valid JSON objects whose
    # keys differ only by case (for example Cargo package metadata containing
    # both "Default" and "default"). Parse Cargo's case-sensitive JSON with
    # Python instead and inspect only package rust_version fields.
    $python = Get-KasKoldPython
    $root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..'))
    $checker = Join-Path $root 'scripts/common/lib/cargo_metadata_compat.py'
    if (-not (Test-Path -LiteralPath $checker -PathType Leaf)) { throw "Missing Cargo metadata compatibility checker: $checker" }
    $metadataPath = [IO.Path]::GetTempFileName()
    try {
        Write-KasKoldUtf8NoBom -Path $metadataPath -Text $Json
        $result = Invoke-KasKoldCapture -Command $python -Arguments @($checker,'--metadata',$metadataPath,'--max-rust',$MaxRust) -WorkingDirectory $root
        if ($result.ExitCode -eq 0) { return $true }
        if ($result.ExitCode -eq 1) {
            @($result.Output -split '\r?\n') | Where-Object { $_ } | ForEach-Object { Write-Warning $_ }
            return $false
        }
        throw "Cargo metadata compatibility check failed with exit code $($result.ExitCode).`n$($result.Output)"
    } finally {
        Remove-Item -LiteralPath $metadataPath -Force -ErrorAction SilentlyContinue
    }
}

function Repair-KasKoldWebMsrvLock {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$Manifest,
        [Parameter(Mandatory = $true)][string]$Lock
    )
    $lockPath = Join-Path $Root $Lock
    $metadata = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--filter-platform','wasm32-unknown-unknown','--locked') -Capture
    if ($metadata.ExitCode -eq 0 -and (Test-KasKoldMetadataRustCompatibility -Json $metadata.Output -MaxRust $env:KASKOLD_REPRO_HOST_RUST)) { return }
    $backup = [IO.Path]::GetTempFileName()
    Copy-Item -LiteralPath $lockPath -Destination $backup -Force
    try {
        Write-Host "$Label Cargo.lock is not compatible with reproducible Rust $($env:KASKOLD_REPRO_HOST_RUST); resolving an MSRV-compatible lock transactionally."
        Write-Host "  Existing: sha256=$(Get-KasKoldLockSha256 $lockPath) packages=$(Get-KasKoldLockPackageCount $lockPath)"
        Remove-Item -LiteralPath $lockPath -Force
        $offline = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--offline') -Capture
        if ($offline.ExitCode -ne 0) {
            Remove-Item -LiteralPath $lockPath -Force -ErrorAction SilentlyContinue
            Write-Host '  Offline MSRV reconciliation was insufficient; retrying with registry access.'
            $online = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -Capture
            if ($online.ExitCode -ne 0) { Copy-Item $backup $lockPath -Force; throw "Cargo could not resolve an MSRV-compatible $Label lock.`n$($online.Output)" }
        }
        $final = Invoke-KasKoldHostCargoMetadata -Root $Root -Manifest $Manifest -ExtraArguments @('--filter-platform','wasm32-unknown-unknown','--locked') -Capture
        if ($final.ExitCode -ne 0 -or -not (Test-KasKoldMetadataRustCompatibility -Json $final.Output -MaxRust $env:KASKOLD_REPRO_HOST_RUST)) {
            Copy-Item $backup $lockPath -Force
            throw "MSRV-reconciled $Label lock is not valid under reproducible Rust $($env:KASKOLD_REPRO_HOST_RUST)."
        }
        Write-Host "  MSRV-compatible: sha256=$(Get-KasKoldLockSha256 $lockPath) packages=$(Get-KasKoldLockPackageCount $lockPath)"
    } finally { Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue }
}

function Repair-KasKoldCompanionMsrvLock {
    param([Parameter(Mandatory = $true)][string]$Root)
    Repair-KasKoldWebMsrvLock $Root 'KasKold Companion Web' 'apps/kaskold-companion-web/Cargo.toml' 'apps/kaskold-companion-web/Cargo.lock'
}

function Repair-KasKoldVaultWebMsrvLock {
    param([Parameter(Mandatory = $true)][string]$Root)
    Repair-KasKoldWebMsrvLock $Root 'KasKold Vault Web' 'apps/kaskold-vault-web/Cargo.toml' 'apps/kaskold-vault-web/Cargo.lock'
}

function Repair-KasKoldHostLocks {
    param([Parameter(Mandatory = $true)][string]$Root)
    Require-KasKoldCommand rustup 'Install rustup for Windows and reopen PowerShell.' | Out-Null
    Get-KasKoldPython | Out-Null
    $probe = Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run',$env:KASKOLD_STABLE_RUST,'cargo','--version')
    if ($probe.ExitCode -ne 0) {
        Write-Host "==> Installing pinned host Rust $($env:KASKOLD_STABLE_RUST) for lock verification"
        Invoke-KasKoldCommand -Command 'rustup' -Arguments @('toolchain','install',$env:KASKOLD_STABLE_RUST,'--profile','minimal') | Out-Null
    }
    Repair-KasKoldOneHostLock $Root 'Root workspace' 'Cargo.toml' 'Cargo.lock'
    Repair-KasKoldOneHostLock $Root 'Signer firmware workspace' 'apps/kaskold-hardware/Cargo.toml' 'apps/kaskold-hardware/Cargo.lock'
    Repair-KasKoldOneHostLock $Root 'KasKold Companion Web' 'apps/kaskold-companion-web/Cargo.toml' 'apps/kaskold-companion-web/Cargo.lock'
    Repair-KasKoldOneHostLock $Root 'KasKold Vault Web' 'apps/kaskold-vault-web/Cargo.toml' 'apps/kaskold-vault-web/Cargo.lock'
    Repair-KasKoldCompanionMsrvLock $Root
    Repair-KasKoldVaultWebMsrvLock $Root
    Repair-KasKoldOneHostLock $Root 'External rqrr workspace' 'external/rqrr-nostd/Cargo.toml' 'external/rqrr-nostd/Cargo.lock'
    Repair-KasKoldOneHostLock $Root 'Funded/tools workspace' 'tools/Cargo.toml' 'tools/Cargo.lock'
    Repair-KasKoldOneHostLock $Root 'QA workspace' 'qa/Cargo.toml' 'qa/Cargo.lock'
}
