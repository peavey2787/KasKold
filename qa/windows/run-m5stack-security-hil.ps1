# Read-only post-provision CoreS3 production-security evidence collector.
# This script NEVER burns eFuses and NEVER writes/erases flash.
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Port,
    [Parameter(Position = 1)][string]$OutputDir = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')

if (-not $OutputDir) { $OutputDir = Join-Path $root 'target/qa/state/m5stack-security-hil' }
$policyPath = Join-Path $root 'apps/kaskold-hardware/release-policy.env'
$policy = @{}
foreach ($line in Get-Content -LiteralPath $policyPath) {
    $trimmed = $line.Trim()
    if (-not $trimmed -or $trimmed.StartsWith('#')) { continue }
    $parts = $trimmed.Split('=', 2)
    if ($parts.Count -eq 2) { $policy[$parts[0]] = $parts[1] }
}
foreach ($required in @('KASKOLD_ESPTOOL_VERSION','KASKOLD_UPDATE_SEQUENCE','KASKOLD_SECURITY_VERSION')) {
    if (-not $policy.ContainsKey($required)) { throw "Missing $required in release-policy.env" }
}

$python = Get-KasKoldPython
$toolRoot = Join-Path $root ('target/qa/state/tools/esptool-' + $policy.KASKOLD_ESPTOOL_VERSION)
$venvPython = Join-Path $toolRoot 'Scripts/python.exe'
$esptool = Join-Path $toolRoot 'Scripts/esptool.exe'
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $toolRoot) | Out-Null

if (-not (Test-Path -LiteralPath $venvPython -PathType Leaf)) {
    Write-Host "==> Bootstrapping pinned esptool $($policy.KASKOLD_ESPTOOL_VERSION) into repository QA state"
    Invoke-KasKoldCommand -Command $python -Arguments @('-m','venv',$toolRoot) -WorkingDirectory $root | Out-Null
    Invoke-KasKoldCommand -Command $venvPython -Arguments @(
        '-m','pip','install','--disable-pip-version-check',('esptool==' + $policy.KASKOLD_ESPTOOL_VERSION)
    ) -WorkingDirectory $root | Out-Null
}

$version = (Invoke-KasKoldCapture -Command $esptool -Arguments @('version') -WorkingDirectory $root)
if ($version.ExitCode -ne 0 -or $version.Output -notmatch [regex]::Escape($policy.KASKOLD_ESPTOOL_VERSION)) {
    throw "Pinned esptool version mismatch: $($version.Output)"
}

$humanReport = Join-Path $OutputDir 'get-security-info.txt'
& $esptool --chip esp32s3 --port $Port get-security-info 2>&1 | Tee-Object -FilePath $humanReport
if ($LASTEXITCODE -ne 0) { throw "esptool get-security-info failed with exit code $LASTEXITCODE" }

$stateJson = Join-Path $OutputDir 'security-state.json'
$rawJson = Join-Path $OutputDir 'security-info-raw.json'
Invoke-KasKoldCommand -Command $venvPython -Arguments @(
    (Join-Path $root 'qa/linux/lib/collect_esptool_security.py'),
    $Port,$stateJson,$rawJson
) -WorkingDirectory $root | Out-Null

$collection = Join-Path $OutputDir 'collection.json'
$collector = @'
import hashlib, json, pathlib, sys
out = pathlib.Path(sys.argv[1])
state = json.loads((out / "security-state.json").read_text())
report = out / "get-security-info.txt"
raw = out / "security-info-raw.json"
payload = {
    "collector": "read-only-post-provision-v4",
    "esptool_pin": sys.argv[2],
    "esptool_version_output": sys.argv[3],
    "update_sequence": int(sys.argv[4]),
    "security_version_policy": int(sys.argv[5]),
    "security_state": state,
    "report": report.name,
    "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
    "structured_report": raw.name,
    "structured_report_sha256": hashlib.sha256(raw.read_bytes()).hexdigest(),
    "notes": "SECURE_VERSION/eFuse detail is bound from provisioning-prelock evidence plus the dedicated anti-rollback HIL fixture; Secure Download mode intentionally blocks espefuse summary after lockdown.",
}
(out / "collection.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
'@
Invoke-KasKoldCommand -Command $venvPython -Arguments @(
    '-c',$collector,$OutputDir,$policy.KASKOLD_ESPTOOL_VERSION,$version.Output,
    $policy.KASKOLD_UPDATE_SEQUENCE,$policy.KASKOLD_SECURITY_VERSION
) -WorkingDirectory $root | Out-Null

Write-Host "Read-only M5Stack security evidence collected in: $OutputDir"
