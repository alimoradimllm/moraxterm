[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$Bundle)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Bundle).Path
$exe = Get-ChildItem -LiteralPath $root -Filter '*.exe' | Select-Object -First 1
if (-not $exe) { throw 'portable bundle has no executable' }
if (-not (Test-Path (Join-Path $root 'PORTABLE-README.txt'))) { throw 'portable bundle missing manifest' }
if (-not (Test-Path (Join-Path $root 'profile'))) { throw 'portable bundle missing profile directory' }
if ((Get-Content (Join-Path $root 'PORTABLE-README.txt') -Raw) -notmatch 'does not install services') { throw 'portable manifest missing no-install claim' }
Write-Output "Portable extraction verification passed: $($exe.Name), profile directory, and no-install manifest present."
