$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
. (Join-Path $root 'scripts/windows/lib/common.ps1')
Import-KasKoldToolchains $root
$toolchain=if($env:CRAP_BRANCH_TOOLCHAIN){$env:CRAP_BRANCH_TOOLCHAIN}else{$env:KASKOLD_BRANCH_RUST}
$llvm=if($env:CRAP_LLVM_COV_VERSION){$env:CRAP_LLVM_COV_VERSION}else{$env:KASKOLD_CARGO_LLVM_COV_VERSION}
$crap=if($env:CRAP_CARGO_CRAP_VERSION){$env:CRAP_CARGO_CRAP_VERSION}else{$env:KASKOLD_CARGO_CRAP_VERSION}
foreach($c in @('rustup','cargo')){Require-KasKoldCommand $c|Out-Null}

function Refresh-ProcessPath {
 $machine=[Environment]::GetEnvironmentVariable('Path','Machine')
 $user=[Environment]::GetEnvironmentVariable('Path','User')
 $entries=New-Object System.Collections.Generic.List[string]
 foreach($value in @($env:PATH,$machine,$user)){
  if(-not $value){continue}
  foreach($entry in ($value -split ';')){
   if($entry -and -not $entries.Contains($entry)){$entries.Add($entry)}
  }
 }
 $env:PATH=($entries -join ';')
}
function Ensure-Node {
 if(Get-Command node -ErrorAction SilentlyContinue){
  $version=(& node --version 2>$null | Select-Object -First 1)
  Write-Host ("  {0,-17}{1}" -f 'node:',([string]$version).Trim())
  return
 }
 $winget=Get-Command winget.exe -ErrorAction SilentlyContinue
 if(-not $winget){
  throw 'Node.js is required for browser coverage. Windows Package Manager (winget) is unavailable; install App Installer or install Node.js LTS, then rerun this setup.'
 }
 Write-Host 'Node.js is missing; installing OpenJS.NodeJS.LTS with winget...'
 & $winget.Source install --id OpenJS.NodeJS.LTS --exact --accept-package-agreements --accept-source-agreements
 if($LASTEXITCODE -ne 0){throw "winget failed to install OpenJS.NodeJS.LTS with exit code $LASTEXITCODE"}
 Refresh-ProcessPath
 if(-not (Get-Command node -ErrorAction SilentlyContinue)){
  foreach($candidate in @(
   (Join-Path $env:ProgramFiles 'nodejs'),
   $(if($env:LOCALAPPDATA){Join-Path $env:LOCALAPPDATA 'Programs/nodejs'}else{$null})
  )){
   if($candidate -and (Test-Path -LiteralPath (Join-Path $candidate 'node.exe') -PathType Leaf)){
    $env:PATH=$candidate+';'+$env:PATH
    break
   }
  }
 }
 $node=Get-Command node -ErrorAction SilentlyContinue
 if(-not $node){throw 'Node.js LTS was installed but node.exe is still unavailable in this process. Open a new terminal and rerun make qa.'}
 $version=(& $node.Source --version 2>$null | Select-Object -First 1)
 Write-Host ("  {0,-17}{1} (installed)" -f 'node:',([string]$version).Trim())
}
Ensure-Node

# Prefer a pin-qualified repository-local install when provisioning is needed,
# but reuse an exact matching Cargo plugin already available on PATH first. The
# target tree is disposable, so reuse an exact matching Cargo plugin already
# available on the normal Cargo PATH before provisioning a local copy.
$toolRoot=Join-Path $root ("target/development-tools/branch-coverage-{0}-llvm-{1}-crap-{2}" -f $toolchain,$llvm,$crap)
$toolBin=Join-Path $toolRoot 'bin'
New-Item -ItemType Directory -Force -Path $toolBin|Out-Null
$pathEntries=@($env:PATH -split ';'|Where-Object{$_})
if($pathEntries -notcontains $toolBin){$env:PATH=$toolBin+';'+$env:PATH}

Write-Host "Provisioning pinned branch-coverage tools:`n  Toolchain:       $toolchain`n  Local tool root: $toolRoot"
$p=Invoke-KasKoldCapture -Command 'rustup' -Arguments @('run',$toolchain,'rustc','--version')
if($p.ExitCode -eq 0){Write-Host '  Rust toolchain:  already installed'}else{Invoke-KasKoldCommand -Command 'rustup' -Arguments @('toolchain','install',$toolchain,'--profile','minimal','--component','llvm-tools-preview')|Out-Null}
$components=Invoke-KasKoldCapture -Command 'rustup' -Arguments @('component','list','--toolchain',$toolchain,'--installed')
if($components.Output -match '(?m)^llvm-tools'){Write-Host '  LLVM tools:      already installed'}else{Invoke-KasKoldCommand -Command 'rustup' -Arguments @('component','add','llvm-tools-preview','--toolchain',$toolchain)|Out-Null;Write-Host '  LLVM tools:      installed'}
function Get-PluginVersion([string]$sub){
 $probe=Invoke-KasKoldCapture -Command 'cargo' -Arguments @("+$toolchain",$sub,'--version')
 return $probe
}
function Ensure-Plugin([string]$sub,[string]$package,[string]$expected){
 $exe=Join-Path $toolBin ($package+'.exe')

 # Probe through the exact Cargo invocation used by the QA job itself. This
 # keeps discovery and execution on the same rustup proxy/toolchain path and
 # naturally prefers the repository-local bin because it is first on PATH.
 $probe=Get-PluginVersion $sub
 if($probe.ExitCode -eq 0 -and $probe.Output.Trim().EndsWith(" $expected")){
  Write-Host ("  {0,-17}{1}" -f ($package+':'),$probe.Output.Trim())
  return
 }

 # A stale QA-owned local executable can shadow a correct user installation.
 # Remove only that disposable local copy, then probe the normal Cargo PATH
 # once more before attempting any installation.
 if(Test-Path -LiteralPath $exe -PathType Leaf){
  Remove-Item -LiteralPath $exe -Force
  $probe=Get-PluginVersion $sub
  if($probe.ExitCode -eq 0 -and $probe.Output.Trim().EndsWith(" $expected")){
   Write-Host ("  {0,-17}{1} (reused installed tool)" -f ($package+':'),$probe.Output.Trim())
   return
  }
 }

 # Provision only when no exact usable installation exists. Cargo already
 # reuses its registry/source/build caches, so an extra failed cache-only install
 # probe only adds a second failure mode on Windows and can leave a stale native
 # exit status in the parent PowerShell process.
 Write-Host "Installing $package $expected for $toolchain into repository-local tooling..."
 $installArgs=@("+$toolchain",'install',$package,'--version',$expected,'--locked','--root',$toolRoot)
 try {
  Invoke-KasKoldCommand -Command 'cargo' -Arguments $installArgs|Out-Null
 } catch {
  throw "unable to provision pinned $package $expected. Restore network/DNS access or install it once with: cargo +$toolchain install $package --version $expected --locked"
 }

 $final=Get-PluginVersion $sub
 if($final.ExitCode -ne 0 -or -not $final.Output.Trim().EndsWith(" $expected")){
  throw "expected $package $expected, found: $($final.Output)"
 }
 Write-Host ("  {0,-17}{1}" -f ($package+':'),$final.Output.Trim())
}
Ensure-Plugin 'llvm-cov' 'cargo-llvm-cov' $llvm
Ensure-Plugin 'crap' 'cargo-crap' $crap
Write-Host 'Pinned branch-coverage tools are ready.'
