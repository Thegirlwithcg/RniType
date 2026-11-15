---
title: ADR-06 — D-pad → = เมนู Copy / Paste / Undo / Redo
status: superseded by ADR-07 (v0.7)
updated: 2026-11-15
tags: [decision, interaction, clipboard]
---

# ADR-06 — เมนูแก้ไขด้วย D-pad →

> ⚠️ **ถูกแทนที่แล้ว (v0.7)** — ปุ่มเปิดย้ายจาก D-pad → เป็น **D-pad ←** และรวมเข้าวงใน
> กับโหมด 4 ตัว (v0.10: เลือกด้วย RS): ดู [[10-command-wheel-rs]]
> (พฤติกรรม Copy/Paste/Undo/Redo ด้านล่างยังใช้ได้ทั้งหมด)

## บริบท

เจ้าของขอทางลัดสำหรับงานแก้ข้อความที่ใช้บ่อย: **Copy, Paste, Undo, Redo**
โดยไม่ต้องจำคีย์ผสมและไม่ต้องออกจากโหมดพิมพ์

## การตัดสินใจ

- **D-pad →** เปิด "เมนูแก้ไข" เป็นวงล้อ 4 ช่อง (ไม่ใช่ 8 เพื่อให้ยิงด้วย RS ได้แม่น)
- ใช้ **RS เลือกทิศ** แล้ว **ปล่อย RS = ทำทันที** (dwell 60 ms), **A = ทำ**, **B = ยกเลิก**
- ใช้กับ **LS** ได้ด้วย (เหมือนวงล้อโหมด) สำหรับคนที่ถนัดซ้าย

| ทิศ RS | คำสั่ง | พฤติกรรมในม็อกอัป |
| --- | --- | --- |
| ↑ | Copy | คัดข้อความที่เลือก; ถ้าไม่มี selection = คัดทั้งบรรทัด |
| → | Paste | แทรก clipboard ที่ caret (แทนที่ selection ถ้ามี) |
| ↓ | Undo | ย้อน snapshot ล่าสุด (เก็บ 80 ครั้ง) |
| ← | Redo | ทำซ้ำ snapshot ที่ถูกย้อน |

- ปุ่มที่ใช้ไม่ได้ (clipboard ว่าง / ไม่มี undo) จะจางลงในวงล้อ
- ทำจริงบน Windows: ใช้ `SendInput` ส่ง `Ctrl+C/V/Z/Y` ให้แอปเป้าหมาย
  — **ไม่ต้องอ่าน clipboard เอง** ใน v1 (ลดความเสี่ยง privacy และ UIPI)
  ยกเว้น Copy/Paste ที่ผู้ใช้เลือกผ่านเมนูของแอปเอง

## เหตุผลที่เลือก D-pad →

- D-pad ↑ ถูกใช้เปิดวงล้อโหมดแล้ว; → ว่างและจำง่าย ("ไปข้างหน้า = แก้ไข")
- ไม่ชนกับ Quick bar (LB ค้าง) ที่เป็นทางลัดแบบคีย์ผสม
- 4 ช่องพอดีกับของที่ใช้จริงบ่อย (Copy/Paste/Undo/Redo) — ไม่ต้องยัด 8

## ผลที่ตามมา

- ม็อกอัป v0.5: `Design/05-mockup/hud-prototype/` (27 checks ผ่าน)
- เอกสาร: [[01-interaction-model]] (ย่อหน้า D-pad →), [[04-state-machine-spec]] (`EDIT_MENU`)
- ยังไม่แตะ `rnitype-core` — เมนูนี้เป็นชั้น UI/controller ของแอป

{"properties":{"title":"ADR-06 — D-pad → = เมนูแก้ไข","status":"superseded by ADR-07 (v0.7)","updated":"2026-11-15","tags":["decision","interaction","clipboard"]}}
