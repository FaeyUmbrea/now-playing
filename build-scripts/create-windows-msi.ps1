param(
  [string]$Tag
)

if (-not $Tag) {
  if ($env:RELEASE_TAG) { $Tag = $env:RELEASE_TAG } else { Write-Error "Tag is required as argument or RELEASE_TAG env var"; exit 1 }
}

Write-Host "Creating MSI for tag: $Tag"

# Install WiX via Chocolatey if available (best-effort)
try {
  Write-Host "Installing WiX Toolset via Chocolatey (may already be present)"
  choco install -y wixtoolset -r | Out-Null
} catch {
  Write-Host "choco install may have failed or choco is not available; continuing and checking for existing WiX tools"
}

$distDir = Join-Path $PSScriptRoot 'dist'
if (-not (Test-Path $distDir)) { New-Item -ItemType Directory -Path $distDir | Out-Null }
$windowsDir = Join-Path $PSScriptRoot 'windows'

# Find the first exe in the windows dir to package
$exe = Get-ChildItem -Path $windowsDir -Filter *.exe | Select-Object -First 1
if ($null -eq $exe) { Write-Host "No executable found in windows/ to package; skipping MSI creation"; exit 0 }

# Use a fixed UpgradeCode GUID so MSI upgrades are recognized across versions.
# This GUID must remain constant for all releases. Generate once and hardcode it here.
$upgradeCode = '80f7e72a-b935-4004-b866-c4d63bff20b1'

# Generate a per-build ProductCode (ProductCode must change for each product version)
$productGuid = [guid]::NewGuid().ToString()

# Component GUID remains per-build (or can be made stable if desired)
$componentGuid = [guid]::NewGuid().ToString()
$version = $Tag.TrimStart('v')

# Build the WiX XML with interpolation
$wxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="$productGuid" Name="Now Playing" Language="1033" Version="$version" Manufacturer="Now Playing" UpgradeCode="$upgradeCode">
    <Package InstallerVersion="500" Compressed="yes" InstallScope="perMachine" />
    <MediaTemplate />
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLFOLDER" Name="Now Playing" />
      </Directory>
    </Directory>
    <DirectoryRef Id="INSTALLFOLDER">
      <Component Id="MainExecutable" Guid="$componentGuid">
        <File Source="windows\$($exe.Name)" KeyPath="yes" />
      </Component>
    </DirectoryRef>
    <Feature Id="DefaultFeature" Level="1">
      <ComponentRef Id="MainExecutable" />
    </Feature>
  </Product>
</Wix>
"@

$wxsPath = Join-Path $PSScriptRoot 'product.wxs'
$wxs | Out-File -FilePath $wxsPath -Encoding utf8

# Typical WiX install paths for candle.exe and light.exe
$candle = 'C:\Program Files (x86)\WiX Toolset v3.11\bin\candle.exe'
$light = 'C:\Program Files (x86)\WiX Toolset v3.11\bin\light.exe'

if (Test-Path $candle -and Test-Path $light) {
  Push-Location $PSScriptRoot
  & $candle -out product.wixobj product.wxs
  & $light -out "now-playing-$Tag.msi" product.wixobj
  if (Test-Path "now-playing-$Tag.msi") {
    Move-Item -Path "now-playing-$Tag.msi" -Destination $distDir -Force
    Write-Host "MSI created: $(Join-Path $distDir ("now-playing-$Tag.msi"))"
  } else {
    Write-Host "MSI was not created by light.exe"
  }
  Pop-Location
} else {
  Write-Host "WiX tools not found at expected path: $candle and $light; MSI not created"
}
