---
title: Visual language — ทันสมัยแบบ Apple/macOS สำหรับ RniType
status: accepted v0.1 — implemented (ADR-22)
updated: 2026-11-16
tags: [design, ux, ui, apple, tokens]
---

# 02.5 — Visual language (Apple/macOS-inspired)

> สรุปจากคำปรึกษา Claude Fable 5.2 (2026-11-16) — คำตอบดิบอยู่ที่ [[05a-claude-ux-consult]]
> เอกสารนี้คือ "สิ่งที่เราจะใช้จริง" ไม่ใช่ลิสต์ทั้งหมดของ Claude
> สิ่งที่ลงโค้ดแล้ว: [[22-visual-refresh-apple-style]] (tokens, HUD plate, active state, pill, Settings sidebar)

## 1. หลักที่ยึด

1. **ลำดับชั้นด้วยความสว่าง ไม่ใช่เส้นขอบ** — `bg → surface → elevated` ต่างกันทีละขั้น + hairline 1px
2. **Accent เดียว ใช้น้อย** — สีสด = "สิ่งที่กำลังเลือก" เท่านั้น (active wedge / focus / slider)
3. **หนึ่งโหมด = หนึ่งสี หนึ่งรูปทรง หนึ่งป้าย** — ลด mode error ซึ่งกิน error budget 1.5% มากที่สุด
4. **Motion สั้นแบบ ease-out** — 80/140 ms; highlight ของ wedge ต้อง 0 ms
5. **HUD ต้องอ่านได้บนพื้นหลังใดก็ได้** — แผ่นทึบ 88% + ขอบสองชั้น (ขาวใน/ดำนอก)
6. **Settings ต้องดูเป็นผลิตภัณฑ์ ไม่ใช่เครื่องมือ dev** — sidebar 200px + inset group + keymap แบบวง ไม่ใช่ตาราง 64 ช่อง
7. **ไม่ลอกสิ่งที่เป็น macOS-only** — ไม่มี traffic lights, ไม่มี SF font, ไม่มี Mica บน HUD, ไม่มี glow/RGB

## 2. Tokens

### 2.1 Settings (dark)

```css
--bg: #1C1C1E;
--surface: #2C2C2E;
--surface-elevated: #3A3A3C;
--separator: rgba(255,255,255,.10);
--text-1: rgba(255,255,255,.92);
--text-2: rgba(235,235,245,.62);
--text-3: rgba(235,235,245,.38); /* disabled/placeholder เท่านั้น */
--accent: #0A84FF;               /* fill */
--accent-text: #409CFF;          /* ข้อความ/ลิงก์ ≈ 6:1 บน --bg */
--success: #30D158;
--warning: #FFD60A;
--danger: #FF453A;
```

### 2.2 HUD (ต้องอ่านบนพื้นหลังใดก็ได้)

```css
--hud-plate: rgba(18,18,20,.88);
--hud-plate-idle: rgba(18,18,20,.70);
--hud-stroke-in: rgba(255,255,255,.16);
--hud-stroke-out: rgba(0,0,0,.55);
--hud-text: #FFF;
--hud-text-dim: rgba(255,255,255,.62);
--hud-active-fill: rgba(10,132,255,.24);
--mode-type: #0A84FF;
--mode-mouse: #FF9F0A;
--mode-nav: #BF5AF2;
--mode-caret: #64D2FF;
```

- ห้ามใช้ `backdrop-filter` บน HUD — WebView เบลอได้แค่เนื้อหาในตัวเอง ไม่ได้เบลอเกม/แอปด้านหลัง
- Mica/Acrylic ใช้ได้เฉพาะหน้าต่าง Settings (Tauri `windowEffects`) และต้อง fallback เป็นสีทึบบน Win10

### 2.3 Scale / motion

```css
--r-xs: 6px; --r-sm: 10px; --r-md: 14px; --r-lg: 20px; --r-pill: 999px;
--s-1: 4px; --s-2: 8px; --s-3: 12px; --s-4: 16px; --s-5: 20px; --s-6: 24px; --s-8: 32px;
--e-1: 0 1px 2px rgba(0,0,0,.30);
--e-2: 0 8px 24px rgba(0,0,0,.40), 0 0 0 1px rgba(255,255,255,.06);
--e-hud: 0 4px 16px rgba(0,0,0,.45);
--t-fast: 80ms; --t-base: 140ms; --t-slow: 220ms;
--ease-out: cubic-bezier(.2,0,0,1);
--ease-inout: cubic-bezier(.4,0,.2,1);
```

## 3. HUD

- **แผ่นเดียวต่อเนื่อง**: disc/plate ชิ้นเดียว + คั่น wedge ด้วย hairline `rgba(255,255,255,.08)` (ไม่เว้นช่องโปร่ง)
- **Active wedge** (ไม่ scale wedge):
  - fill `--hud-active-fill` + arc accent 3px ที่ขอบนอก
  - wedge อื่นลด opacity glyph เหลือ .45
  - ถ้าอยากให้ glyph เด่น → `transform: scale(1.12)` ที่ **ตัว glyph เท่านั้น**
- **ตัวอักษรที่ RS ชี้**: วงกลมทึบ accent 34–36px หลัง glyph, ตัวอักษรขาว 600, ไม่มี transition
- **Hub กลางวง**: ตัวที่จะพิมพ์ 40–44px เป็นจุดพักสายตา
- **Pill**:
  - สูง 28px · radius 14px · padding 0 12px · ตัวอักษร 12px/600
  - เนื้อหา: จุดสีโหมด 8px · ชื่อโหมด · ภาษา · badge Shift/Layer
  - idle เกิน 3 วิ → `--hud-plate-idle` และ opacity .6
  - Shift lock ต้องมี badge ค้างตลอด (toggle ที่มองไม่เห็น = error)
- **Crosshair (Mouse/Nav)**: กากบาท 24px เส้น 1.5px ขาว + outline ดำ 1px + จุดกลาง 4px สีโหมด
- **Typography**: glyph 17px × text scale 1.4 ≈ 24px/500 · ตัวเลข `tabular-nums` · สระลอยบน `◌` opacity .5
- **ความหนาเส้น**: ขอบวง 1px สองชั้น + active arc 3px เท่านั้น
- **Motion**: HUD 80ms · เปลี่ยนโหมด 140ms · commit flash 120ms · highlight 0ms
- **Reduced motion**: ปิด transition/animation ทั้งหมด

## 4. Settings

- **Sidebar 200px** (ไม่ใช่ segmented/top tabs) — หน้าต่าง 760×560, ขั้นต่ำ 680×480, แถว 32px, มีไอคอน + ป้าย
- **Form**: inset group บน `--surface` radius 10px, แถว 44px, label ซ้าย/control ขวา, 13–14px, hint 12px `--text-2`
- **ทุกค่ามีผลทันที** — ไม่มีปุ่ม Save
- **Keymap**: วงจำลอง ~280px (ซ้าย) + 8 ช่องของกลุ่ม (ขวา) ตามทิศ RS (กริด 3×3 เว้นกลาง) + segmented `Base | Layer 2 | Shift` + จุด accent 6px ที่ช่องต่างจาก default + Reset group
- **Reset**: "Reset this page" / "Reset everything" + dialog บอกสิ่งที่หาย + ปุ่ม danger ไม่เป็น default + backup `settings.backup.json` + toast Undo 8 วิ
- **Empty state**: "No controller connected" ใน General + สถานะสด; Languages มีปุ่ม Add language
- **จอยคุม Settings ได้**: D-pad เดิน focus, A = Enter, B = Esc (ใช้ focus management ชุดเดียวกับคีย์บอร์ด)

## 5. Quick bar / Caret nav (สำหรับ P2 slice 4)

- **Quick bar** (LB ค้าง):
  - เปิดหลังค้าง 150ms (การแตะ LB ยังเป็น Ctrl chord ได้)
  - แถบแนวนอน 8 ช่อง 44×44 ห่าง 4px radius 10px + ป้าย 11px
  - วงพิมพ์ dim .25; ปล่อย RS = สั่ง; ปล่อย LB โดยไม่เลือก = ยกเลิก
  - flash 120ms เมื่อสำเร็จ
- **Caret nav** (R3 toggle):
  - ซ่อนวงพิมพ์ เหลือ indicator กากบาท/D-pad ~120px สี `--mode-caret`
  - pill "CARET" + glyph R3 + คำใบ้ "R3 to exit"
  - มีป้ายสั้นในแต่ละแขน (LS = ตัวอักษร/บรรทัด · RS = คำ/ต้น-ท้ายบรรทัด)
  - ไม่มี auto-timeout

## 6. Accessibility / DPI

- Contrast: HUD หลัก ≥ 7:1 · รอง ≥ 4.5:1 · Settings ≥ 4.5:1
- ไม่พึ่งสีอย่างเดียว — โหมดต่างกันทั้งสี รูปทรง ข้อความ
- `:focus-visible { outline: 2px solid var(--accent-text); outline-offset: 2px; }`
- Keyboard + controller focus management ชุดเดียวกัน; `forced-colors` ต้องยังเห็นขอบ/focus
- ใช้ CSS px; วงเป็น SVG; เก็บ pivot เป็น monitor id + สัดส่วน 0–1 (อนาคต)
- ขนาดขั้นต่ำ: glyph ≥ 16px ที่ scale 1.0 · pill ≥ 12px · Settings ≥ 13px · เป้าคลิก ≥ 32×32
- ไทย: line-height ≥ 1.5 · ห้าม letter-spacing ติดลบ · padding-top 2px กันวรรณยุกต์ซ้อน · ห้ามตัดคำกลางคำ

## 7. สิ่งที่เราไม่รับมา

| ข้อเสนอ | เหตุผลที่ไม่ทำ |
| --- | --- |
| ลอก traffic lights / SF font | ผิดแพลตฟอร์มและผิดไลเซนส์ |
| Vibrancy/blur บน HUD | WebView เบลอได้แค่ในตัวเอง + เปลือง GPU; ใช้แผ่น .88 |
| หน้าต่าง HUD 420×420 | overlay ปัจจุบันเต็ม virtual desktop และคลิกทะลุได้; การย่อหน้าต่างเป็นงาน architecture แยก |
| เปลี่ยน pivot เป็น monitor id + ratio ทั้งหมด | ต้องมี geometry ต่อจอ; ทำใน P4/P2 รอบ pivot clamp |
| ย้าย Settings เป็น sidebar ทันทีทั้ง 4 แท็บ | แท็บ General/Languages/Keymap ยังไม่มี backend; ทำ sidebar + HUD tab ก่อน |

## 8. ลำดับลงมือ (Top 7 ของ Claude → งานจริง)

1. Tokens ชุดเดียวใน `ui/shared/tokens.css` (Settings + HUD + mode)
2. HUD plate .88 + ขอบสองชั้น (เอา `backdrop-filter` ออก)
3. Active state: fill + arc 3px + dim ตัวอื่น + glyph scale 1.12 (0ms highlight)
4. ระบบโหมด: สี/รูปทรง/ป้าย + Shift badge
5. Pill ใหม่ + idle state
6. Settings sidebar + inset group + preview ใหม่ (Keymap เต็มรูปแบบรอ P5)
7. Motion 80/140ms + reduced-motion + Reset แบบ Undo

> หมายเหตุ: ข้อ 6 เต็มรูปแบบ (keymap วง + 8 ช่อง) เป็นงาน P5 — รอบนี้ทำ sidebar + inset group + HUD tab ให้ดูเป็นผลิตภัณฑ์ก่อน
