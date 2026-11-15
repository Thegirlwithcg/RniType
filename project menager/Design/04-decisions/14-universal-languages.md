---
title: ADR-14 — Universal languages + Settings window
status: accepted (mockup v0.11)
updated: 2026-11-15
tags: [decision, i18n, language, settings, ime]
---

# ADR-14 — รองรับทุกภาษา (layout + IME) + หน้าตั้งค่าแยก

## บริบท

เจ้าของต้องการให้แอป **universal** ไม่ใช่แค่ EN/TH:

1. ภาษาที่ติดตั้งใน Windows ควรปรากฏเป็นเมนู **แทนช่องภาษาไทย**
2. UI/Setting เป็น **ภาษาอังกฤษ**
3. ภาษาอื่น map กับ **ตำแหน่ง QWERTY** ก่อน แล้วแก้ไขได้
4. ไทยใช้ตารางในม็อกอัปเป็นค่าเริ่มต้น และแก้ไขได้
5. หน้าตั้งค่าเป็น **หน้าต่างแยก** มี **Reset** กลับค่าเริ่มต้น
6. ภาษากลุ่ม **IME ต้องทำงานเหมือนคีย์บอร์ดจริง** (Windows IME ประกอบเอง)

## การตัดสินใจ

### 1. Language registry 2 ชนิด

| ชนิด | ตัวอย่าง | พฤติกรรม |
| --- | --- | --- |
| `layout` | EN, TH, RU, EL | map เป็นตัวอักษร 8×8 (QWERTY หรือตารางที่ออกแบบ) |
| `ime` | JA, KO, ZH | HUD ส่ง **physical key** ให้ Windows IME ประกอบ (romaji→kana→kanji ฯลฯ) — ไม่ map ตัวอักษรเอง |

- **EN = base** (ตาราง QWERTY ที่ออกแบบไว้), **TH = ภาษาที่สองหลัก**
- **ตรวจภาษาก่อนใช้งาน**: ตอนเริ่มแอป enumerate layout ที่ติดตั้งใน Windows
  (`GetKeyboardLayoutList` / installed input languages) แล้วจึงสร้างรายการภาษา
  - ถ้าไม่มี layout ที่สอง → มีแค่ **English**
  - เครื่องไทย → **English + Thai** (ค่าเริ่มต้นของม็อกอัปนี้)
  - layout อื่นที่ติดตั้ง (RU/EL/JA/KO) จะถูกเพิ่มอัตโนมัติ — ในม็อกอัปแสดงเป็น `sample`
    และปิดไว้ จนกว่าจะเปิดใน Settings
- ภาษาอื่นที่ติดตั้ง: เริ่มจาก **QWERTY positions** (`ToUnicodeEx` กับ HKL ของภาษานั้น)

### 2. วงภาษา (level 2 ของวงคำสั่ง)

- ช่อง **↗ = "ภาษาอื่น" (Other language)** — ไม่ใช่รหัสภาษาปัจจุบัน
  - **มีภาษาที่สอง 1 ภาษา** → แสดงรหัสภาษานั้น (เช่น `TH`) และเลือก = สลับไปภาษานั้นทันที
  - **มีมากกว่า 1 ภาษา** → แสดง `Lang` แล้วเปิด **วงภาษา** ให้เลือก (เฉพาะภาษาที่ไม่ใช่ EN)
  - **มีภาษาเดียว (English)** → **ไม่มีช่องนี้ในวง** (7 ช่อง) — เปิดภาษาเพิ่มได้ภายหลังใน Settings
- **RS เลือก · ปล่อย RS = เปลี่ยนภาษา** · B = ยกเลิก
- ภาษา EN ยังเข้าถึงเร็วจากช่อง ↑ ของวงคำสั่ง

### 3. Mapping

- `` type-<lang> `` = 8 กลุ่ม × 8 ช่อง; ค่าเริ่มต้นของ `layout` มาจาก QWERTY ของ layout นั้น
- **แก้ได้ทีละช่อง** ในแท็บ Keymap (คลิกช่อง → กดคีย์ใหม่, Esc ยกเลิก)
- **Reset this language / Reset all mappings / Reset all to defaults**
- ไทยใช้ตาราง 8×8 ปัจจุบัน (ADR-12) เป็นค่าเริ่มต้น

### 4. หน้าตั้งค่า = หน้าต่างแยก (English UI)

- แท็บ **General · Languages · Keymap · HUD** + ปุ่ม **Reset all to defaults**
- ภาษาอังกฤษทั้งหมด (ป้าย, คำอธิบาย, toast)
- ในแอปจริงเป็นหน้าต่าง Tauri แยกจาก overlay — ม็อกอัปจำลองเป็นหน้าต่างลอย
- เนื้อหาที่เป็นของภาษา (ตัวอักษร, ชื่อกลุ่ม) ไม่แปล

### 5. IME = passthrough

- HUD แสดง **แป้น QWERTY** (physical keys) + badge `IME`
- ส่งคีย์แบบ scancode/VK (ไม่ใช่ Unicode) เพื่อให้ IME ของ Windows เห็นและประกอบข้อความ
- candidate window เป็นของ Windows (ไม่วาดเอง)
- แท็บ Keymap ของภาษา IME เป็น **read-only** (แก้ไม่ได้ เพราะไม่ได้ map ตัวอักษร)

## ผลต่อ state machine

- เพิ่ม state ย่อย `LANG_WHEEL` (คล้าย `CMD_WHEEL`)
- โหมดพิมพ์เปลี่ยนจาก `type-en/type-th` เป็น `type-<lang>` — `isTyping()` = ขึ้นต้นด้วย `type-`

## ผลที่ตามมา

- ม็อกอัป v0.11: `hud-prototype/` (วงภาษา, Settings window, Keymap editor, Reset)
- smoke test: **50/50 ผ่าน** (RU mapping, IME passthrough, settings UI, editor, reset)
- เอกสาร: [[01-interaction-model]], [[04-state-machine-spec]], [[02-keymap-en-th]],
  [[01-architecture]], [[02-roadmap-and-smoke-tests]], [[README]], [[HANDOFF]]

## ความเสี่ยง

| ความเสี่ยง | ระดับ | ทางรับมือ |
| --- | --- | --- |
| ภาษา IME ถูกคาดหวังให้ HUD แสดงตัวอักษรที่ประกอบแล้ว | กลาง | badge `IME` + เอกสารชัดว่า candidate window เป็นของ OS |
| layout บางตัวมี dead keys/組合 (เช่น เวียดนาม) | กลาง | ใช้ `ToUnicodeEx` หลายจังหวะ; ถ้าไม่ได้ให้ fallback เป็นคีย์ตรง |
| ตัวอักษรซ้ำ/ไม่มีในบาง layout | ต่ำ | ค่าเริ่มต้นจาก QWERTY + แก้ mapping ได้ + reset |
| หน้าตั้งค่าหลายภาษาแต่ UI อังกฤษอย่างเดียว | ต่ำ | v1 ตั้งใจ; เพิ่ม i18n ของ UI ภายหลังได้ |
