---
title: ADR-21 — P2 slice 3: Mouse/Nav math · auto-repeat · layer 2 · double-tap/hold
status: accepted (2026-11-16)
updated: 2026-11-16
tags: [decision, core, mouse, nav, repeat, layer2]
---

# ADR-21 — P2 slice 3: Mouse/Nav math · auto-repeat · layer 2 · double-tap/hold

## บริบท

HANDOFF v0.5 ระบุงานถัดไปเป็น P2 slice 3: mouse math/accel, Nav pivot,
auto-repeat, layer 2, double-tap/hold เอกสารออกแบบมีสูตร/ค่าเวลาแล้ว แต่ยังไม่ลงโค้ด
และมีจุดที่เอกสารเปิดช่องไว้ (ทิศทางของ repeat, การล็อก layer 2, A double-tap)
ADR นี้บันทึกค่าที่ลงจริงเพื่อไม่ต้องตัดสินซ้ำ

## การตัดสินใจ

| เรื่อง | ค่าที่ใช้ | ที่มา |
| --- | --- | --- |
| Pointer | deadzone 0.15 · gamma 1.7 · vmax 1600 px/s · smoothing 0.5 · LT ×0.3 · LB ×1.8 | `03-mouse-and-navigation.md` §1 (ไม่เปลี่ยนค่า) |
| Pointer smoothing | `d_out = lerp(d_prev, d, 0.5)` ที่ทุก tick + เก็บเศษพิกเซล | §1 + กันการขยับหายตอน dt 4 ms |
| Recenter (Mouse) | กด RS → `Action::RecenterPointer` → กลางจอหลัก (session ถือ home) | §3 |
| Scroll | LS ↑/↓ 1 notch/40 ms, เร่ง 3 notch หลัง 500 ms, LT = 1 notch/120 ms, LS ←/→ = wheel แนวนอน | §5 |
| Scroll ปลายสุด | LS ↖ = Ctrl+Home · ↘ = Ctrl+End (ครั้งเดียวต่อการดัน) | §5 |
| Nav | LS 900 px/s · RS 320 px/s · LT ×0.35 · RS click = pivot กลับจุดเริ่มต้น | §6 |
| Auto-repeat | commit แล้ว **ดัน RS กลับทิศเดิม** ค้าง 350 ms → พิมพ์ซ้ำทันที แล้วทุก 90 ms | spec §2/§4 |
| `REPEATING` | ไม่ถูก `cancel_dwell` 600 ms ตัด · ปล่อย RS = หยุด · ปล่อย LS = ลืม pending | spec §2 |
| Layer 2 | RB/RT ใน Type mode · ค้าง = ชั่วคราว · **แตะเร็ว (< 280 ms) โดยไม่พิมพ์ระหว่างนั้น = ล็อก/ปลดล็อก** | keymap §4 |
| Layer กลาง gesture | เปลี่ยนชั้นขณะ `GROUP_ARMED/CHAR_SELECTED/COMMITTING` = ยกเลิก gesture | กันพิมพ์ผิดชุด |
| A double-tap | 280 ms · Space แรกออกทันที · แตะครั้งที่สอง = Enter (ไม่ส่ง Space ที่สอง) | interaction model §4 |
| B hold | 400 ms = Ctrl+Backspace (ลบทั้งคำ) | §4 |
| Y tap/hold | Tab ตอนปล่อย · ค้าง 400 ms = Esc (กดค้างไม่ส่ง Tab) | §4 |

## ทิศทางของ auto-repeat (ตัดสินจากเอกสารที่ขัดกัน)

สเปกเขียนว่า commit เกิดตอน **ปล่อย RS** แต่ก็เขียนว่า "หลัง commit ถ้า RS ยังค้างทิศเดิม"
จึงตีความว่า **ผู้ใช้ดัน RS กลับไปทิศเดิมแล้วค้าง** ไม่ใช่ค้างตั้งแต่ก่อน commit
ลำดับที่ลงจริง:

```
commit (ปล่อย RS + dwell 60 ms) → จำ (group, sector) ไว้
ดัน RS กลับทิศเดิม → CharSelected → selected_ms ≥ 350 ms → REPEATING + พิมพ์ 1 ครั้ง
ค้างต่อ → ทุก 90 ms · เปลี่ยน sector = เลือกตัวใหม่ · ปล่อย RS/LS = หยุด
```

ผลข้างเคียง: หลัง commit ผู้ใช้ยังพิมพ์ตัวเดิมซ้ำแบบ "ครั้งเดียว" ได้ถ้าปล่อย RS ก่อน 350 ms
ซึ่งเป็นพฤติกรรมปกติของการพิมพ์ และการค้างเกิน 600 ms จะไม่ถูกยกเลิกถ้าเป็นตัวที่ repeat ได้

## A double-tap: ยอมมี Space ก่อน Enter

ทางเลือกมี 2 ทาง:

1. หน่วง Space 280 ms เพื่อรอ double-tap → ตัวอักษรที่ใช้บ่อยที่สุดช้าลงทุกครั้ง (ไม่เลือก)
2. ส่ง Space ทันที · ถ้าแตะซ้ำภายใน 280 ms ส่ง Enter แทน Space ครั้งที่สอง (เลือก)

ผลลัพธ์คือ `" \n"` (มีช่องว่างก่อนขึ้นบรรทัดใหม่) ซึ่งไม่กระทบข้อความส่วนใหญ่
และแลกมาด้วย latency 0 ของ Space ถ้า owner ไม่ชอบ สามารถเปลี่ยนเป็นแบบหน่วงได้ใน Settings ภายหลัง

## Layer 2 ของไทย

- ใช้ตารางตาม `02-keymap-en-th.md` §4; ช่องที่ไม่มีคีย์ = `None` (commit แล้วไม่ส่งอะไร)
- EN กลุ่ม ↘ สเปกมี 6 ตัว → เติม `° ± §` ให้ครบ 8 ช่อง (ตรงกับม็อกอัป `hud-prototype` เดิม)
- กลุ่ม ↖ ช่อง Ctrl/Shift ยังว่าง (ต้องมีคีย์ผสมแบบค้าง — ไปกับ Quick bar ในรอบถัดไป)

## Action ที่เพิ่ม

| Action | ใครใช้ |
| --- | --- |
| `MouseHWheel { delta }` | Mouse mode scroll แนวนอน → `MOUSEEVENTF_HWHEEL` |
| `RecenterPointer` | Mouse mode: กด RS (session แปลงเป็น `MouseMoveAbs` ที่กลางจอ) |
| `ResetPivot` | Nav mode: กด RS (session คืน pivot ไป `pivot_home`) |
| `NamedKey::PrintScreen` | ชั้น 2 EN กลุ่ม ↙ |

## หลักฐาน

```
cargo test --workspace        → 145 passed (core 84 + win 36 + app 25)
node tools/contracts/validate.mjs
node tools/tests/hud-smoke.mjs
node tools/tests/hud-typography.mjs
```

unit test ใหม่ที่สำคัญ: pointer deadzone/gamma/vmax/smoothing/residual · pivot coarse/fine ·
scroll 40/120/500 ms · repeat 350/90 ms · layer 2 tap/hold · double-tap/hold ·
RS recenter/reset ผ่าน session → HUD (`pointer`, `pivot`, `layer`, `state=repeat`, `preview.count`)

## ความเสี่ยง / ที่ยังไม่ทำ

| เรื่อง | สถานะ |
| --- | --- |
| RB แตะ 2 ครั้ง = sticky drag | ยังไม่ทำ (ต้องมี state drag ค้างใน Mouse mode) |
| Quick bar (LB ค้าง) / caret nav (R3) | ยังไม่ทำ — งาน P2 รอบถัดไป |
| Pivot clamp ตามขอบจอ + margin 172×scale | ยังไม่ทำ — ต้องรู้ geometry ของ overlay ฝั่ง app |
| Mixed-DPI pointer | ยังใช้ px ของ virtual desktop; DPI-aware ตามจอที่ pointer อยู่ยังไม่ทำ |
| RT analog hysteresis | จอยที่ทดสอบเป็นดิจิทัล (LZ/RZ = 0) → ใช้ `buttons.rt` ก่อน |
