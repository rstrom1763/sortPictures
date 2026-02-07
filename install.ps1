# install.ps1 - Idempotent installer for sortPictures
# This script builds the project, installs it to %APPDATA%\sortPictures, and updates the registry.

$ErrorActionPreference = "Stop"

$projectName = "sortPictures"
$installDir = Join-Path $env:APPDATA $projectName
$exeName = "$projectName.exe"
$targetExePath = Join-Path $installDir $exeName

Write-Host "--- Installing $projectName ---"

# 1. Build the project
Write-Host "Building project in release mode..."
cargo build --release

$sourceExePath = "target\release\$exeName"
if (-not (Test-Path $sourceExePath)) {
    Write-Error "Could not find compiled binary at $sourceExePath"
    exit 1
}

# 2. Create installation directory
if (-not (Test-Path $installDir)) {
    Write-Host "Creating installation directory: $installDir"
    New-Item -ItemType Directory -Path $installDir | Out-Null
} else {
    Write-Host "Installation directory already exists: $installDir"
}

# 3. Copy binary
Write-Host "Copying $exeName to $installDir..."
Copy-Item -Path $sourceExePath -Destination $targetExePath -Force

# 4. Update Registry
Write-Host "Updating Windows Registry for context menu..."

# Helper function to set registry keys
function Set-SortPicturesRegistry {
    param($baseKey)
    
    $shellKey = "$baseKey\shell\SortPictures"
    $commandKey = "$shellKey\command"
    
    if (-not (Test-Path "Registry::$shellKey")) {
        New-Item -Path "Registry::$shellKey" -Force | Out-Null
    }
    Set-ItemProperty -Path "Registry::$shellKey" -Name "(Default)" -Value "Sort Pictures Here"
    Set-ItemProperty -Path "Registry::$shellKey" -Name "Icon" -Value "shell32.dll,141"
    
    if (-not (Test-Path "Registry::$commandKey")) {
        New-Item -Path "Registry::$commandKey" -Force | Out-Null
    }
    
    # %V for background, %1 for folder item. We handle them specifically.
    if ($baseKey -match "Background") {
        $commandValue = "`"$targetExePath`" `"%V`""
    } else {
        $commandValue = "`"$targetExePath`" `"%1`""
    }
    
    Set-ItemProperty -Path "Registry::$commandKey" -Name "(Default)" -Value $commandValue
}

# HKEY_CURRENT_USER is safer as it doesn't always require Admin and is user-specific.
# Windows Explorer also checks HKCU\Software\Classes before HKCR.
$registryBase = "HKCU\Software\Classes"

Set-SortPicturesRegistry -baseKey "$registryBase\Directory"
Set-SortPicturesRegistry -baseKey "$registryBase\Directory\Background"

Write-Host "Successfully installed and registered $projectName!"
Write-Host "You can now right-click any folder or folder background and select 'Sort Pictures Here'."
