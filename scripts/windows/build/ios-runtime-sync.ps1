$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
$python = Get-KasKoldPython
& $python (Join-Path $root 'tools/build/web/build_companion_runtime.py') --mode release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& $python (Join-Path $root 'tools/build/ios/sync_runtime.py')
exit $LASTEXITCODE
