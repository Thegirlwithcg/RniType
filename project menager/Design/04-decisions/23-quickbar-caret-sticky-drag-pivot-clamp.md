---
title: ADR-23 — Quick bar, caret nav, sticky drag และ pivot clamp
status: accepted
updated: 2026-11-16
tags: [decision, core, mouse, quickbar, caret, hud]
---

# ADR-23 — P2 slice 4: quick bar · caret nav · sticky drag · pivot clamp

## บริบท

P2 slice 3 ปิด mouse/Nav/repeat/layer 2 แล้ว เหลือ 4 อย่างที่ค้างใน HANDOFF §4.3:

1. quick bar (LB ค้าง) — interaction model §5 กำหนดช่องครบแล้ว แต่ยังไม่มี core/HUD
2. caret nav (R3) — ADR-05 กำหนดพฤติกรรมแล้ว ยังไม่ย้ายลง core
3. sticky drag (RB 2 แตะ) — ยังไม่มีที่ไหนในโค้ด
4. pivot clamp — HUD clamp ฝั่ง JS อยู่ (`MARGIN 172`) แต่แอปไม่ clamp → state จริงล้นจอได้

## การตัดสินใจ

### 1. Quick bar = `WheelKind::QuickBar` (ไม่เพิ่ม state machine ใหม่)

| เรื่อง | ค่า |
| --- | --- |
| เปิด | **กด LB ค้าง 150 ms** (`quickbar_hold_ms`) ขณะ Type mode · gesture ว่าง · LS < 0.35 — แตะสั้นไม่เปิด |
| เลือก | RS 8 ทิศ (เหมือนวงคำสั่ง) — arm หลัง RS กลับศูนย์ถ้าเปิดขณะดันค้าง |
| ยืนยัน | ปล่อย RS หลัง dwell 60 ms · A = ทันที |
| ยกเลิก | B · D-pad ← · **ปล่อย LB** |
| Ctrl+ตัวอักษร | ดัน LS ≥ 0.35 ขณะ LB ค้าง → ปิด quick bar แล้วพิมพ์ด้วย `ctrl = true` |
| เปิดซ้ำ | ไม่เปิดจนกว่าจะปล่อย LB (edge + hold 150 msเท่านั้น) |
| HUD | **แถบแนวนอน 8 ช่อง 44×44** (05-visual-language §5) · active = สี Type · วงพิมพ์ซ่อน |

ช่อง (interaction model §5): ↑ Copy · ↗ Paste · → Cut · ↘ Undo · ↓ Redo · ↙ Select All · ← Save · ↖ Alt+Tab

**Alt+Tab**: เพิ่ม `alt: bool` ใน `KeyPress` (เดิมมีแค่ shift/ctrl) และ `vk::MENU` ใน plan
— ยังไม่ตัดสิน Q4 (Alt อยู่ปุ่มไหน) แต่ quick bar ใช้ Alt ได้แล้ว

### 2. Caret nav = โหมดย่อยของ Type (ADR-05)

| เรื่อง | ค่า |
| --- | --- |
| R3 แตะ 1 | เปิด/ปิดลูกศร (ค่าเริ่มต้น RS ว่าง) — ตัด gesture พิมพ์ที่ค้าง |
| R3 ×2 < 280 ms | เปิดลูกศร + โหมดเลือกข้อความ (ลูกศร + Shift) |
| RS ทิศ | ยิงลูกศรทันที · ค้าง = repeat **350 ms แล้วทุก 90 ms** (`GestureConfig` เดิม) |
| LS | ไม่มีผลในโหมดนี้ |
| A / B | ออกจากโหมดลูกศรโดยไม่ส่ง Space/Backspace (กันพิมพ์ทับ selection) — X/Y ยังทำงานปกติ |
| HUD | mode `caret` · ไม่มีวง · **กากบาท 4 ทิศ 132px สี `--mode-caret`** + pill `Caret` · `ลูกศร` / `เลือกข้อความ` |
| R3 ใน Mouse/Nav | ยังเป็น recenter/reset pivot ตามเดิม (หน้าต่าง double-tap รีเซ็ตเมื่อออกจาก Type) |

ใช้ 350/90 ตาม state machine §4 ไม่ใช่ 360/90 ของม็อกอัป (ม็อกอัปไม่ใช่ค่าจริง)

### 3. Sticky drag = RB แตะ 2 ครั้งใน Mouse mode

```
tap RB ครั้ง 1  → click ปกติ (down + up) · เริ่มหน้าต่าง 280 ms
tap RB ครั้ง 2  → down ค้างไว้ (ไม่ปล่อยตอนยกนิ้ว) = sticky drag
RS             → ลาก pointer ตามปกติ
tap RB อีกครั้ง → ปล่อยปุ่มซ้าย (จบการลาก)
```

- หน้าต่าง double-tap ใช้ `RuntimeConfig::double_tap_ms` (280) ค่าเดียวกับ A
- **A/B ในโหมดลูกศร** = ออกจากโหมดเฉย ๆ ไม่ส่งคีย์ (ADR-05: A = ยืนยัน, B = ยกเลิก)
  — ยังไม่ยุบ selection เองเพราะ core ไม่รู้ caret ของแอปปลายทาง (ข้อจำกัดเดิมของ ADR-05)
- **กันเมาส์ค้าง**: `Session::step` ตรวจ `sticky_drag` ก่อน/หลัง tick — ถ้า engine ล้างค่า
  (สลับ mode / emergency / จอยหลุด) จะ inject `MouseButton{Left,false}` ให้เสมอ
- HUD แสดง badge `Drag` ระหว่างล็อก

### 4. Pivot clamp = แอปเป็นเจ้าของ (ไม่ใช่ฝั่ง HUD)

- `Session::set_pivot_clamp(min, max)` เก็บขอบเขตพิกัดหน้าจอ แล้ว clamp ทุกครั้งที่
  `MovePivot` / `ResetPivot` / ตั้งค่าเริ่มต้น
- margin = **172 px × `ui.hudScale`** (รัศมีวง 340 px / 2 + ขอบ) — ค่าเดียวกับ `MARGIN`
  ใน `ui/hud/hud.js` (ADR-11) ตอนนี้แอป clamp แล้ว HUD เป็นชั้นกันสำรอง
- ใช้ virtual desktop ทั้งกรอบ (2 จอของ owner = x −1920..1920)

### 5. Contract v1 เพิ่มค่า enum (ไม่เพิ่ม field)

`mode` เพิ่ม `quickbar` และ `caret`; `inner.kind` ยังใช้ `command` (8 ช่อง) และ `none`
— HUD เดิมอ่าน state ได้ทันทีโดยไม่ต้องมี field ใหม่
เพิ่ม sample 2 ชุด (`quickbar/idle`, `caret/idle`) → contract checks 10 → **12/12**

## ผลที่ตามมา

- `cargo test --workspace` = **166 tests** (core 98 · win 37 · app 31)
- `hud-typography` 30 → **36 checks** (เพิ่ม quick bar 3 + caret 3)
- `hud-smoke` 54/54 ไม่เปลี่ยน (เป็นม็อกอัป ไม่ใช่ HUD จริง)
- งานที่ยังไม่ทำ: exclusion list/UIPI, scancode/IME, `timeBeginPeriod(1)`, pivot clamp
  ต่อจอแบบ mixed-DPI (ตอนนี้ใช้กรอบ virtual desktop เดียว)
