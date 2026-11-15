---
title: สถาปัตยกรรมทางเทคนิค
status: draft v0.1
updated: 2026-11-15
tags: [architecture, windows, rust, tauri]
---

# 03.1 — สถาปัตยกรรมทางเทคนิค

## 1. เลือกเทคโนโลยี

> **อัปเดต 2026-11-15**: เปลี่ยนภาษาหลักจาก C# เป็น **Rust + Tauri v2** ตาม [[03-stack-rust]]
> (โครงสร้างโมดูล/สัญญาระหว่างชั้นยังเหมือนเดิมทุกข้อ)

| ตัวเลือก | ข้อดี | ข้อเสีย | สรุป |
| --- | --- | --- | --- |
| **Rust + Tauri v2 + SDL3/gilrs** | เบา ไม่มี runtime, latency ต่ำ, FFI ตรง, ไฟล์เล็ก | UI ต้องผ่าน WebView2, จอย DS5 ต้องทดสอบ | **เลือก** |
| C# .NET 8 + WPF/WinUI 3 + SDL3 | tray/IME/overlay ครบ, พัฒนาเร็ว | runtime/ขนาดใหญ่กว่า, GC jitter เล็กน้อย | แผนสำรอง (ถอยได้ทุกเมื่อ) |
| Electron + node-gamepad | UI สวยเร็ว | latency, native addon เปราะ, ไฟล์ใหญ่ | ไม่เลือก |
| C++ + Win32 | คุมสุด | พัฒนาช้า, UI ต้องทำเอง | ไม่เลือก |
| Godot 4 | UI สวยแบบเกม, cross-platform | ไม่มี tray/IME/autostart, ต้อง interop Win32 เอง, เริ่มช้า, ไฟล์ใหญ่ | ไม่เลือก (ดู [[02-engine-choice-godot]]) |

**ทำไม gilrs ก่อน, SDL3 เป็นตัวสำรอง**: `gilrs` เบาและใช้ Windows Gaming Input (WGI) บน Windows
→ ได้ hotplug + rumble ฟรี; ถ้า DS5 (ไจโร/adaptive trigger) ไม่ครบ ให้เปลี่ยนชั้นนีเป็น `sdl3`
โดยไม่กระทบชั้นอื่น (สัญญา: ส่งแกน/ปุ่มที่ normalize แล้วเท่านั้น)

## 2. โมดูล

```
┌────────────────┐   ┌──────────────┐   ┌────────────────┐
│ Device Layer   │→  │ Normalizer   │→  │ Gesture Engine │
│ (gilrs/SDL3)   │   │ deadzone/    │   │ (state machine)│
│ detect, axes   │   │ sector/hyst. │   │ 02.4           │
└────────────────┘   └──────────────┘   └───────┬────────┘
                                                │ Action
                    ┌──────────────┐   ┌────────▼────────┐
                    │ HUD Overlay  │←  │ Action Mapper   │
                    │ (Tauri, click│   │ keymap EN/TH    │
                    │  -through)   │   │ profiles        │
                    └──────────────┘   └────────┬────────┘
                                                │ Key/Mouse cmd
                                       ┌────────▼────────┐
                                       │ Output Injector │
                                       │ SendInput       │
                                       └─────────────────┘
   Config/Profiles (JSON) ─── Tray & UI (Tauri v2) ─── Log (tracing)
```

**Thread model**

| เธรด | งาน | รอบ |
| --- | --- | --- |
| Input | poll จอย (gilrs/SDL3), normalize, tick state machine | 250 Hz (4 ms) |
| Injector | คิวคำสั่ง → SendInput | ตาม event |
| UI/HUD | วาด overlay, อัปเดตเมื่อ state เปลี่ยน | 60 fps |
| Config | โหลด/บันทึก JSON, hot reload | on demand |

## 3.1 ภาษาและ mapping (v0.11)

- **ตรวจ layout ที่ติดตั้ง**: `GetKeyboardLayoutList` + `GetLocaleInfo(LOCALE_SLOCALIZEDDISPLAYNAME)`
  → แสดงใน Settings → Languages; เก็บเฉพาะที่ผู้ใช้เปิด (`enabled`)
- **จำแนกชนิด**: layout ที่มี IME (JA/ZH/KO) → `kind: ime`; ที่เหลือ → `layout`
- **สร้าง 8×8 เริ่มต้น** (`layout`): ใช้ `ToUnicodeEx` กับ HKL ของภาษานั้นทีละ VK (shift states)
  → ได้ตัวอักษรตาม **ตำแหน่ง QWERTY**; cache ไว้ใน config
- **ไทย/อังกฤษ**: ใช้ตารางที่ออกแบบไว้ ([[02-keymap-en-th]]) เป็นค่าเริ่มต้น
- **override**: `config.languages[].overrides` เก็บเฉพาะช่องที่ผู้ใช้แก้ (index กลุ่ม-ช่อง)
- **IME**: ใช้ **scancode path** เท่านั้น (ไม่ใช้ Unicode) เพื่อให้ IME ของ Windows ประกอบข้อความ;
  HUD แสดง QWERTY + badge `IME`; ไม่มี override ตัวอักษร
- **Settings window**: หน้าต่าง Tauri แยก (ไม่ใช่ overlay) — General/Languages/Keymap/HUD + Reset
  (`reset` = ล้าง overrides + ค่า default ของ section/ทั้งหมด)

## 3. การส่งคีย์ (สำคัญที่สุดของแอปนี้)

**สองเส้นทาง เลือกอัตโนมัติ**

1. **Scancode/VK path** (ค่าเริ่มต้น) — แปลงตัวอักษร → `(VK, shift)` ด้วย `VkKeyScanEx`
   ของ layout ที่ active; ส่ง `KEYEVENTF_SCANCODE` → พฤติกรรมเหมือนพิมพ์จริง
   (เกม/แอปที่อ่าน scan code เห็น, IME ทำงานปกติ)
2. **Unicode path** (fallback) — `KEYEVENTF_UNICODE` ส่งอักขระตรงๆ
   ใช้เมื่อ layout ปัจจุบันพิมพ์ตัวนั้นไม่ได้ (เช่น พิมพ์ไทยขณะ layout เป็น EN)

**ข้อควรระวัง**

| เรื่อง | วิธีจัดการ |
| --- | --- |
| UIPI (หน้าต่าง elevated) | มี option "Run as administrator" (Task Scheduler) + เตือนผู้ใช้ |
| Modifier ค้าง | ปล่อย modifier ด้วย RAII guard (Drop) และเมื่อจอยหลุด |
| Auto-repeat ของ OS | ตั้ง `repeat_delay` ฝั่งเราเอง, อย่าส่ง `KEYEVENTF_DOWN` ซ้ำโดยไม่ปล่อย |
| Thai IME | ถ้า IME เปิดอยู่ การส่ง scan code อาจถูก IME แปลง — ตรวจ `ImmGetContext` และใช้ Unicode path |
| เกมที่บล็อก | exclusion list ต่อโปรเซส (`GetForegroundWindow` → ชื่อ exe) |
| Antivirus | ไม่ใช้ global keyboard hook (อ่านจากจอยเท่านั้น) → ลด false positive มาก |

**Mouse**: `SendInput` แบบ `MOUSEEVENTF_MOVE` (relative) ใน Mouse mode
และ `MOUSEEVENTF_MOVE | ABSOLUTE` เมื่อ recenter/ย้ายข้ามจอ (normalize 0..65535 บน virtual desktop)

## 4. Config (JSON, per-app profile)

```jsonc
{
  "version": 1,
  "defaults": {
    "typing": { "mode": "th", "commitDwellMs": 60, "hud": true },
    "mouse":  { "vmax": 1600, "gamma": 1.7, "deadzone": 0.15 },
    "hotkeys": { "emergency": "LB+RB", "hudToggle": "LT+RT" }
  },
  "profiles": [
    { "match": { "exe": "Code.exe" },   "mode": "en", "quickbar": "code" },
    { "match": { "exe": "WINWORD.EXE" }, "mode": "th", "hud": false }
  ],
  "keymaps": {
    "en": { "layers": [ /* 8 กลุ่ม × 8 ทิศ */ ] },
    "th": { "layers": [ /* ... */ ] }
  }
}
```

- เก็บที่ `%APPDATA%\RniType\config.json` + `keymaps/*.json`
- validate ด้วย JSON Schema, backup ก่อนเขียน, hot reload
- export/import profile ได้ (แชร์ให้คนอื่น)

## 5. หน้าตาแอป (อ้างอิง Logi Options+)

| หน้า | เนื้อหา |
| --- | --- |
| Home | รูปจอยที่ต่ออยู่ (ใช้ภาพใน `Design/2D`), แบตเตอรี่, mode ปัจจุบัน, ปุ่ม "ทดสอบ" |
| Typing | 8×8 grid แก้ได้, เลือกภาษา, ปรับ dwell/timeout, ปุ่มดูตัวอย่าง HUD |
| Mouse | สไลเดอร์ vmax/gamma/deadzone + กราฟทดสอบการเคลื่อนที่ |
| Shortcuts | Quick bar 8 ช่อง + ปุ่มหน้า |
| App Profiles | เพิ่ม exe, ตั้ง mode ต่อแอป |
| System | autostart, run as admin, HUD, ภาษา UI, export/import, log |

**System integration**: tray icon (สีเปลี่ยนตาม mode — Tauri v2 tray), single instance
(`tauri-plugin-single-instance`), autostart ผ่าน Task Scheduler (ได้สิทธิ์สูงโดยไม่ต้อง UAC ทุกครั้ง),
ตั้งชื่อ process ให้อ่านง่าย; overlay: Tauri `set_ignore_cursor_events(true)` + `always_on_top`
แล้วเสริม Win32 `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` เพื่อไม่แย่ง focus (มี fallback egui)

## 6. Performance budget

| จุด | เป้า |
| --- | --- |
| poll → state machine tick | < 1 ms |
| tick → SendInput | < 1 ms |
| HUD repaint | < 8 ms |
| CPU idle (ไม่มี input) | < 0.5% |
| RAM | core < 20 MB; รวม WebView2 (หน้าตั้งค่าเปิด) < 150 MB |
| แบตจอย | ปิด rumble เป็นค่าเริ่มต้น |
| ขนาดตัวติดตั้ง | < 15 MB (ไม่รวม WebView2 runtime) |
