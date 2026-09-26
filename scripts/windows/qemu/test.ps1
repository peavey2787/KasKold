Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (@($args).Count -ne 0) {
    [Console]::Error.WriteLine("ERROR: qemu-test accepts no positional arguments; received: $($args[0])")
    exit 2
}

$utf8 = New-Object System.Text.UTF8Encoding -ArgumentList $false
[Console]::OutputEncoding = $utf8
$OutputEncoding = $utf8

$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$logDir = Join-Path $root 'target/qa/qemu'
$transcript = Join-Path $logDir 'windows-qemu-powershell.log'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$transcriptStarted = $false
$status = 1

try {
    try {
        Start-Transcript -LiteralPath $transcript -Force | Out-Null
        $transcriptStarted = $true
    } catch {
        [Console]::Error.WriteLine("WARNING: PowerShell transcript could not start: $($_.Exception.Message)")
    }

    [Console]::Out.WriteLine("QEMU PowerShell transcript: $transcript")
    . (Join-Path $PSScriptRoot '../lib/qemu-common.ps1')
    Invoke-KasKoldQemuHarness
    [Console]::Out.WriteLine('PASS: native Windows ESP32-S3 QEMU test completed.')
    $status = 0
} catch {
    [Console]::Error.WriteLine("ERROR: Windows QEMU test failed: $($_.Exception.Message)")
    [Console]::Error.WriteLine("PowerShell exception type: $($_.Exception.GetType().FullName)")
    [Console]::Error.WriteLine("PowerShell category: $($_.CategoryInfo)")
    if ($_.InvocationInfo -and $_.InvocationInfo.PositionMessage) {
        [Console]::Error.WriteLine($_.InvocationInfo.PositionMessage)
    }
    if ($_.ScriptStackTrace) {
        [Console]::Error.WriteLine("PowerShell script stack:`n$($_.ScriptStackTrace)")
    }
    [Console]::Error.WriteLine("PowerShell transcript: $transcript")
    $status = 1
} finally {
    if ($transcriptStarted) {
        try { Stop-Transcript | Out-Null } catch { }
    }
}

exit $status
