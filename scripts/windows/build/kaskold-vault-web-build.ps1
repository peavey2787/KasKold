# Native Windows facade for kaskold-vault-web-build.
& (Join-Path $PSScriptRoot '../lib/_invoke.ps1') -Target 'apps/kaskold-vault-web/build.ps1' -CommandArguments ([string[]]$args)
exit $LASTEXITCODE
