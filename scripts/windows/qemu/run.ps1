[CmdletBinding()]
param([switch]$TestOnly,[Parameter(ValueFromRemainingArguments=$true)][string[]]$RemainingArgs)

$utf8 = New-Object System.Text.UTF8Encoding -ArgumentList $false
[Console]::OutputEncoding = $utf8
$OutputEncoding = $utf8

$unsupportedArgs = @($RemainingArgs | Where-Object { -not [string]::IsNullOrEmpty($_) })
if ($unsupportedArgs.Count -gt 0) {
    [Console]::Error.WriteLine("ERROR: unsupported QEMU run argument: $($unsupportedArgs[0])")
    exit 2
}

$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
try {
    . (Join-Path $PSScriptRoot '../lib/qemu-common.ps1')
    Invoke-KasKoldQemuHarness -KeepRunning:(-not $TestOnly)
    exit 0
} catch {
    [Console]::Error.WriteLine("ERROR: QEMU setup/test failed: $($_.Exception.Message)")
    [Console]::Error.WriteLine("PowerShell exception type: $($_.Exception.GetType().FullName)")
    [Console]::Error.WriteLine("PowerShell category: $($_.CategoryInfo)")
    if ($_.InvocationInfo -and $_.InvocationInfo.PositionMessage) {
        [Console]::Error.WriteLine($_.InvocationInfo.PositionMessage)
    }
    if ($_.ScriptStackTrace) {
        [Console]::Error.WriteLine("PowerShell script stack:`n$($_.ScriptStackTrace)")
    }
    exit 1
}
