# Spike B runner
#
# ทำไมต้องมี script นี้:
# path ของ vault มี apostrophe (Erin's Works) → Windows RC.EXE อ่าน path ของ icon ไม่ได้
# (RC2135) จึง copy icon ไป temp ที่ไม่มี apostrophe แล้วส่ง path ผ่าน TAURI_CONFIG

$ErrorActionPreference = 'Stop'
$icon = Join-Path $env:TEMP 'rnitype-spike-overlay.ico'
Copy-Item (Join-Path $PSScriptRoot 'src-tauri\icons\icon.ico') $icon -Force

$config = @{ bundle = @{ icon = @(($icon -replace '\\', '/')) } } | ConvertTo-Json -Depth 5 -Compress
$env:TAURI_CONFIG = $config

cargo run --manifest-path (Join-Path $PSScriptRoot 'src-tauri\Cargo.toml') @args
