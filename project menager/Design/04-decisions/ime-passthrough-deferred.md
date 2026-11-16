---
title: IME passthrough — เลื่อนไปเวอร์ชันถัดไป
status: deferred
updated: 2026-11-16
tags: [decision, i18n, ime, deferred]
---

# IME passthrough (JA/ZH/KO) — DEFERRED

## การตัดสินใจ (owner, 2026-11-16)

ข้าม IME passthrough ในรอบนี้ **แต่ต้องกลับมาทำในเวอร์ชันถัดไป**

## สถานะปัจจุบัน

- `installed_languages()` จำแนก JA/ZH/KO เป็น `kind: ime` ถูกต้อง
- UI แสดง badge `IME · keys` + `Needs mapping` และปิด toggle
- `set_language_enabled` ตอบ `LANG_NOT_FOUND` เมื่อพยายามเปิด IME
- `build_keymap` สร้างตารางจาก layout ปกติได้แล้ว (RU/EL) แต่ IME ต้องใช้คนละทาง

## สิ่งที่ต้องทำเมื่อกลับมา (v0.2)

1. **ส่งคีย์แบบ physical** (scancode/VK) โดยไม่แปลงเป็น Unicode เพื่อให้ IME ของ Windows ประกอบข้อความ
2. HUD แสดงแป้น QWERTY + badge `IME` (ไม่ map ตัวอักษร)
3. `ToUnicodeEx` ต้องไม่ถูกใช้กับภาษา IME (ตอนนี้ถ้าเปิดจะไม่เกิด เพราะปิดอยู่)
4. candidate window เป็นของ Windows — ห้ามวาดเอง
5. ทดสอบกับ Microsoft IME ญี่ปุ่น/จีน/เกาหลี: romaji→kana→kanji, compose/cancel
6. อัปเดต ADR-14 §5, architecture §3.1, HANDOFF เมื่อทำ

## อ้างอิง

- [[14-universal-languages]] §1, §5
- [[28-universal-qwerty-mapping]] (ฐาน `Keymap`/`LangSlot` ที่ต่อยอดได้)
- `crates/rnitype-win/src/sendinput.rs` (scancode path มีอยู่แล้ว)
