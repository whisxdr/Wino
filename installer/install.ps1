<#
.SYNOPSIS
    Wino Native Windows Installer Script
.DESCRIPTION
    Installs Wino into Program Files, creates Start Menu and Desktop shortcuts,
    and registers Wino in Windows Settings -> Installed Apps (Add/Remove Programs).
#>

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:ProgramFiles\Wino"
)

$ErrorActionPreference = "Stop"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "       Wino Windows Installer v2.6.0     " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# 1. Determine Source Directory and Executable
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourceExe = "$scriptDir\..\target\release\wino.exe"
if (-not (Test-Path $sourceExe)) {
    $sourceExe = "$scriptDir\wino.exe"
}

if (-not (Test-Path $sourceExe)) {
    Write-Error "wino.exe not found! Please build the release binary first (`cargo build --release`)."
    exit 1
}

# 2. Check Administrator Privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $InstallDir = "$env:LOCALAPPDATA\Programs\Wino"
    Write-Host "[!] Running as standard user. Installing to: $InstallDir" -ForegroundColor Yellow
} else {
    Write-Host "[+] Running with Administrator privileges. Target: $InstallDir" -ForegroundColor Green
}

# 3. Stop running instances of Wino
Get-Process -Name "wino" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 4. Create Target Directory & Copy Files
Write-Host "[+] Installing application files..." -ForegroundColor Gray
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Path $sourceExe -Destination "$InstallDir\wino.exe" -Force
if (Test-Path "$scriptDir\uninstall.ps1") {
    Copy-Item -Path "$scriptDir\uninstall.ps1" -Destination "$InstallDir\uninstall.ps1" -Force
}

# 5. Create Start Menu Shortcut
$wsh = New-Object -ComObject WScript.Shell
$startMenuFolder = if ($isAdmin) { "$env:ProgramData\Microsoft\Windows\Start Menu\Programs\Wino" } else { "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Wino" }
New-Item -ItemType Directory -Force -Path $startMenuFolder | Out-Null

$shortcut = $wsh.CreateShortcut("$startMenuFolder\Wino.lnk")
$shortcut.TargetPath = "$InstallDir\wino.exe"
$shortcut.WorkingDirectory = $InstallDir
$shortcut.Description = "Wino - Native Windows Debloater, Optimizer & Memory Suite"
$shortcut.Save()

# 6. Create Desktop Shortcut
$desktopFolder = [Environment]::GetFolderPath("Desktop")
$desktopShortcut = $wsh.CreateShortcut("$desktopFolder\Wino.lnk")
$desktopShortcut.TargetPath = "$InstallDir\wino.exe"
$desktopShortcut.WorkingDirectory = $InstallDir
$desktopShortcut.Description = "Wino - Native Windows Debloater, Optimizer & Memory Suite"
$desktopShortcut.Save()

# 7. Register in Windows Settings / Add Remove Programs
$regPath = if ($isAdmin) { "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wino" } else { "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wino" }
New-Item -Path $regPath -Force | Out-Null

Set-ItemProperty -Path $regPath -Name "DisplayName" -Value "Wino"
Set-ItemProperty -Path $regPath -Name "DisplayVersion" -Value "2.6.0"
Set-ItemProperty -Path $regPath -Name "Publisher" -Value "whisxdr"
Set-ItemProperty -Path $regPath -Name "DisplayIcon" -Value "$InstallDir\wino.exe,0"
Set-ItemProperty -Path $regPath -Name "InstallLocation" -Value $InstallDir
Set-ItemProperty -Path $regPath -Name "UninstallString" -Value "powershell.exe -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`""
Set-ItemProperty -Path $regPath -Name "URLInfoAbout" -Value "https://github.com/whisxdr/Wino"
Set-ItemProperty -Path $regPath -Name "NoModify" -Value 1 -Type DWord
Set-ItemProperty -Path $regPath -Name "NoRepair" -Value 1 -Type DWord

Write-Host "=========================================" -ForegroundColor Green
Write-Host "   [✓] Wino successfully installed!     " -ForegroundColor Green
Write-Host "   Location: $InstallDir\wino.exe        " -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
