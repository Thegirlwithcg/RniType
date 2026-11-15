# rnitype-app runner (P4 slice 5 — runtime จริง)
#
# ทำไมต้องมี script นี้:
# path ของ vault มี apostrophe (Erin's Works) → Windows RC.EXE อ่าน path ของ icon ไม่ได้
# (RC2135) จึง copy icon ไป temp ที่ไม่มี apostrophe แล้วส่ง path ผ่าน TAURI_CONFIG
#
# ใช้ (ทั้งสองแบบเท่ากัน — script เติม `--` ให้เอง):
#   .\crates\rnitype-app\run.ps1 --seconds 30
#   .\crates\rnitype-app\run.ps1 -- --seconds 30        (แบบเดิม ก็ยังใช้ได้)
#   .\crates\rnitype-app\run.ps1 --fake-pad --seconds 20
#   .\crates\rnitype-app\run.ps1 --demo --settings --seconds 30
#
# หมายเหตุ: PowerShell กิน `--` ตัวแรกทิ้งไปเอง ถ้าไม่เติมคืน cargo จะอ่าน --seconds
# เป็น flag ของ cargo (error: unexpected argument '--seconds')
#
# ไม่มี flag = จอยจริง · --fake-pad = จอยปลอม (จะพิมพ์ลงหน้าต่างที่ focus)

$ErrorActionPreference = 'Stop'

# ตัด `--` ตัวแรก (ถ้ามี) แล้วเติมให้ cargo เองเสมอ
$appArgs = @($args)
if ($appArgs.Count -gt 0 -and $appArgs[0] -eq '--') {
    $appArgs = if ($appArgs.Count -gt 1) { $appArgs[1..($appArgs.Count - 1)] } else { @() }
}

$crateDir = $PSScriptRoot
$icon = Join-Path $env:TEMP 'rnitype-app.ico'
Copy-Item (Join-Path $crateDir 'icons\icon.ico') $icon -Force

$config = @{ bundle = @{ icon = @(($icon -replace '\\', '/')) } } | ConvertTo-Json -Depth 5 -Compress
$env:TAURI_CONFIG = $config

$repoRoot = Split-Path (Split-Path $crateDir -Parent) -Parent
Push-Location $repoRoot
try {
    cargo run -p rnitype-app -- @appArgs
}
finally {
    Pop-Location
}
