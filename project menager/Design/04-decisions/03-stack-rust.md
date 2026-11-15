---
title: ADR-03 — ใช้ Rust เป็นภาษาหลัก
status: accepted (ข้อเสนอ)
updated: 2026-11-15
supersedes: ADR-01 (สแตก C#) บางส่วน
tags: [decision, rust, stack]
---

# ADR-03 — เปลี่ยนแกนเป็น Rust ได้ไหม

## บริบท

ADR-01/[[01-architecture]] เลือก C# .NET 8 ไว้ก่อนหน้า ผู้ใช้ถามว่าขอใช้ **Rust เป็นหลัก** ได้หรือไม่
โดยยังขายบน Steam / itch และยังต้องทำงานเป็น background utility บน Windows

## คำตอบ

**ได้ — และ Rust เหมาะกับ "แกน" ของแอปนี้มากกว่าด้วยซ้ำ** (ไม่มี runtime, ไม่มี GC,
คุม latency และขนาดไฟล์ได้, FFI กับ Win32 ตรง)
แต่ต้องยอมรับว่า **UI คือจุดที่ Rust อ่อนกว่า C#** จึงต้องเลือกวิธีทำ UI ให้ชัด

| ส่วน | ภาษา/เทคโนโลยี | เหตุผล |
| --- | --- | --- |
| Input core, state machine, keymap, injector | **Rust** | latency ต่ำ, FFI ตรง, binary เดียว |
| หน้าตั้งค่า + tray | **Tauri v2** (WebView2) | UI สวยเร็ว, tray/autostart/updater มี plugin |
| HUD overlay | Tauri window โปร่งใส (มี fallback egui) | ต้องทดสอบ click-through/DPI ก่อน |
| ตัวช่วย/dev tool | PowerShell ตามเดิม | ใช้ทดสอบบน Windows |

## เทียบกับงานจริง (เปลี่ยนจากตารางใน ADR-01)

| งาน | Rust | C# .NET 8 |
| --- | --- | --- |
| `SendInput` / `VkKeyScanEx` | `windows` crate (unsafe แต่ตรง) | เรียกได้เลย |
| อ่านจอย Xbox | `gilrs` — บนเครื่อง owner WGI ไม่ส่ง event จึงใช้ **XInput backend** ([[15-gilrs-xinput-backend]]); `sdl3` เป็นทางถอย | SDL3 ตรง |
| อ่านจอย DualSense | **ต้องทดสอบ** — ถ้าต้องการ gyro/adaptive trigger ต้องใช้ `hidapi` เพิ่ม | SDL3 จัดการให้ |
| Tray | Tauri v2 tray (built-in) | มีในตัว |
| Autostart / single instance / updater | Tauri plugin ครบ | ทำเองบางส่วน |
| Overlay คลิกทะลุ | Tauri `set_ignore_cursor_events` + Win32 flags (มี quirk) | WPF ตรงไปตรงมา |
| IME ไทยในช่องข้อความ | WebView2 จัดการ (ปกติใช้ได้) | native TextBox |
| ขนาดติดตั้ง | ~5–15 MB (ไม่รวม WebView2) | ~40–70 MB self-contained |
| RAM | ~80–150 MB (WebView2) หรือ < 20 MB ถ้า egui | ~100 MB |
| เวลาเปิด | < 500 ms (core), UI ตาม WebView | < 1 s |
| Dev speed (UI) | ช้ากว่า (ต้องมี HTML/CSS หรือ egui) | เร็วสุด |
| ความปลอดภัยหน่วยความจำ | ดีกว่า (ไม่มี UB จาก GC/pointer) | ดี |

## สแตกที่แนะนำ (crates)

| งาน | crate |
| --- | --- |
| Win32 API | `windows` (Input::KeyboardAndMouse, UI::WindowsAndMessaging, Graphics::Gdi) |
| จอย | `gilrs` (เริ่มต้น) — ถ้า DS5 ไม่ครบ เปลี่ยนเป็น `sdl3` หรือ `hidapi` |
| UI + tray | `tauri` 2 + `tauri-plugin-autostart`, `-single-instance`, `-updater`, `-global-shortcut` |
| Overlay เบา (fallback) | `eframe`/`egui` (โปร่งใส, always-on-top) |
| Config | `serde`, `serde_json`, `schemars` (สร้าง JSON Schema) |
| Log | `tracing`, `tracing-appender` |
| Test | `insta` (snapshot ของ keymap), `criterion` (latency) |
| Time/poll | `std::thread` + `Instant` (ไม่ต้องใช้ async runtime) |

## สถาปัตยกรรมที่เปลี่ยน

โครงเดิมใน [[01-architecture]] ใช้ได้ทั้งหมด — เปลี่ยนเฉพาะตัวภาษา:

```
Input thread (250 Hz)  →  crossbeam channel  →  Injector thread
        │                        │
        └── state snapshot ──────┴──► Tauri event (HUD)
```

- **ไม่ใช้ tokio** ในเส้นทาง input — async runtime เพิ่ม jitter โดยไม่จำเป็น
- ใช้ channel แบบ bounded เพื่อไม่ให้ HUD ทำให้ input ช้า
- `SendInput` เรียกจาก thread เดียวเสมอ (กันลำดับคีย์สลับ)

## ความเสี่ยงและวิธีลด

| ความเสี่ยง | ระดับ | วิธีลด |
| --- | --- | --- |
| DS5 บน Windows ผ่าน `gilrs` ไม่ครบ (ปุ่ม/ทริกเกอร์/ไจโร) | สูง | spike 1 วัน: ทดสอบปุ่มครบ 8 + trigger + deadzone; ถ้าไม่พอ → `sdl3` |
| Overlay โปร่งใสของ Tauri มี quirk (focus, DPI, z-order) | กลาง | ทดสอบ 3 จอ + scaling 125/150%; fallback egui |
| WebView2 ไม่มีในเครื่อง (Windows N/LTSC เก่า) | ต่ำ | bundle Evergreen bootstrapper ใน installer |
| พัฒนา UI ช้ากว่า C# | กลาง | ใช้ HTML template + ทำหน้าตั้งค่าเป็นหน้าเดียว (ไม่ต้องมี design system ใหญ่) |
| `unsafe` ใน Win32 FFI | กลาง | ห่อเป็น module เดียว, unit test ทุก unsafe block, `#![deny(unsafe_op_in_unsafe_fn)]` |
| คนอื่นร่วมพัฒนาอ่าน Rust ยากกว่า | ต่ำ | เอกสาร ADR + comment ภาษาไทย/อังกฤษ |

## เกณฑ์ "ไปต่อหรือถอย" (ตัดสินใน 3 วันแรก)

1. อ่านจอย Xbox + DS5 ได้ครบทุกปุ่ม/แกน ผ่าน `gilrs` (ไม่ต้องใช้ driver พิเศษ)
2. overlay โปร่งใส คลิกทะลุ ไม่แย่ง focus ได้บน 2 จอ + scaling 150%
3. `SendInput` พิมพ์ไทย (Unicode path) ลง Notepad ได้ถูกต้อง

ถ้าข้อ 1 ไม่ผ่าน → เปลี่ยนชั้นจอยเป็น `sdl3` (ยังใช้ Rust ต่อ)
ถ้าข้อ 2 ไม่ผ่านซ้ำๆ → ใช้ egui เป็น overlay (ยังใช้ Rust ต่อ)
ถ้าทั้งคู่ไม่ผ่านและต้องส่งงานเร็ว → ถอยไป C# ตาม ADR-01 (เอกสารออกแบบใช้ร่วมกันได้ทั้งหมด)

## ผลที่ตามมา

- อัปเดต [[01-architecture]]: ภาษาเป้าหมายเป็น Rust, diagram และ performance budget ปรับตาม
- แผน M0–M5 ใน [[02-roadmap-and-smoke-tests]] **ไม่เปลี่ยน** — เพิ่มงาน spike 2–3 วันก่อน M0
- smoke script ยังเป็น PowerShell เหมือนเดิม (ทดสอบจากภายนอก ไม่ผูกภาษา)
- หน้าตั้งค่าใช้ HTML/CSS ภายใน Tauri → mockup ใน `Design/2D` ยังใช้ได้
