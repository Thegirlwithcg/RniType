---
title: ADR-30 — Practice tab: real error rate (edit distance)
status: accepted
updated: 2026-11-16
tags: [decision, measurement, settings, benchmark]
---

# ADR-30 — วัด error rate จริงด้วยแท็บ Practice

## บริบท

P6 slice 2 วัด "corrections" (จำนวน Backspace) เป็นเพียง proxy — ไม่รู้ว่าพิมพ์ผิดจริงไหม
Roadmap M5 ต้องการ error < 1.5% จากข้อความอ้างอิง และ owner ต้องมีวิธีทดสอบเองโดยไม่ต้องนับมือ

## การตัดสินใจ

### 1. วัดใน Settings (ไม่ต้องอ่านข้อความจากแอปอื่น)

- แท็บ **Practice**: แสดงประโยคอ้างอิง + `<textarea>` + ปุ่ม Start
- ผู้ใช้กด Start → textarea ได้ focus → พิมพ์ด้วยจอย (คีย์ถูก inject เข้า Settings เอง)
- JS คำนวณ **Levenshtein distance** ระหว่างข้อความที่พิมพ์กับประโยคอ้างอิง
  - `errors` = edit distance (insert/delete/substitute)
  - `error%` = errors / จำนวนตัวอักษรอ้างอิง × 100 (cap 100)
  - `WPM` = (ตัวอักษรที่พิมพ์ / 5) ÷ นาที
- จบเมื่อข้อความตรงกันเป๊ะ → freeze เวลาแล้วบันทึก

**ทำไมไม่ใช้ OCR/อ่าน Notepad**: เปราะและช้า · การวัดในหน้าต่างของแอปเองคือ end-to-end
จริง (gesture → keymap → SendInput → focused control) และ diff ได้ตรงไปตรงมา

### 2. เก็บผล

- command `record_bench { lang, target, typed, errors, errorPercent, wpm, seconds }`
- append JSONL ที่ `%APPDATA%\RniType\bench.jsonl` (1 บรรทัด/ครั้ง, มี timestamp)
- log: `[bench] lang=th chars=34 errors=2 error=5.9% wpm=9.3 secs=44.0`
- preview mode (ไม่มี Tauri) คำนวณในหน้าแต่ไม่เขียนไฟล์

### 3. ประโยคตัวอย่าง

| ภาษา | ใช้ทดสอบ |
| --- | --- |
| en | the quick brown fox jumps over the lazy dog |
| th | การพิมพ์ด้วยจอยต้องใช้ความคุ้นเคยและฝึกฝนอย่างสม่ำเสมอ |
| ru | быстрая коричневая лиса прыгает через ленивую собаку |
| el | η γρήγορη καφέ αλεπού πηδάει πάνω από τον τεμπέλη σκύλο |

## ผลที่ตามมา

- settings-smoke **33/33** (เพิ่ม exact match = 0 error, ข้อความผิด = error สูง, Start focus)
- ตัวเลข error rate เป็นของจริง (diff) ไม่ใช่ proxy — ใช้รายงาน M5 ได้
- ยังไม่ทำ: บันทึกผลลง HUD/กราฟ, เปรียบเทียบหลายรอบ, export CSV
