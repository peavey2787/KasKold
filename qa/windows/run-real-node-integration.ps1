# This gate intentionally accepts no parameters. Use PowerShell's automatic $args
# collection so a zero-argument invocation cannot be turned into a phantom
# positional value by parameter binding on Windows PowerShell 5.1.
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
. (Join-Path $root 'scripts/windows/lib/cargo_locks.ps1')
Import-KasKoldToolchains $root
if (@($args).Count -ne 0) {
    [Console]::Error.WriteLine("Usage: $($MyInvocation.MyCommand.Path)")
    [Console]::Error.WriteLine("This gate uses Kaspa's public-node resolver only; no local-node mode exists.")
    exit 2
}
Write-Host "==> Reconciling/verifying host Cargo.lock files under pinned Cargo $($env:KASKOLD_STABLE_RUST)"
Repair-KasKoldHostLocks $root
Write-Host '==> Building the real KasKold Companion WebAssembly package'
$python = Get-KasKoldPython
& (Join-Path $root 'scripts/windows/build/kaskold-companion-web-build.ps1')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host '==> Real Kaspa public-node integration (official resolver pool)'
$evidence = 'target/qa/security/real-node-integration.json'
Invoke-KasKoldCommand -Command $python -Arguments @('qa/checks/integration/real_node_browser.py','--evidence',$evidence) -WorkingDirectory $root | Out-Null
if (-not $env:KASKOLD_SECURITY_RUN_DIR -and -not $env:KASKOLD_QA_CATALOG_ACTIVE) {
    Invoke-KasKoldCommand -Command $python -Arguments @('qa/checks/security/complete_hardening.py','--real-node-evidence',$evidence) -WorkingDirectory $root | Out-Null
}
