---
title: ADR-24 — Target guard, scancode path และ timer resolution (P3 slice 2)
status: accepted
updated: 2026-11-16
tags: [decision, win, safety, ime, performance]
---

# ADR-24 — P3 slice 2: exclusion/UIPI guard · scancode path · 1 ms timer

## บริบท

P3 slice 1 ส่งคีย์ได้ครบ (Unicode/VK/mouse) แต่ยังขาด 3 อย่างที่ HANDOFF ค้างไว้:

1. **exclusion list / UIPI** — เกมที่บล็อก SendInput และหน้าต่าง elevated ยังทำให้ผู้ใช้สงสัยว่าทำไมพิมพ์ไม่ได้
2. **scancode/IME path** — Unicode path ใช้กับ layout ไทยได้ แต่เกม/IME ที่อ่าน scan code ไม่เห็น
3. **`timeBeginPeriod(1)`** — loop 250 Hz ใช้ `sleep(3ms)` แต่ Windows ปกติให้ ~15 ms

## การตัดสินใจ

### 1. Target guard (exclusion + elevated) — `rnitype-win/src/target.rs`

| ส่วน | ค่า |
| --- | --- |
| ตรวจ | foreground HWND → PID → exe basename + process token elevated |
| exclusion | เทียบ basename แบบไม่สนตัวพิมพ์ · รองรับ `*` และ `?` (เช่น `game*.exe`) |
| UIPI | target elevated + แอปเราไม่ elevated → หยุด + เตือน |
| cadence | ตรวจทุก **250 ms** (ไม่ query process token ทุก 4 ms) |
| ระหว่างถูกกัน | **ไม่เรียก `session.step`** — ปล่อยคีย์ที่ค้าง, ยกเลิก gesture, HUD เป็น `disabled` + เหตุผล |
| กลับมา | `set_mode(mode เดิม)` อีกครั้ง → ต้องปล่อย LS ก่อนพิมพ์ต่อ (กันคีย์ค้าง) |

- HUD: `status.text` = `แอปนี้อยู่ในรายการกัน` หรือ `ต้องรันแบบ admin`
- engine **ไม่เปลี่ยน mode** ตอนถูกกัน → กลับมาแล้วยังเป็น Mouse/Nav/Type เดิม
- `--no-scancode` ยังไม่เกี่ยวกับ guard; guard ทำงานทุกโหมด

### 2. Scancode path อัตโนมัติ — `plan::plan_scancode` + `SendInputInjector`

```
Key::Char(ch):
  HKL = layout ของ thread เจ้าของ foreground window
  VkKeyScanExW(ch, HKL) → VK + shift state
  MapVirtualKeyExW(VK, HKL) → scan code
  ถ้าได้ → KEYEVENTF_SCANCODE (เกม/IME เห็นเหมือนคีย์บอร์ดจริง)
  ถ้าไม่ได้ / ต้อง Ctrl/AltGr → Unicode fallback (พฤติกรรมเดิม)
```

| เรื่อง | ค่า |
| --- | --- |
| เปิด/ปิด | `SendInputInjector::with_scancode(bool)` — app เปิด auto เป็นค่าเริ่มต้น |
| Ctrl/Alt chord | ใช้ VK path เดิม (Ctrl+ASCII = VK) — ไม่แปลงเป็น scan |
| NamedKey | VK path เดิม |
| Shift | ถ้า chord ต้อง Shift (ตัวพิมพ์ใหญ่/สัญลักษณ์) plan จะครอบ Shift ให้ |
| HeldState | รู้จัก `Input::ScanCode` → `release_all` ปล่อยได้ |
| Debug | `--no-scancode` = บังคับ Unicode ทั้งหมด |

**ทำไม auto ต่อ char**: Thai 'ก' มีใน layout ไทยแต่ไม่มีใน EN → เมื่อ foreground เป็น EN
`VkKeyScanExW` คืน -1 จึง fallback Unicode; เมื่อ foreground เป็นไทยจะใช้ scan code
ทำให้เกม/IME ทำงานเหมือนคีย์บอร์ดจริงโดยไม่ต้องตั้งค่า

### 3. Timer resolution 1 ms — `rnitype-win/src/timer.rs`

- `TimerResolution::new(1)` เรียก `timeBeginPeriod(1)` ตอนเริ่ม thread runtime
- `Drop` เรียก `timeEndPeriod(1)` (RAII) — ไม่ทิ้งค่าทั้งระบบค้าง
- log `[timer] 1 ms resolution: เปิด/ไม่สำเร็จ` ตอนเริ่ม

### 4. แก้บั๊กพ่วง: unpause คืน mode เดิม

เดิม tray Pause เรียก `on_pad_lost()` แล้ว unpause ไม่เรียก `on_pad_found()` →
พิมพ์ต่อไม่ได้จนกว่าจอยจะ reconnect. เพิ่ม `Session::pause()` / `unpause()` จำ mode เดิม
(และไม่รีเซ็ต `disabled_remaining_ms` ถ้ากำลังอยู่ระหว่าง emergency)

## ผลที่ตามมา

- `cargo test --workspace` = **188 tests** (core 98 · win 53 · app 37)
- FFI ใหม่ทั้งหมดอยู่ใน `target.rs` / `sendinput.rs`; ส่วน policy test ได้ทุกเครื่อง
- ยังไม่ทำ: Settings UI สำหรับ exclusion list, Task Scheduler/"Run as admin", app:status event
  (P5/P6) — ตอนนี้แก้ `system.exclusion` ใน config ได้โดยตรง
