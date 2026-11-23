param(
  [string]$Version = "0.1.0",
  [string]$Arch = "x64"
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path "$PSScriptRoot\.."
$Dist = Join-Path $Root "dist"
$Stage = Join-Path $Dist "LingXiT3-Native-$Version-windows-$Arch"
$ZipPath = Join-Path $Dist "LingXiT3-Native-$Version-windows-$Arch.zip"

Set-Location $Root
cargo build --release --bin lingxi-t3-native

if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Root "target\release\lingxi-t3-native.exe") $Stage
Copy-Item (Join-Path $Root "assets\icons\icon-512.png") $Stage

if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path "$Stage\*" -DestinationPath $ZipPath
Write-Host "Packaged Windows archive: $ZipPath"
