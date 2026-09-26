$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
$python=Get-KasKoldPython
& $python (Join-Path $PSScriptRoot 'crap_windows.py') @args
exit $LASTEXITCODE
