$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $PSScriptRoot '../lib/qemu-common.ps1')
Initialize-KasKoldQemuEnvironment
& (Join-Path $root 'tools/firmware/qemu/build.ps1')
exit $LASTEXITCODE
