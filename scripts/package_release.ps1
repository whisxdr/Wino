<#
.SYNOPSIS
    Packages Wino for GitHub Releases
.DESCRIPTION
    Builds the release binary, creates the portable ZIP package,
    bundles installer files, and generates SHA256 checksums.
#>

param(
    [string]$Version = "2.6.0"
)

$ErrorActionPreference = "Stop"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "     Packaging Wino v$Version Release    " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$baseDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $baseDir

# 1. Verify Compiler and Build
$mingw = (Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs*" -Recurse -Filter "gcc.exe" -ErrorAction SilentlyContinue | Select-Object -First 1).DirectoryName
if ($mingw) {
    $env:Path = "$mingw;C:\Program Files\Rust stable GNU 1.98\bin;" + $env:Path
}

Write-Host "[1/4] Compiling optimized release binary..." -ForegroundColor Yellow
Get-Process -Name "wino" -ErrorAction SilentlyContinue | Stop-Process -Force
cargo build --release

# 2. Prepare dist directory
Write-Host "[2/4] Assembling release artifacts..." -ForegroundColor Yellow
$distDir = "$baseDir\dist"
$pkgDir = "$distDir\Wino"

if (Test-Path $distDir) { Remove-Item $distDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null

Copy-Item "$baseDir\target\release\wino.exe" -Destination "$pkgDir\wino.exe"
Copy-Item "$baseDir\README.md" -Destination "$pkgDir\README.md"
Copy-Item "$baseDir\LICENSE" -Destination "$pkgDir\LICENSE"
Copy-Item "$baseDir\ARCHITECTURE.md" -Destination "$pkgDir\ARCHITECTURE.md"
Copy-Item "$baseDir\installer\install.ps1" -Destination "$pkgDir\install.ps1"
Copy-Item "$baseDir\installer\uninstall.ps1" -Destination "$pkgDir\uninstall.ps1"

# 3. Create Portable ZIP
Write-Host "[3/4] Creating Portable ZIP archive..." -ForegroundColor Yellow
$zipPath = "$distDir\Wino-v$Version-windows-x64-portable.zip"
Compress-Archive -Path "$pkgDir\*" -DestinationPath $zipPath -Force

# 4. Generate SHA256 Checksums
Write-Host "[4/4] Generating SHA256 Checksums..." -ForegroundColor Yellow
$checksumFile = "$distDir\checksums.sha256"

$filesToHash = Get-ChildItem -Path $distDir -Filter "*.zip"
foreach ($f in $filesToHash) {
    $hash = (Get-FileHash -Path $f.FullName -Algorithm SHA256).Hash
    "$hash  $($f.Name)" | Out-File -FilePath $checksumFile -Append -Encoding utf8
}

Write-Host "=========================================" -ForegroundColor Green
Write-Host "   Release Packaged Successfully!        " -ForegroundColor Green
Write-Host "   Artifacts in: $distDir                " -ForegroundColor Green
Get-ChildItem -Path $distDir | Select-Object Name, Length
Write-Host "=========================================" -ForegroundColor Green
