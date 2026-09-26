$Dir = $PSScriptRoot
$Root = [IO.Path]::GetFullPath((Join-Path $Dir '../..'))
$OutputRoot = if ($env:KASKOLD_SDK_OUTPUT_ROOT) { $env:KASKOLD_SDK_OUTPUT_ROOT } else { Join-Path $Root 'target/sdk' }
$PkgDir = Join-Path $OutputRoot 'kaskold-protocol/pkg'
& (Join-Path $Dir '../../scripts/windows/lib/rust-wasm-sdk.ps1') -Package 'kaskold-protocol' -WasmStem 'kaskold_protocol' -PkgDir $PkgDir -Label 'KasKold protocol Rust/WASM' -NpmName '@kaskold/protocol'
if ($LASTEXITCODE) { exit $LASTEXITCODE }
