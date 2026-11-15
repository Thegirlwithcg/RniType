---
title: HUD notes — v0.9 → v0.11 (B, RS, pivot, ภาษา, Settings)
status: accepted — v0.11
updated: 2026-11-15
tags: [mockup, ui, review, decision]
---

# HUD notes — v0.9 → v0.11

> v0.9: variant B ([[08-hud-declutter-variant-b]]) · เปลี่ยนกลุ่มด้วย LS ([[09-ls-switch-group]])
> v0.10: วงคำสั่งเลือกด้วย RS ([[10-command-wheel-rs]]) · HUD pivot + Place UI ([[11-hud-pivot-nav]]) ·
> วรรณยุกต์/ขีดกำกับ ([[12-thai-tone-and-below-vowel]]) · Shift toggle ([[13-shift-toggle]])
> v0.11: หลายภาษา + Settings window ([[14-universal-languages]])

## v0.11 — Universal languages (เน้นไทย)

- **ตรวจภาษาก่อน**: ม็อกอัปจำลองเครื่องไทย → เปิด **English + Thai** (RU/EL/JA/KO เป็น `sample` ที่เปิดได้ใน Settings)
- **ช่องภาษาอื่น** ในวงคำสั่ง: 1 ภาษา → แสดงรหัส (TH) และสลับทันที · หลายภาษา → `Lang` เปิดวงภาษา · **ภาษาเดียว → ไม่มีช่องนี้** (ไปเปิดภายหลังได้ใน Settings)
- **layout** (RU, EL, …) map จากตำแหน่ง QWERTY อัตโนมัติ · แก้ได้ทุกช่อง
- **IME** (JA, KO, …) HUD แสดง QWERTY + badge `IME` — ส่งคีย์ให้ Windows ประกอบเอง
- **Settings window แยก** ภาษาอังกฤษ: General · Languages · Keymap · HUD + Rescan/Reset
- ไทยใช้ตาราง 8×8 เดิม (ป้ายกลุ่มเป็นภาษาไทย) เป็นค่าเริ่มต้น

### วงคำสั่ง: ช่อง "ภาษาอื่น" แสดง TH
![[v11-slot-th.png]]

### Settings → Languages (detected layouts)
![[v11-settings-languages.png]]

### Settings → Keymap (แผนที่จอย + 8×8 editor)
![[v11-settings-keymap.png]]

## v0.10 — สิ่งที่เปลี่ยน

- **วงคำสั่งเลือกด้วย RS** (เดิม LS) — D-pad ← ยังเป็นปุ่มเปิด; LS ไม่มีผลในวง
- **HUD ตรึงที่ pivot** ไม่ตามเมาส์แล้ว · **Nav = โหมดจัดตำแหน่ง UI** (LS หยาบ · RS ละเอียด · R3/R กลับตำแหน่ง)
- **ไทย**: วรรณยุกต์ ่ ้ ๊ ๋ รวมกลุ่ม ↘ เดียวกัน (ย้าย ฤ ฮ ไปกลุ่ม →)
- **สระล่าง ุ ู** มีขีดกำกับเหนือตัวอักษร
- **Shift/LT = toggle**: กดครั้งเดียวล็อกตัวพิมพ์ใหญ่ (UI แสดง `ตัวใหญ่`, ตัวอักษรบนวงเป็น A–Z), กดอีกครั้งกลับตัวเล็ก

### Shift = ล็อกตัวพิมพ์ใหญ่ (pill แสดง ตัวใหญ่)
![[v10-shift.png]]

### ไทย: สระล่างมีขีด
![[v10-th-vowel.png]]

### Nav = จัดตำแหน่ง UI (pivot)
![[v10-nav.png]]

## v0.9 — สิ่งที่เปลี่ยน

- idle เหลือ **pill สถานะ** ใต้ cursor · วงโผล่เมื่อดัน LS
- กลุ่มที่ armed เด่น กลุ่มอื่นจาง
- ตัด ghost / aim wedge / chip `L1` / `·` placeholder · รวมสถานะเป็นบรรทัดเดียว
- วงคำสั่งซ่อนวงนอกสนิท + ชื่อคำสั่งกลางวง
- **เลื่อน LS ไปกลุ่มอื่นได้ทันที** ไม่ต้องปล่อยแล้วดันใหม่
- เปิด "แสดงวงตลอดเวลา (โหมดเรียนรู้)" ในแผงตั้งค่า = พฤติกรรมแบบ variant A

## ภาพ v0.9

### idle (เหลือ pill)
![[v09-idle.png]]

### เปลี่ยนกลุ่มด้วย LS (↑ → →) กลุ่ม C
![[v09-group2.png]]

### หน้าเต็ม
![[v09-full.png]]

## ประวัติการเทียบ A/B (ก่อนเลือก)

| | A — ตัดของซ้ำ | B — A + progressive |
| --- | --- | --- |
| idle | วงใน + วงนอกว่าง | เหลือ pill สถานะ |
| กลุ่มอื่น | เท่ากัน | จาง 35% |
| วงคำสั่ง | เหมือนกัน | เหมือนกัน |

![[a-idle.png]]
![[b-idle.png]]

## วิธีลอง

เปิด `project menager/Design/05-mockup/hud-prototype/index.html`
→ ปุ่ม ⚙ มุมขวาบน = **Settings window** (General / Languages / Keymap / HUD)
→ `M`/`C` หรือ D-pad ← = วงคำสั่ง → ช่อง **Language** → เลือกภาษาด้วย RS
→ ใน Keymap: **แผนที่จอย Xbox / DualSense** (สลับด้วยปุ่มขวาบน · hover แถบคำอธิบาย = วงแหวนไฮไลต์ปุ่มจริง)
→ ตาราง 8×8: **แถว = ทิศ LS (กลุ่ม) · คอลัมน์ = ทิศ RS (ตัวอักษร)** · คลิกช่องแล้วกดคีย์ใหม่ · **Reset this language** / **Reset all to defaults**
→ ตัวเลือกภาษา (dropdown) เป็นธีมมืด อ่านชัดทั้งรายการ

ทดสอบอัตโนมัติ: `node tools/tests/hud-smoke.mjs` — **54/54 ผ่าน**

## เกณฑ์ที่จะวัดกับคนจริง (ต่อใน M2.5)

- เปิดวงคำสั่ง + เลือกคำสั่งได้ภายใน 2 วินาที
- เปลี่ยนกลุ่มด้วย LS ถูกกลุ่มภายใน 1.2 วินาที
- ไม่มีสถานะไหนที่ผู้ใช้ต้องเดา (mode / ชั้น 2 / Shift / ลูกศร)
- คะแนน "รก" ลดลงเทียบ v0.7 (สอบถาม 3–5 คน)

