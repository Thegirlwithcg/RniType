---
title: ADR-19 — ยกเลิก transform animation ของวง + Shift ต้องเปลี่ยนตัวอักษรบนวง
status: accepted (2026-11-15)
updated: 2026-11-15
tags: [decision, hud, animation, shift]
---

# ADR-19 — เอาขยาย sector ออก และแก้ Shift ให้ตัวอักษรบนวงเป็นตัวใหญ่

## บริบท (feedback รอบ 3 จาก owner)

1. **"ตัว Animation ไม่ค่อยดีเท่าไหร่ ทำให้วงล้อเพี้ยน"** — ADR-17 §5 ขยาย sector ด้วย
   `transform: scale(1.05–1.07)` จากศูนย์กลางวง → wedge 45° กินมุมเกิน 45° (เช่น 48°)
   ทำให้ sector ทับกัน เห็นเส้นStrokeทับกันเป็นรอยหัก วงดูเพี้ยน
2. **"กด Shift แล้วตัวอักษรภาษาอังกฤษ UI ไม่เปลี่ยนเป็นตัวพิมพ์ใหญ่"** — ADR-13 บอกว่าล็อกแล้ว
   วงต้องแสดง A–Z แต่ฝั่งแอปส่ง label ดิบจาก keymap (ตัวเล็ก) ตรง ๆ

## การตัดสินใจ

### 1. ไม่ใช้ transform กับ sector/ข้อความในวง (กลับไปใช้สี + stroke + ขนาดฟอนต์)

| องค์ประกอบ | เดิม (ADR-17) | ใหม่ (ADR-19) |
| --- | --- | --- |
| sector ที่เลือก | `scale(1.05–1.07)` | **ไม่ขยาย** · fill + stroke หนาขึ้น (1 → 1.5/1.6) |
| group ที่เลือก | sector + label `scale(1.1)` | ไม่ขยาย · label สว่างขึ้น `#e6f4ff` |
| char ที่เลือก | `scale(1.3)` | **font-size ใหญ่ขึ้น** 22.5 × textScale (≈31.5px) + ขาว + glow |
| cmd label ที่เลือก | `scale(1.16)` | font-size 11.5 × textScale (≈16px) + ขาว |
| lang label | `scale(1.14)` | ขาวเท่านั้น |

เหตุผล: `font-size` ขยายจากจุดกึ่งกลางของตัวอักษร **ไม่แตะเรขาคณิตของ sector**
วงจึงไม่มีวันเพี้ยนไม่ว่าค่า scale เท่าไร · motion ยังมี (transition font-size 0.2 s)

### 2. Shift lock → ตัวอักษรบนวงเป็นตัวใหญ่ (ที่ฝั่งแอป)

- `Session` แปลง label ด้วยกฎเดียวกับ `RuntimeEngine::with_shift_lock`:
  `a–z → A–Z` (ASCII เท่านั้น, ไทย/สัญลักษณ์คงเดิม)
- ใช้กับ **outer labels** (วงตัวอักษร) และ **preview กลางวง**
- `status.text = "ตัวใหญ่"` เมื่อล็อกและอยู่ในโหมดพิมพ์ (ตรงกับ [[13-shift-toggle]])
- HUD ไม่ต้องแก้อะไร (วาดตาม contract) · `hud-state.samples.json` + demo states
  ปรับ label ของ state ที่ `shiftLock: true` เป็นตัวใหญ่ให้ตรงกฎ

## ผลที่ตามมา

- `ui/hud/hud.css`: ลบ `transform-box/transform/transition: transform` ออกจาก sector + label
  ทั้งหมด · คืน `--fs-char-selected` (22.5) และ `--fs-cmd-selected` (11.5)
- `crates/rnitype-app/src/session.rs`: `shifted()` + ใช้กับ labels/preview/status
- `tools/tests/hud-typography.mjs`: เปลี่ยน 3 checks จาก "scale" เป็น
  **"ตัวอักษรที่เลือก ≥ 1.15×"**, **"stroke หนาขึ้น"**, **"ไม่มี transform บน sector"** (กัน regression)
- `cargo test`: +1 test `shift_lock_uppercases_the_ring_labels_and_the_preview`

## หลักฐาน

| หลักฐาน | ผล |
| --- | --- |
| `cargo test --workspace` | **109 passed** (app 21 · core 52 · win 36) |
| `node tools/tests/hud-typography.mjs` | 29/29 (รวม check ใหม่ 4 ข้อ) |
| `node tools/contracts/validate.mjs` | 10/10 |
| screenshot | `05-mockup/hud-shots/hud-typing-100dpi.png` (ไม่มี sector ทับกัน) |

## ถ้าอยากได้ animation กลับมา

ทางที่ไม่ทำให้เรขาคณิตเพี้ยน:
- ขยาย **fill/glow** (`filter: drop-shadow`) หรือ opacity แทน transform
- หรือขยาย **sector ทั้งวง** (scale ทั้ง `<g>` ของ ring ชั้นนั้น) ไม่ใช่ขยายชิ้นเดียว
- หรือทำ outer ring ให้กว้างขึ้นชั่วคราว (r เปลี่ยน) แล้ว clip ตามมุมเดิม
