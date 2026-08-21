<#
.SYNOPSIS
    Wino Native Windows Uninstaller Script
.DESCRIPTION
    Cleanly removes Wino application files, shortcuts, and registry entries.
#>

$ErrorActionPreference = "SilentlyContinue"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "       Wino Windows Uninstaller          " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# 1. Terminate running instances
Get-Process -Name "wino" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 2. Remove Shortcuts
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$startMenuFolder = if ($isAdmin) { "$env:ProgramData\Microsoft\Windows\Start Menu\Programs\Wino" } else { "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Wino" }
if (Test-Path $startMenuFolder) {
    Remove-Item -Path $startMenuFolder -Recurse -Force
}

$desktopShortcut = "$([Environment]::GetFolderPath('Desktop'))\Wino.lnk"
if (Test-Path $desktopShortcut) {
    Remove-Item -Path $desktopShortcut -Force
}

# 3. Remove Registry Uninstaller Entries
$regHKLM = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wino"
$regHKCU = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Wino"
if (Test-Path $regHKLM) { Remove-Item -Path $regHKLM -Recurse -Force }
if (Test-Path $regHKCU) { Remove-Item -Path $regHKCU -Recurse -Force }

# 4. Remove Install Folder (scheduled via background cmd to remove self if needed)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Write-Host "[+] Cleaning up application files in: $scriptDir" -ForegroundColor Gray

Start-Process -FilePath "cmd.exe" -ArgumentList "/c timeout /t 1 >nul & rd /s /q `"$scriptDir`"" -WindowStyle Hidden

Write-Host "=========================================" -ForegroundColor Green
Write-Host "   [✓] Wino has been uninstalled.        " -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
