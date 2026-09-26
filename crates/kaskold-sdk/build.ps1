$Dir = $PSScriptRoot
$Root = [IO.Path]::GetFullPath((Join-Path $Dir '../..'))
$OutputRoot = if ($env:KASKOLD_SDK_OUTPUT_ROOT) { $env:KASKOLD_SDK_OUTPUT_ROOT } else { Join-Path $Root 'target/sdk' }
$PkgDir = Join-Path $OutputRoot 'kaskold-sdk/pkg'
& (Join-Path $Dir '../../scripts/windows/lib/rust-wasm-sdk.ps1') -Package 'kaskold-sdk' -WasmStem 'kaskold_sdk' -PkgDir $PkgDir -Label 'KasKold SDK Rust/WASM' -NpmName '@kaskold/sdk'
if ($LASTEXITCODE) { exit $LASTEXITCODE }
