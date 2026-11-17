# build-installer.ps1 — สร้าง installer (NSIS) + portable zip สำหรับทดสอบ
#
# สิ่งที่ต้องมี: Rust toolchain + Tauri CLI v2 (`cargo install tauri-cli --version "^2.0" --locked`)
# ครั้งแรก Tauri จะดาวน์โหลด NSIS ลง %LOCALAPPDATA%\tauri (ต้องต่อเน็ต)
#
# ใช้: .\installer\build-installer.ps1
# ผลลัพธ์:
#   dist\RniType_0.3.9_x64-setup.exe     ← installer (current user, ไม่ต้อง admin)
#   dist\RniType-portable-0.3.9.zip      ← portable exe + วิธีใช้
#
# build แบบ release ปกติ (ไม่มี feature `console`) → เปิดแอปแล้วไม่มีหน้าต่าง terminal
# ถ้าต้อง debug ด้วย console: เติม `--features console` เองตอน build

$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$crate = Join-Path $root 'crates\rnitype-app'
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force $dist | Out-Null

Write-Host '[installer] cargo tauri build --bundles nsis' -ForegroundColor Cyan
Push-Location $crate
try {
    cargo tauri build --bundles nsis
}
finally {
    Pop-Location
}

$releaseExe = Join-Path $root 'target\release\RniType.exe'
if (-not (Test-Path $releaseExe)) {
    $releaseExe = Join-Path $root 'target\release\rnitype-app.exe'
}
if (-not (Test-Path $releaseExe)) {
    throw "ไม่พบ release exe — build ไม่สำเร็จ"
}

# portable
$portableDir = Join-Path $dist 'portable'
New-Item -ItemType Directory -Force $portableDir | Out-Null
Copy-Item $releaseExe (Join-Path $portableDir 'RniType.exe') -Force
Copy-Item (Join-Path $PSScriptRoot 'README-beta.txt') $portableDir -Force
$version = (Get-Content (Join-Path $crate 'tauri.conf.json') -Raw | ConvertFrom-Json).version
$zip = Join-Path $dist "RniType-portable-$version.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $portableDir '*') -DestinationPath $zip -Force

# NSIS
$setup = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis') -Filter "RniType_${version}_*-setup.exe" -ErrorAction SilentlyContinue |
    Select-Object -First 1
if ($setup) {
    Copy-Item $setup.FullName $dist -Force
    Write-Host "[installer] พร้อม: $($setup.FullName)" -ForegroundColor Green
}
else {
    Write-Warning 'ไม่พบ NSIS setup — Tauri bundler อาจดาวน์โหลด NSIS ไม่สำเร็จ (ดู log ด้านบน)'
}

Write-Host "[installer] ผลลัพธ์อยู่ใน $dist" -ForegroundColor Green
Get-ChildItem $dist | Select-Object Name, Length, LastWriteTime
