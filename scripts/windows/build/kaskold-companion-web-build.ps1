# Native Windows facade for kaskold-companion-web-build.
& (Join-Path $PSScriptRoot '../lib/_invoke.ps1') -Target 'apps/kaskold-companion-web/build.ps1' -CommandArguments ([string[]]$args)
exit $LASTEXITCODE
