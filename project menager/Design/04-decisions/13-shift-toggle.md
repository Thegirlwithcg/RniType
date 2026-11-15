---
title: ADR-13 — Shift/LT = สลับตัวพิมพ์ใหญ่ (toggle)
status: accepted (mockup v0.10)
updated: 2026-11-15
tags: [decision, typing, modifier, shift]
---

# ADR-13 — Shift/LT กดครั้งเดียว = ล็อกตัวพิมพ์ใหญ่

## บริบท

เดิม LT (Shift) เป็น **hold**: ต้องกดค้างตลอดเวลาที่พิมพ์ตัวใหญ่ และไม่มีสถานะบน UI
เจ้าของขอให้ **กด Shift 1 ครั้ง = ตัวพิมพ์ใหญ่ (ล็อก) และ UI แสดงสถานะ**, กดอีกครั้ง = ตัวพิมพ์เล็ก

## การตัดสินใจ

- ในโหมดพิมพ์ (Type-EN/TH): **LT กด = toggle** `shiftLock`
  - เปิด: ตัวอักษร a–z ทั้งวงแสดงเป็น A–Z, preview/ตัวที่พิมพ์เป็นตัวใหญ่
  - ปิด: กลับเป็นตัวเล็ก
- **UI แสดงสถานะ**: pill สถานะเพิ่ม `· ตัวใหญ่` และตัวอักษรบนวงเปลี่ยนเป็น uppercase ทันที
- **โหมด Mouse/Nav**: LT ยังเป็น **hold = precision** ตามเดิม (toggle เฉพาะตอนพิมพ์)
- ใช้ edge ของสัญญาณ LT (analog trigger ผ่าน threshold 0.4) — ไม่มี double-tap
- shiftLock คงอยู่จนกดอีกครั้ง (ไม่รีเซ็ตอัตโนมัติหลังพิมพ์ 1 ตัว)

## ผลต่อ state machine

- เพิ่มฟิลด์ `shiftLock` ใน state (ไม่เพิ่ม state ใหม่)
- `currentChar()` และการแสดงผลวงใช้ `shiftLock` แทน `pad.lt`
- LT ยังถูกอ่านเป็น analog สำหรับ precision ใน Mouse/Nav

## ผลที่ตามมา

- ม็อกอัป v0.10: `hud.js` — edge detect, ring แสดง uppercase, pill `ตัวใหญ่`
- smoke test: กด Shift ครั้งเดียว → ล็อก + chars ใหญ่ + pill; กดอีกครั้ง → กลับเล็ก; พิมพ์ `A` ได้จริง
- เอกสาร: [[01-interaction-model]] §5, [[02-keymap-en-th]], [[README]], [[HANDOFF]]

## ความเสี่ยง

| ความเสี่ยง | ระดับ | ทางรับมือ |
| --- | --- | --- |
| ผู้ใช้เคยชินกับการกดค้าง แล้วตัวใหญ่ค้างโดยไม่รู้ | กลาง | pill `ตัวใหญ่` ชัด + ตัวอักษรบนวงเป็นตัวใหญ่ |
| LT ที่เป็น analog trigger สั่นตรง threshold | ต่ำ | ใช้ threshold 0.4 + edge เดียวต่อการกด |
| Shift ค้างข้ามโหมด/แอป | ต่ำ | แสดงใน pill เฉพาะโหมดพิมพ์; รีเซ็ตเมื่อปิด/เปลี่ยนโปรไฟล์ (แอปจริง) |
