$ErrorActionPreference = "Stop"
$mingw = (Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs*" -Recurse -Filter "gcc.exe" -ErrorAction SilentlyContinue | Select-Object -First 1).DirectoryName
if ($mingw) {
    $env:Path = "$mingw;C:\Program Files\Rust stable GNU 1.98\bin;" + $env:Path
}
Write-Host "Compiling capture_screenshots..." -ForegroundColor Cyan
cargo run --bin capture_screenshots

Write-Host "Converting BMP screenshots to PNG..." -ForegroundColor Cyan
Add-Type -AssemblyName System.Drawing
$bmpFiles = Get-ChildItem -Path "docs\screenshots" -Filter "*.bmp"
foreach ($bmp in $bmpFiles) {
    $pngPath = [System.IO.Path]::ChangeExtension($bmp.FullName, ".png")
    $img = [System.Drawing.Bitmap]::FromFile($bmp.FullName)
    $img.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $img.Dispose()
    Remove-Item $bmp.FullName -Force
    Write-Host "Saved: $pngPath" -ForegroundColor Green
}
Write-Host "All screenshots captured and converted to PNG successfully!" -ForegroundColor Green
