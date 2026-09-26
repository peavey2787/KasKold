$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
$evidence = $env:KASKOLD_RELEASE_EVIDENCE_DIR
$source = $env:KASKOLD_SOURCE_SHA256
$release = $env:KASKOLD_RELEASE_ARTIFACT_SHA256
$releaseManifest = $env:KASKOLD_RELEASE_MANIFEST
$trustPolicy = $env:KASKOLD_RELEASE_TRUST_POLICY
$trustPolicySha = $env:KASKOLD_RELEASE_TRUST_POLICY_SHA256
if (-not $evidence -or -not $source -or -not $release -or -not $releaseManifest -or -not $trustPolicy -or -not $trustPolicySha) {
    [Console]::Error.WriteLine('ERROR: release readiness requires a concrete release artifact and signed evidence.')
    [Console]::Error.WriteLine('Set KASKOLD_RELEASE_EVIDENCE_DIR, KASKOLD_SOURCE_SHA256, KASKOLD_RELEASE_ARTIFACT_SHA256, KASKOLD_RELEASE_MANIFEST, KASKOLD_RELEASE_TRUST_POLICY, and KASKOLD_RELEASE_TRUST_POLICY_SHA256.')
    [Console]::Error.WriteLine('See qa/release/README.md; these values cannot be synthesized safely by the launcher.')
    exit 2
}
Require-KasKoldCommand 'openssl' | Out-Null
$python = Get-KasKoldPython
Invoke-KasKoldCommand -Command $python -Arguments @(
    (Join-Path $root 'qa/checks/release/release_readiness.py'),
    '--evidence-dir',$evidence,
    '--source-sha256',$source,
    '--release-artifact-sha256',$release,
    '--release-manifest',$releaseManifest,
    '--trust-policy',$trustPolicy,
    '--trust-policy-sha256',$trustPolicySha
) -WorkingDirectory $root | Out-Null
