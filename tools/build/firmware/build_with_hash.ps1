[CmdletBinding()]
param(
    [Parameter(Position=0)][string]$Label = 'firmware',
    [Parameter()][ValidateSet('m5stack','m5stack-lite')][string]$Board = '',
    [Parameter(ValueFromRemainingArguments=$true)][string[]]$CargoArgs = @()
)
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')

function Read-ExpectedFirmwareHash {
    param([Parameter(Mandatory=$true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "generated firmware hash source not found: $Path" }
    $text = Get-Content -LiteralPath $Path -Raw
    $marker = 'pub static EXPECTED_FIRMWARE_HASH: [u8; 32] = ['
    $markerCount = [regex]::Matches($text, [regex]::Escape($marker)).Count
    if ($markerCount -ne 1) { throw "expected exactly one static EXPECTED_FIRMWARE_HASH declaration; found $markerCount" }
    $pattern = [regex]::Escape($marker) + '\s*(.*?)\s*\];'
    $match = [regex]::Match($text, $pattern, [Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) { throw 'EXPECTED_FIRMWARE_HASH declaration is not terminated canonically' }
    $bytes = [regex]::Matches($match.Groups[1].Value, '0x[0-9a-fA-F]{2}') | ForEach-Object { $_.Value.Substring(2).ToLowerInvariant() }
    if ($bytes.Count -ne 32) { throw "EXPECTED_FIRMWARE_HASH must contain exactly 32 byte literals; found $($bytes.Count)" }
    $residual = [regex]::Replace($match.Groups[1].Value, '0x[0-9a-fA-F]{2}', '')
    $residual = [regex]::Replace($residual, '[\s,]', '')
    if ($residual) { throw 'EXPECTED_FIRMWARE_HASH contains non-canonical content' }
    return ($bytes -join '')
}

if ($Label -eq '--read-generated-hash') {
    if ($CargoArgs.Count -ne 1) { [Console]::Error.WriteLine("usage: build_with_hash.ps1 --read-generated-hash <firmware_hash.rs>"); exit 2 }
    Write-Output (Read-ExpectedFirmwareHash $CargoArgs[0]); exit 0
}

Require-KasKoldCommand cargo | Out-Null
Require-KasKoldCommand espflash | Out-Null
$python = Get-KasKoldPython
$app = Join-Path $root 'apps/kaskold-hardware'
$elf = Join-Path $app 'target/xtensa-esp32s3-none-elf/release/kaskold-hardware'
$hashSource = Join-Path $app 'src/firmware_hash.rs'
$boardHelper = Join-Path $root 'tools/build/firmware/board_layout.py'
$verifyHelper = Join-Path $root 'tools/build/firmware/verify_image_hash.py'
$stackBudgetHelper = Join-Path $root 'qa/checks/firmware/compiled_stack_budget.py'
$lockReconciler = Join-Path $root 'tools/build/firmware/reconcile_tools_lock.py'
$toolsLock = Join-Path $root 'tools/Cargo.lock'
$espflashArgs = @()
if ($Board) {
    & $python $boardHelper 'check' '--board' $Board
    if ($LASTEXITCODE -ne 0) { throw "board layout validation failed for $Board" }
    $espflashArgs = @(& $python $boardHelper 'espflash-args' '--board' $Board)
    if ($LASTEXITCODE -ne 0) { throw "board layout validation failed for $Board" }
}
$genHashKeyArgs = @()
if ($env:KASKOLD_SIGNING_KEY) {
    if (-not (Test-Path -LiteralPath $env:KASKOLD_SIGNING_KEY -PathType Leaf)) { throw 'KASKOLD_SIGNING_KEY does not exist' }
    $keyBytes = [IO.File]::ReadAllBytes($env:KASKOLD_SIGNING_KEY)
    if ($keyBytes.Length -ne 32) { throw "KASKOLD_SIGNING_KEY must be exactly 32 bytes; got $($keyBytes.Length)" }
}
$cargoText = ' ' + ($CargoArgs -join ' ') + ' '
if ($env:KASKOLD_SIGNING_KEY) {
    $signingIdentity = if ($cargoText.Contains('production')) { 'production' } else { 'development' }
    $genHashKeyArgs = @([IO.Path]::GetFullPath($env:KASKOLD_SIGNING_KEY), $signingIdentity)
}
$isLiteBuild = $cargoText.Contains('m5stack-lite')
$isCoreS3Build = $cargoText.Contains('m5stack')
if ($isLiteBuild -and $Board -ne 'm5stack-lite') {
    throw 'CoreS3 Lite builds require explicit -Board m5stack-lite so board qualification policy cannot be bypassed'
}
if ($isCoreS3Build -and -not $isLiteBuild -and $Board -ne 'm5stack') {
    throw 'CoreS3 builds require explicit -Board m5stack so the CoreS3 partition table cannot be omitted'
}
if ($Board -eq 'm5stack-lite' -and -not $isLiteBuild) {
    throw '-Board m5stack-lite requires an m5stack-lite firmware feature set'
}
if ($Board -eq 'm5stack' -and (-not $isCoreS3Build -or $isLiteBuild)) {
    throw '-Board m5stack requires an m5stack (non-Lite) firmware feature set'
}
$tmp = Join-Path ([IO.Path]::GetTempPath()) ('kaskold-fw-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$hashes = @()
$originalToolsLock = [IO.File]::ReadAllBytes($toolsLock)

function Invoke-FirmwareCargoBuild {
    $oldRustFlags = $env:RUSTFLAGS
    $stackFlags = '-Z emit-stack-sizes'
    $env:RUSTFLAGS = if ($oldRustFlags) { "$oldRustFlags $stackFlags" } else { $stackFlags }
    try {
        Invoke-KasKoldCommand -Command 'cargo' -Arguments (@('build','--locked','--release') + $CargoArgs) -WorkingDirectory $app | Out-Null
    } finally {
        if ($null -eq $oldRustFlags) { Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue } else { $env:RUSTFLAGS = $oldRustFlags }
    }
}
try {
    Invoke-KasKoldCommand -Command $python -Arguments @($lockReconciler,'--workspace',(Join-Path $root 'tools')) -WorkingDirectory $root | Out-Null
    for ($pass = 1; $pass -le 5; $pass++) {
        Invoke-FirmwareCargoBuild
        $image = Join-Path $tmp "$Label-pass$pass.bin"
        Invoke-KasKoldCommand -Command 'espflash' -Arguments (@('save-image','--chip','esp32s3') + $espflashArgs + @($elf,$image)) -WorkingDirectory $root | Out-Null
        Invoke-KasKoldCommand -Command 'cargo' -Arguments (@('run','--locked','--manifest-path',(Join-Path $root 'tools/Cargo.toml'),'--bin','gen-hash','--release','--',$image) + $genHashKeyArgs) -WorkingDirectory $root | Out-Null
        $hash = Read-ExpectedFirmwareHash $hashSource
        $hashes += $hash
        Write-Host "pass $pass`: $hash"
    }
    if (($hashes[1] -ne $hashes[2]) -or ($hashes[2] -ne $hashes[3]) -or ($hashes[3] -ne $hashes[4])) { throw 'passes 2 through 5 did not converge; generated identity must not affect executable bytes' }
    Invoke-FirmwareCargoBuild
    Invoke-KasKoldCommand -Command $python -Arguments @($stackBudgetHelper,$elf) -WorkingDirectory $root | Out-Null
    $finalImage = Join-Path $tmp "$Label-final.bin"
    Invoke-KasKoldCommand -Command 'espflash' -Arguments (@('save-image','--chip','esp32s3') + $espflashArgs + @($elf,$finalImage)) -WorkingDirectory $root | Out-Null
    Invoke-KasKoldCommand -Command $python -Arguments @($verifyHelper,$finalImage,$hashSource) -WorkingDirectory $root | Out-Null
    $finalHash = Read-ExpectedFirmwareHash $hashSource
    if ($finalHash -ne $hashes[4]) { throw 'final embedded hash drifted after convergence' }
    Write-Host "converged: $($hashes[4])"
    if ($env:KASKOLD_FINAL_IMAGE_OUT) {
        $destination = [IO.Path]::GetFullPath($env:KASKOLD_FINAL_IMAGE_OUT)
        $parent = Split-Path -Parent $destination
        if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        Copy-Item -LiteralPath $finalImage -Destination $destination -Force
        Write-Host "final image: $destination"
    }
} finally {
    [IO.File]::WriteAllBytes($toolsLock, $originalToolsLock)
    Remove-KasKoldPath $tmp
}
