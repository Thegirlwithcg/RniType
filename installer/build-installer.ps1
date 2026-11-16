# build-installer.ps1 — สร้าง installer (NSIS) + portable zip สำหรับทดสอบ
#
# สิ่งที่ต้องมี: Rust toolchain + Tauri CLI v2 (`cargo install tauri-cli --version "^2.0" --locked`)
# ครั้งแรก Tauri จะดาวน์โหลด NSIS ลง %LOCALAPPDATA%\tauri (ต้องต่อเน็ต)
#
# ใช้: .\installer\build-installer.ps1
# ผลลัพธ์:
#   dist\RniType_0.1.0_x64-setup.exe     ← installer (current user, ไม่ต้อง admin)
#   dist\RniType-portable-0.1.0.zip      ← portable exe + วิธีใช้
#
# build ด้วย feature `console` (beta) เพื่อให้เห็น log ตอนทดสอบ
# ถ้าต้องการซ่อน console: แก้ --features console ออก

$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$crate = Join-Path $root 'crates\rnitype-app'
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force $dist | Out-Null

Write-Host '[installer] cargo tauri build --features console --bundles nsis' -ForegroundColor Cyan
Push-Location $crate
try {
    cargo tauri build --features console --bundles nsis
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
$zip = Join-Path $dist 'RniType-portable-0.1.0.zip'
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $portableDir '*') -DestinationPath $zip -Force

# NSIS
$setup = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis') -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
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
