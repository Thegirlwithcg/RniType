---
title: ADR-27 — Keymap editor + per-cell overrides (P5 slice 3)
status: accepted
updated: 2026-11-16
tags: [decision, settings, keymap, config]
---

# ADR-27 — P5 slice 3: Keymap editor (EN/TH) + override รายช่อง

## บริบท

ADR-14 กำหนดให้แท็บ Keymap แก้ได้ทีละช่อง (คลิกช่อง → กดคีย์ใหม่, Esc ยกเลิก,
มีจุด accent ที่ช่องต่างจาก default, Reset group / Reset language)

แต่ core ยัง lookup ตาราง static (`keymap::lookup_layer`) — ไม่มีที่ให้ override
และ Settings ยังไม่มีแท็บ Keymap (แสดงเป็น disabled)

## การตัดสินใจ

### 1. เก็บ override ใน `languages[].overrides` (ไม่ใช่ตาราง 64 ช่อง)

```json
"languages": [
  { "code": "th", "enabled": true, "kind": "layout",
    "overrides": { "1-2-5": "ฬ", "2-3-4": "" } }
]
```

| เรื่อง | ค่า |
| --- | --- |
| key | `<layer>-<group>-<sector>` (layer 1–2, group/sector 0–7) |
| key รูปเก่า | `<group>-<sector>` = layer 1 (schema เดิม) — parser รองรับทั้งคู่ |
| value | 1 ตัวอักษร · `""` = ไม่มีคีย์ (อนุญาตเฉพาะ layer 2) |
| ทำไมไม่เขียน 64 ช่อง | diff เล็ก, merge กับ default ได้, reset ง่าย, ตรง ADR-14 |
| schema | `config.schema.json` ขยาย pattern เป็น `^([12]-)?[0-7]-[0-7]$` (backward compatible) |

### 2. Core: override ชนะตารางเริ่มต้น

- `TypingPipeline` ถือ `BTreeMap<(Lang, layer, group, sector), Option<Key>>`
- `key_at()` = override ถ้ามี ไม่งั้น `lookup_layer()` — ทั้งการพิมพ์จริงและ HUD preview ใช้ทางเดียวกัน
- `set_override` / `clear_overrides` · แยกตามภาษา → สลับภาษาแล้ว override ยังอยู่
- `RuntimeEngine::preview_key()` ให้ session วาด preview/วงนอกด้วยค่าที่ override แล้ว
- `Lang` derive `Ord` เพิ่มเพื่อใช้เป็น key ของ BTreeMap

### 3. Runtime apply

- `build_session` + live reload (250 ms) เรียก `Session::set_keymap_overrides(config::all_keymap_overrides())`
- เปลี่ยนช่องแล้วมีผลทันทีโดยไม่ต้องรีสตาร์ต (เหมือน ADR-25)

### 4. Commands / IPC

| command | args | returns |
| --- | --- | --- |
| `get_keymap` | `{ code }` | `{ code, label, groups, groups2, layers[2] {name,cells[64],defaults[64]}, overrides }` |
| `set_keymap_cell` | `{ code, layer, group, sector, text }` | payload เดียวกับ `get_keymap` |
| `reset_keymap` | `{ code, group? }` | payload เดียวกับ `get_keymap` |

- ผิดรูป = `CONFIG_INVALID` · ภาษาไม่มีตาราง = `LANG_NOT_FOUND` · เขียนไฟล์ไม่ได้ = `CONFIG_IO`
- `save_config` ตรวจ `validate_keymaps()` ด้วย (กัน config ภายนอกที่ key เพี้ยน)

### 5. UI

- Sidebar เปิดแท็บ **Keymap** (เดิม disabled)
- ซ้าย: 8 กลุ่ม (ชื่อตาม layer) · ขวา: กริด 3×3 ตามทิศ RS (กลางว่าง แสดงชื่อกลุ่ม)
- segmented **Base | Layer 2** · ช่องที่ต่างจาก default มีจุด accent 6px
- คลิกช่อง → input inline (Enter บันทึก · Esc ยกเลิก · blur = บันทึก)
- Base layer ห้ามว่าง · Layer 2 ว่างได้ (ช่องไม่มีคีย์)
- **Reset group** / **Reset language** · preview mode (ไม่มี Tauri) ใช้ตาราง EN ตัวอย่าง

### 6. ขอบเขตที่ยังไม่ทำ

- **Universal QWERTY mapping** สำหรับ layout อื่น (RU/EL/JA…) — ยังขึ้น `Needs mapping`
  และ toggle ปิดอยู่ · แท็บ Keymap แก้ได้เฉพาะ EN/TH
  (**อัปเดต ADR-28**: universal QWERTY ทำแล้ว · layout ปกติแก้ใน Keymap ได้ · IME ยังปิด)
- IME ยังต้องใช้ scancode path (ไม่ map ตัวอักษร) ตาม ADR-14 §5
- Shift-lock แสดงตัวพิมพ์ใหญ่บน HUD ตาม ADR-13 (ไม่ใช่ override)

## ผลที่ตามมา

- `cargo test --workspace` = **213 tests** (core 100 · win 56 · app 57)
- settings-smoke = **28/28** + `settings-keymap.png`
- E2E (WebView จริง): ตั้ง `th` ช่อง 1-0-0 = `z` → runtime พิมพ์ `z`, reset → กลับ `ก`
- ยังไม่ทำ: universal layout tables, keymap profiles, import/export
