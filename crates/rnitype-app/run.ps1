# rnitype-app runner (P4 slice 2 — real Tauri overlay)
#
# ทำไมต้องมี script นี้:
# path ของ vault มี apostrophe (Erin's Works) → Windows RC.EXE อ่าน path ของ icon ไม่ได้
# (RC2135) จึง copy icon ไป temp ที่ไม่มี apostrophe แล้วส่ง path ผ่าน TAURI_CONFIG
#
# ใช้:
#   .\crates\rnitype-app\run.ps1 -- --demo --seconds 8
#   .\crates\rnitype-app\run.ps1 -- --seconds 30        (ไม่มี demo)
#
# หมายเหตุ: ไม่มี --demo = HUD ไม่ได้ state (รอ runtime จริงใน P4 slice 3)

$ErrorActionPreference = 'Stop'

$crateDir = $PSScriptRoot
$icon = Join-Path $env:TEMP 'rnitype-app.ico'
Copy-Item (Join-Path $crateDir 'icons\icon.ico') $icon -Force

$config = @{ bundle = @{ icon = @(($icon -replace '\\', '/')) } } | ConvertTo-Json -Depth 5 -Compress
$env:TAURI_CONFIG = $config

$repoRoot = Split-Path (Split-Path $crateDir -Parent) -Parent
Push-Location $repoRoot
try {
    cargo run -p rnitype-app @args
}
finally {
    Pop-Location
}
