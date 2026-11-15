---
title: ADR-16 — Typography ของ HUD (ขนาดตัวหนังสือ + ฟอนต์ไทยไม่มีหัว)
status: accepted (P4 slice 3 + P5 slice 1)
updated: 2026-11-15
tags: [decision, hud, typography, thai, font, settings]
---

# ADR-16 — Typography: text scale แยกจาก ring scale + IBM Plex Sans Thai

## บริบท

owner รายงาน (2026-11-15) ว่า **ตัวหนังสือ HUD เล็กและอ่านไม่ชัด** โดยเฉพาะบนจอ 3840×1080
และภาษาไทย และขอ **ฟอนต์ไทยไม่มีหัว (loopless)** + **Settings ปรับขนาด/เลือกฟอนต์ได้**

ค่าเดิม: group label 11px (len-l 8px) · char 15–27px · sub/badge 8–10.5px · pill 10.5px · ring 340px
ฟอนต์เดิม: `Segoe UI, Leelawadee UI` (Leelawadee = ฟอนต์ไทยมีหัว)

ปัญหาที่ต้องแก้ให้ถูกจุด: **การขยายวง (scale) ไม่ทำให้ตัวหนังสือใหญ่ขึ้น**
ถ้าเพิ่ม `hudScale` อย่างเดียว วงจะใหญ่แต่ตัวอักษรยังเล็กเท่าเดิมในสัดส่วน

## การตัดสินใจ

### 1. แยกตัวคูณ 2 ตัว

| ตัวคูณ | ค่าเริ่มต้น | คุมอะไร | มาจาก |
| --- | --- | --- | --- |
| `ui.hudScale` → `HudState.scale` | **1.2** (เดิม 1) | ขนาดวงทั้งวง (340 → 408px) | state รายเฟรม |
| `ui.hudTextScale` → `hud:style.textScale` | **1.2** | ตัวหนังสือทุกตัว (label/char/sub/badge/pill) | event `hud:style` |

`hud.css` ประกาศ `--hud-text-scale` แล้วให้ทุก `font-size` เป็น `calc(<base>px * var(--hud-text-scale))`
(ตัวแปร `--fs-*` กลาง ๆ ใน `:root` ของ `hud.css`)

### 2. ขนาดขั้นต่ำที่ล็อก (ที่ text scale 1.2)

| องค์ประกอบ | เดิม | ใหม่ | หมายเหตุ |
| --- | --- | --- | --- |
| group label | 11 | **13.8** | len-m 12.6 · len-l **11.4** (ป้ายไทย 7–8 ตัว เช่น วรรณยุกต์) |
| char (unselected / selected) | 15 / 27 | **20.4 / 30** | dim = 20.4 แต่สีจางลง |
| sub · badge | 8 | **11** | offset ใต้ label ขยายตาม text scale ด้วย |
| pill (chip สถานะ) | 10.5 | **13.2** | pill-badge 12 |
| cmd label | 10.5 / 12 | **13.2 / 15** | lang label 13.8 |
| preview / repeat (กลางวง) | 26 / 11 | **26.4 / 12** | |

ตรวจอัตโนมัติใน `tools/tests/hud-typography.mjs` (24 checks) — ทุกเกณฑ์ข้างบนเป็น assert

### 3. ฟอนต์

- **IBM Plex Sans Thai** (loopless, OFL) = ค่าเริ่มต้น · 4 น้ำหนัก 400/500/600/700
- **Noto Sans Thai** (variable 100–900, OFL) = ตัวสำรอง → เลือกได้ใน Settings
- **System** = `Segoe UI, Leelawadee UI` (ทางถอย)
- ไฟล์ + license อยู่ใน `ui/shared/fonts/` · `@font-face` อยู่ที่ `ui/shared/fonts.css` · stack อยู่ที่ `ui/shared/fonts.js` (แหล่งเดียวของ HUD + Settings)
- `font-display: block` — ยอมให้ข้อความหายไปเสี้ยววินาที ดีกว่าโชว์ fallback ที่เปลี่ยนรูปทรงไทยกลางคัน (HUD เปลี่ยนข้อความทุกเฟรม)

### 4. ช่องทางส่งค่า typography = `hud:style` (ไม่ใส่ใน HudState)

- `HudState` = state รายเฟรม (≤ 60 Hz) — typography เปลี่ยนปีละไม่กี่ครั้ง จึงไม่ควรกิน payload ทุกเฟรม
- เพิ่ม **event `hud:style`** `{ textScale, font }` + **command `hud_style`** (ให้ HUD ดึงค่าตอน boot กันหลุด event)
- payload เดิมไม่เปลี่ยน → **additive v1.1** (ไม่ bump `version: 1`)
- เพิ่ม `ui.hudTextScale` (0.8–2, default 1.2) + `ui.hudFontFamily` (enum 3 ค่า) ใน `config.schema.json`

### 5. Settings window (P5 slice 1)

- หน้าต่างแยก `settings` (มีขอบ มี focus) เปิดจาก tray → **Settings…** หรือ `--settings`
- แท็บ HUD: Text size slider (0.8–2.0, step 0.05) + Font 3 ตัวเลือก + preview ฟอนต์/ขนาดจริง + Reset HUD to defaults
- เปลี่ยนแล้ว save ทันที (debounce 200 ms) → `save_config` → app เขียน `%APPDATA%\RniType\config.json` → emit `hud:style` → HUD เปลี่ยนทันที
- UI ภาษาอังกฤษ (ตาม ADR-14) · แท็บ General/Languages/Keymap แสดงเป็น disabled (P5)

### 6. Contrast

ตัวอักษรจางยกสว่างขึ้นเพื่อให้ ≥ 4.5:1 บนพื้น HUD เข้ม (`#12161e`)
— `char-text.is-dim` `#566076` (2.9:1) → `#94a0b6` (**6.87:1**)
— `sector-label.is-muted` `#5f6a7f` → `#8d98ad` · `slot-sub` `#7f8ba1` → `#93a0b6`

## ผลที่ตามมา

- `ui/hud/hud.css` — ตัวแปร `--fs-*` + `--hud-text-scale` + `--hud-font-family`
- `ui/hud/hud.js` — `applyStyle()` (จาก `hud:style` / `hud_style` / `?textScale=&font=`), offset ของ sub/badge ขยายตาม text scale
- `ui/shared/fonts.css`, `ui/shared/fonts.js`, `ui/shared/fonts/*.ttf` (+ OFL)
- `crates/rnitype-app/src/config.rs` — อ่าน/เขียน config, `ui_style()`, `validate_ui()` (6 unit tests)
- `crates/rnitype-app/src/main.rs` — `hud_style` / `get_config` / `save_config`, หน้าต่าง settings, `--settings`, แปลง pivot เป็นพิกัดหน้าต่าง
- เอกสาร: [[ipc]] v1.1, `config.schema.json`, [[HANDOFF]], [[README]]

## หลักฐานตรวจรับ (2026-11-15)

| หลักฐาน | ผล |
| --- | --- |
| `cargo test --workspace` | **90 passed** (app 6 + core 50 + win 34) |
| `node tools/contracts/validate.mjs` | 10/10 |
| `node tools/tests/hud-smoke.mjs` | 54/54 (ม็อกอัปไม่พัง) |
| `node tools/tests/hud-typography.mjs` | **24/24** (ขนาดขั้นต่ำ, ไม่ชนกัน, contrast 6.87:1, text scale แยกจาก ring) |
| screenshot | `05-mockup/hud-shots/` — typing 100%/150% DPI, วงคำสั่ง, Settings, overlay จริงบนเดสก์ท็อป 2 จอ |

## ความเสี่ยง

| ความเสี่ยง | ระดับ | ทางรับมือ |
| --- | --- | --- |
| ป้ายกลุ่มไทย 7–8 ตัวล้นช่องที่ 45° | กลาง | len-m/len-l ลดขนาด + assert "ไม่ชนกัน" ใน test; ถ้าต้องใหญ่กว่านี้ต้องขยายวงในหรือย่อป้ายที่ต้นทาง |
| ผู้ใช้ขยายเฉพาะ text scale จนตัวอักษรล้นวง | ต่ำ | เพดาน 2.0 + ตรวจด้วยตา 1 คลิก (Settings preview) |
| ฟอนต์ไม่โหลด (ไฟล์หาย) → ไทยมีหัว | ต่ำ | Noto Sans Thai เป็น fallback ในทุก stack; test ตรวจ `document.fonts.check` |
| หน้าต่าง Settings เปิดค้างหลายจอ | ต่ำ | `open_settings` ใช้หน้าต่างเดิม (show + focus) |
