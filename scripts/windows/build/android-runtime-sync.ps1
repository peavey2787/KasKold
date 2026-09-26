$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
$python = Get-KasKoldPython
& $python (Join-Path $root 'tools/build/web/build_companion_runtime.py') --mode release
exit $LASTEXITCODE
