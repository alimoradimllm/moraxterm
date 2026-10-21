[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$Executable,
  [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Executable).Path
$out = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $out | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $out ([IO.Path]::GetFileName($exe))) -Force
New-Item -ItemType Directory -Force -Path (Join-Path $out 'profile') | Out-Null
@'
Moraxterm portable edition

Launch Moraxterm.exe from this directory. The app stores its session profile under .\profile and does not install services, registry entries, or require administrator/UAC elevation.
This bundle contains only the application executable. Windows WebView2 and protocol clients (OpenSSH, RDP, etc.) remain system prerequisites.
'@ | Set-Content -LiteralPath (Join-Path $out 'PORTABLE-README.txt') -Encoding UTF8
Write-Output "Portable bundle staged at $out"
