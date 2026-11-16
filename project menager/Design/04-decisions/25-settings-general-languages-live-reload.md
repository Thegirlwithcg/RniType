---
title: ADR-25 — Settings General/Languages + exclusion UI + live config reload (P5 slice 2)
status: accepted
updated: 2026-11-16
tags: [decision, settings, i18n, config, win]
---

# ADR-25 — P5 slice 2: General · Languages · exclusion list · hot reload

## บริบท

P5 slice 1 เปิดหน้าต่าง Settings ได้ แต่มีแค่แท็บ HUD (ขนาด/ฟอนต์) และ config ที่เปลี่ยน
ไม่มีผลกับ runtime จนกว่าจะรีสตาร์ต (ยกเว้น typography ที่ส่งผ่าน `hud:style`)

งานที่ค้างตาม HANDOFF v0.8:

1. General (สถานะจอย/pause) + exclusion list UI — ก่อนหน้านี้ต้องแก้ JSON เอง
2. Languages — ตรวจ `GetKeyboardLayoutList`, เปิด/ปิดภาษา, badge IME
3. Keymap editor — ต้องมี universal QWERTY mapping ก่อน (แยกไป slice ถัดไป)

## การตัดสินใจ

### 1. General tab

| ส่วน | พฤติกรรม |
| --- | --- |
| Status | `get_status` + event `app:status` → Controller / Typing / Version |
| Pause | `set_paused { paused }` = เมนู tray (จอยยังอ่าน แต่ไม่ส่งคีย์) |
| Exclusion | อ่าน/เขียน `system.exclusion` ผ่าน `save_config` · เพิ่ม/ลบในหน้าเดียว |
| Reset | `reset_config { section: "general" }` — รีเซ็ต `defaults.*` + autostart/runAsAdmin แต่ **ไม่ล้าง exclusion list** |

### 2. Languages tab

- `rnitype-win/src/language.rs` ใหม่:
  - `GetKeyboardLayoutList` → HKL → `GetKeyboardLayoutNameW` (KLID) + `GetLocaleInfoW(ENGLISHDISPLAYNAME)`
  - `code` จาก language id (en/th/ru/el/ja/ko/zh, อื่น ๆ `0xNNNN`)
  - `kind`: JA/ZH/KO = `ime` · อื่น = `layout` (ADR-14 §1)
- **EN/TH แสดงเสมอ** แม้ Windows ไม่ได้ติดตั้ง layout — เพราะตาราง 8×8 อยู่ในแอป
  (`builtin: true`); layout ที่ติดตั้งได้ badge `Detected`
- **EN = base**: toggle ถูกล็อก (`locked: true`) ปิดไม่ได้
- ภาษาอื่นที่ยังไม่มีตาราง: toggle ปิด + badge `Needs mapping` → เปิดได้เมื่อ universal mapping มา
- เปิดได้สูงสุด **8** ภาษา (`MAX_ENABLED_LANGUAGES`) — เกิน = `CONFIG_INVALID`
- `rescan_languages` คืนแถวที่ merge กับ config แล้ว (detected/builtin/enabled/supported/locked)

### 3. Live config reload

เพิ่ม `AppState.config_rev: AtomicU64` — ทุก command ที่เขียน config เรียก `touch_config()`

Runtime loop (ทุก 250 ms, จังหวะเดียวกับ guard):

```
rev != last_rev → clone config
                → session.set_enabled_languages(enabled_languages(&config))
                → apply_pivot_clamp(session, &config, virtual_desktop())
                → log [config] runtime reload: languages=N hudScale=X
```

- typography ยังใช้ `hud:style` (เดิม) และ `hud:state.scale` (เดิม)
- ไม่ต้องรีสตาร์ตแอปเมื่อเพิ่ม/ลดภาษา หรือเปลี่ยนขนาดวง

### 4. `enabled_languages` (runtime)

| config | ผล |
| --- | --- |
| ไม่มี `languages` / `[]` | EN + TH (ค่าเริ่มต้น) |
| มีรายการ | EN เสมอ + TH เมื่อ `enabled: true` |
| ปิด TH | มีแค่ EN (ไม่มี language slot) |

เดิม logic fallback เปิด TH กลับเมื่อไม่มีรายการที่ enabled — แก้เป็น "ใช้รายการที่มี" ตามด้านบน

### 5. UI

- Sidebar เป็นปุ่มจริง (`data-tab`) สลับ pane — Keymap แสดง disabled พร้อมเหตุผล
- General: status grid 3 ช่อง + switch + exclusion list + Reset (general/all)
- Languages: แถวรหัส/ชื่อ/badge + switch, `Rescan languages`, `Reset languages`
- ทุกปุ่มเรียก command จริง; preview (ไม่มี Tauri) ใช้ข้อมูลตัวอย่างและไม่เขียนไฟล์

## ผลที่ตามมา

- `cargo test --workspace` = **201 tests** (core 98 · win 56 · app 47)
- `node tools/tests/settings-smoke.mjs` = **19/19** + screenshot
  `settings-general.png` / `settings-languages.png`
- E2E กับ WebView จริง (CDP 9224): ปิด TH → config `[{th,false}]` → log
  `[config] runtime reload: languages=0`; เปิดกลับ → `languages=1`
  - เพิ่ม exclusion ผ่านฟอร์ม → config มี `live-test.exe`; ลบ → `[]`
- ยังไม่ทำ: Keymap editor (ต้อง universal QWERTY mapping) · autostart/runAsAdmin toggle
  ที่เรียก Task Scheduler จริง (P6) · `open_settings { tab }`
