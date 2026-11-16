---
title: ADR-31 — Installer beta: NSIS (current user) + portable zip
status: accepted
updated: 2026-11-16
tags: [decision, packaging, installer, release]
---

# ADR-31 — Installer beta สำหรับทดสอบ

## บริบท

ถึงจุดที่ owner ต้องทดสอบบนเครื่องจริงแบบ "ติดตั้งแล้วใช้" ไม่ใช่ `run.ps1` จาก repo
แต่ยังไม่มี CI/packaging · ต้องเลือกวิธีที่ทำได้ในเครื่องนี้และถอนได้สะอาด

## การตัดสินใจ

### 1. Tauri bundler → NSIS

```json
"bundle": {
  "active": true,
  "targets": ["nsis"],
  "windows": { "nsis": { "installMode": "currentUser", "languages": ["English"] } }
}
```

| เรื่อง | ค่า |
| --- | --- |
| installer | NSIS `.exe` · **current user** (ไม่ต้อง admin) |
| output | `target\release\bundle\nsis\RniType_0.1.0_x64-setup.exe` → copy ไป `dist\` |
| uninstall | ลงทะเบียนใน Apps & Features ตามปกติ |
| WebView2 | ใช้ bootstrapper ของ Tauri (Win11 มีอยู่แล้ว) |
| icon | `crates/rnitype-app/icons/icon.ico` (32×32) |
| binary | `mainBinaryName: "RniType"` → ติดตั้งเป็น `RniType.exe` |
| ที่ติดตั้ง | `%LOCALAPPDATA%\RniType\` (current user) |
| console | build ด้วย feature **`console`** → ไม่ตั้ง `windows_subsystem` เพื่อเห็น log |

### 2. Portable zip (fallback)

`dist\RniType-portable-0.1.0.zip` = `RniType.exe` + `README-beta.txt`
ใช้เมื่อ installer ติดปัญหา SmartScreen/สิทธิ์

### 3. Build script

`installer/build-installer.ps1`:

1. `cargo tauri build --features console --bundles nsis` (จาก `crates/rnitype-app`)
2. copy release exe → `dist\portable\RniType.exe` + README → zip
3. copy `*-setup.exe` → `dist\`

Tauri CLI ไม่ได้อยู่ใน repo — ติดตั้งครั้งเดียว: `cargo install tauri-cli --version "^2.0" --locked`
(ครั้งแรก bundler จะดาวน์โหลด NSIS เอง)

### 4. ข้อจำกัดที่ยอมรับใน beta

- **ไม่ code-sign** → SmartScreen เตือน Unknown publisher
- console window เปิดอยู่ (ตั้งใจ ให้เห็น log)
- ไม่มี auto-update
- ยังไม่อัปโหลด Steam/itch (ต้อง build ผ่าน Steamworks/itch butler)

## ผลที่ตามมา

- owner ทดสอบได้ด้วย `dist\RniType_0.1.0_x64-setup.exe` หรือ portable zip
- ขั้นตอน retest อยู่ใน HANDOFF §4.8 ข้อ 22
- **ตรวจแล้วบนเครื่องนี้ (2026-11-16)**:
  - `setup.exe /S` → `%LOCALAPPDATA%\RniType\RniType.exe` + `uninstall.exe`
  - Start Menu: `RniType.lnk` · Apps & Features: `RniType 0.1.0` + `UninstallString`
  - รัน exe ที่ติดตั้ง: overlay/tray/timer/HUD handshake ผ่าน · `--settings` เปิดหน้าต่างได้
  - `uninstall.exe /S` → ลบโฟลเดอร์ + shortcut + registry หมด (ไม่มี process ค้าง)
- warning ที่เจอ: identifier `dev.rnitype.app` ลงท้าย `.app` (Tauri เตือนเรื่อง macOS) — Windows-only จึงรับไว้
- งานถัดไป: code-sign, auto-update, upload ร้าน
