---
title: Claude UX consult — Apple/macOS direction (raw)
status: reference
updated: 2026-11-16
tags: [ux, ui, research, claude]
---

> ที่มา: Claude Fable 5.2 (Claude Code CLI 2.1.325, `--model fable --effort medium`) · 2026-11-16
> บรีฟเป็นแบบ self-contained (ไม่ได้ให้เข้าถึงโค้ด) — ตัวเลขเป็นข้อเสนอเริ่มต้น ต้องจูนกับจอจริง
> บันทึกดิบไว้เป็นแหล่งอ้างอิง; ข้อสรุปที่เลือกใช้อยู่ใน [[05-visual-language]]

> ⚠️ แจ้งก่อน: claude-mem บันทึกความจำไม่ได้ตอนนี้ ("Claude Code reported the observer as signed out", ถูกปฏิเสธ credentials ตั้งแต่ 2026-11-15T06:41Z) แก้ได้ด้วยการรัน `/login` ใน Claude Code หรือ `claude auth login` ส่วนการ restart ไม่ช่วย

คำตอบนี้อิงจากคำอธิบายในข้อความเท่านั้น ผมยังไม่เห็นโค้ดหรือหน้าจอจริง ตัวเลขทั้งหมดเป็นค่าเริ่มต้นที่ควรจูนบนเครื่องจริง

## 1. Visual language

**ยืม**
- **ลำดับชั้นด้วยความสว่าง ไม่ใช่เส้นขอบ**: พื้นหลัง → surface → elevated ต่างกันทีละขั้น คั่นด้วย hairline 1px ทำให้ดูสะอาดโดยไม่เพิ่มสิ่งที่ต้องอ่าน
- **Accent สีเดียว ใช้น้อย**: สีสดสงวนไว้ให้ "สิ่งที่กำลังเลือก" เท่านั้น ตาจึงหา active state เจอทันที ซึ่งส่งผลตรงกับ WPM
- **Radius ซ้อนกันแบบ concentric**: radius ชั้นใน = radius ชั้นนอก − padding
- **Spacing บนกริด 4/8** และตัวหนังสือน้ำหนัก 400/500/600 เท่านั้น
- **Motion สั้นแบบ ease-out** ไม่เด้ง

**ไม่ยืม**
- **Traffic lights**: ผู้ใช้ Windows คาดหวังปุ่ม caption ด้านขวา ใช้ title bar ของ Windows ตามปกติ
- **ฟอนต์ SF**: ไลเซนส์ไม่อนุญาตให้ใช้นอกแพลตฟอร์ม Apple ใช้ `"Segoe UI Variable Text", "Segoe UI"` สำหรับ EN และ IBM Plex Sans Thai สำหรับไทย
- **Vibrancy/blur บน HUD**: `backdrop-filter` ใน WebView โปร่งใสเบลอได้แค่เนื้อหาใน webview ไม่เบลอเกมหรือแอปด้านหลัง จึงได้แค่ภาระ GPU ให้ใช้แผ่นทึบ 85–90% แทน
- **Mica/Acrylic**: ใช้ได้เฉพาะหน้าต่าง Settings ผ่าน `windowEffects` ของ Tauri v2 บน Win11 และ fallback เป็นสีทึบบน Win10
- **Glow/RGB แบบเกมมิ่ง**: ลด contrast ของตัวอักษรและดูถูกลง ความเป็นเกมมิ่งให้มาจาก glyph ปุ่มจอย (LB/RT/R3) ที่วาดดีก็พอ

## 2. Design tokens (dark)

```css
:root {
  /* Settings */
  --bg: #1C1C1E;
  --surface: #2C2C2E;
  --surface-elevated: #3A3A3C;
  --separator: rgba(255,255,255,.10);
  --text-1: rgba(255,255,255,.92);
  --text-2: rgba(235,235,245,.62);
  --text-3: rgba(235,235,245,.38);   /* disabled/placeholder เท่านั้น */
  --accent: #0A84FF;                 /* fill */
  --accent-text: #409CFF;            /* ข้อความ/ลิงก์บน --bg ≈ 6:1 */
  --success: #30D158; --warning: #FFD60A; --danger: #FF453A;

  /* HUD: ต้องอ่านได้บนพื้นหลังใดก็ได้ */
  --hud-plate: rgba(18,18,20,.88);
  --hud-plate-idle: rgba(18,18,20,.70);
  --hud-stroke-in: rgba(255,255,255,.16);   /* เห็นบนพื้นมืด */
  --hud-stroke-out: rgba(0,0,0,.55);        /* เห็นบนพื้นสว่าง */
  --hud-text: #FFF;
  --hud-text-dim: rgba(255,255,255,.62);
  --hud-active-fill: rgba(10,132,255,.24);
  --mode-type: #0A84FF; --mode-mouse: #FF9F0A;
  --mode-nav: #BF5AF2;  --mode-caret: #64D2FF;

  --r-xs: 6px; --r-sm: 10px; --r-md: 14px; --r-lg: 20px; --r-pill: 999px;
  --s-1: 4px; --s-2: 8px; --s-3: 12px; --s-4: 16px; --s-5: 20px; --s-6: 24px; --s-8: 32px;

  --e-1: 0 1px 2px rgba(0,0,0,.30);
  --e-2: 0 8px 24px rgba(0,0,0,.40), 0 0 0 1px rgba(255,255,255,.06);
  --e-hud: 0 4px 16px rgba(0,0,0,.45);

  --t-fast: 80ms; --t-base: 140ms; --t-slow: 220ms;  /* slow ใช้ใน Settings เท่านั้น */
  --ease-out: cubic-bezier(.2,0,0,1);
  --ease-inout: cubic-bezier(.4,0,.2,1);
}
```

- **Contrast ของ HUD**: แผ่น .88 บนพื้นขาวล้วนยังให้ตัวอักษรขาวราว 13:1 และตัวรอง (.62) ราว 6:1 ถ้าลดต่ำกว่า .80 ตัวรองจะเริ่มอ่านยากบนเอกสารขาว จึงไม่ควรลด
- **ขอบสองชั้น**: เส้นในขาวกับเส้นนอกดำทำให้ขอบ HUD ชัดทั้งบนเกมมืดและเว็บขาว
- **Accent กับตัวอักษรเล็ก**: ขาวบน `#0A84FF` ได้ราว 3.6:1 ผ่านเฉพาะตัวใหญ่ (≥ 24px) ใช้กับ glyph ใน HUD ได้ แต่ห้ามใช้กับข้อความ 13px

## 3. Settings window

- **ใช้ Sidebar ไม่ใช้ segmented control**: กว้าง 200px หน้าต่าง 760×560 (ขั้นต่ำ 680×480) แถวละ 32px มีไอคอนและป้าย เหตุผลคือป้ายไทยยาวกว่าอังกฤษและวันหนึ่งจะมีแท็บที่ 5
- **Form**: จัดเป็นกลุ่มแบบ inset บน `--surface` radius 10px แถวสูง 44px ป้ายอยู่ซ้าย control อยู่ขวา ตัวอักษร 13–14px คำอธิบายรอง 12px สี `--text-2` ทุกค่ามีผลทันที ไม่มีปุ่ม Save
- **Keymap**: ไม่ควรทำเป็นตาราง 64 ช่อง
  - ซ้าย: วงจำลองแบบเดียวกับ HUD (ราว 280px) คลิกหรือโฟกัสเพื่อเลือกกลุ่ม
  - ขวา: 8 ช่องของกลุ่มนั้น วางตามทิศ RS จริง (กริด 3×3 เว้นกลาง) ช่องละ 56px
  - บน: segmented `Base | Layer 2 | Shift` และตัวเลือกภาษา
  - ช่องที่ต่างจาก default มีจุด accent 6px และแต่ละกลุ่มมีปุ่ม "Reset group"
- **Reset**:
  - แยกขอบเขตเป็น "Reset this page" กับ "Reset everything" (อยู่ล่างสุดของ General)
  - dialog บอกชัดว่าอะไรจะหาย ปุ่ม danger ไม่เป็น default และ Esc/B คือยกเลิก
  - เขียน `settings.backup.json` ก่อน reset แล้วแสดง toast "Undo" 8 วินาที ซึ่งปลอดภัยกว่า dialog ซ้อนสองชั้น
- **Empty state**: ตัวที่สำคัญที่สุดคือ "No controller connected" ใน General (ไอคอน หนึ่งประโยค และสถานะสดเมื่อเสียบ) ส่วน Languages ที่มีภาษาเดียวให้แสดงปุ่ม "Add language" พร้อมคำอธิบายหนึ่งบรรทัด
- **ไทย/อังกฤษ**: เก็บ string ไว้ใน dictionary เดียว ส่ง EN ก่อนแล้วตาม TH ทีหลัง layout ต้องไม่ fix ความสูงแถว (`min-height` ไม่ใช่ `height`) และใช้ line-height 1.5
- **ใช้จอยคุม Settings ได้**: ผู้ใช้ถือจอยอยู่แล้ว ให้ map D-pad เป็นการย้ายโฟกัส A เป็น Enter และ B เป็น Esc ทั้งหมดใช้ focus management ชุดเดียวกับคีย์บอร์ด

## 4. HUD

- **แผ่นเดียวต่อเนื่อง**: ใช้ disc `--hud-plate` ชิ้นเดียว คั่น wedge ด้วย hairline `rgba(255,255,255,.08)` ถ้าเว้นช่องโปร่งระหว่าง wedge พื้นหลังจะโผล่มารบกวน
- **Active wedge** (ไม่ scale wedge):
  - เติม `--hud-active-fill` และเพิ่ม arc สี accent หนา 3px ที่ขอบนอก
  - wedge อื่นลด opacity ของ glyph เหลือ .45
  - ถ้าอยากให้ "เด้ง" ให้ `transform: scale(1.12)` ที่ตัว glyph เท่านั้น ไม่ทับกันและวิ่งบน compositor
- **ตัวอักษรที่ RS ชี้**: วงกลมทึบสี accent 34–36px หลัง glyph ตัวอักษรขาวน้ำหนัก 600 เปลี่ยนทันทีโดยไม่มี transition
- **Hub กลางวง**: แสดงตัวที่จะพิมพ์ขนาด 40–44px เพื่อให้สายตามีจุดพักจุดเดียว
- **Typography**:
  - glyph ในวง 17px × text scale 1.4 ≈ 24px น้ำหนัก 500
  - ตัวเลขใช้ `font-variant-numeric: tabular-nums`
  - สระและวรรณยุกต์ลอยให้แสดงบน `◌` (U+25CC) ที่ opacity .5 ไม่อย่างนั้นจะลอยและหาไม่เจอ
- **ความหนาเส้น**: ขอบวง 1px สองชั้น และ active arc 3px ไม่ควรมีเส้นหนาอื่นอีก
- **Pill**: สูง 28px radius 14px padding 0 12px ตัวอักษร 12px/600 เนื้อหาคือจุดสีโหมด 8px, ชื่อโหมด, ภาษา และ badge ของ Shift/Layer เมื่อ idle เกิน 3 วินาทีให้เปลี่ยนเป็น `--hud-plate-idle` และลด opacity เหลือ .6
- **Shift ที่ล็อกจาก LT**: ต้องมี badge ค้างบน pill ตลอด เพราะสถานะ toggle ที่มองไม่เห็นคือแหล่ง error อันดับหนึ่ง
- **Mouse/Nav crosshair**: กากบาท 24px เส้น 1.5px สีขาวพร้อม outline ดำ 1px และจุดกลาง 4px สีโหมด ไม่ควรใช้ `mix-blend-mode: difference` เพราะกะพริบบนวิดีโอและเกม

## 5. Quick bar (LB ค้าง) และ Caret nav (R3)

หลักกันสับสนคือ หนึ่งโหมดมีหนึ่งสี หนึ่งรูปทรง และหนึ่งป้ายบน pill

**Quick bar** (โหมดแบบกดค้าง ปล่อยแล้วกลับเอง)
- แสดงหลัง LB ค้าง 150ms เพื่อให้การแตะ LB เป็น Ctrl chord ไม่ทำให้ UI กะพริบ
- รูปทรงเป็นแถบแนวนอน ไม่ใช่วง: 8 ช่อง ช่องละ 44×44 ห่างกัน 4px radius 10px มีไอคอนและป้าย 11px
- วงพิมพ์ dim เหลือ .25 ทันทีระหว่างค้าง
- RS ชี้เพื่อเลือก ปล่อย RS เพื่อสั่ง และปล่อย LB โดยไม่เลือกคือยกเลิก
- คำสั่งสำเร็จให้ช่อง flash 120ms ถ้าเป็น 8 ทิศเหมือนวง ให้มีลูกศรทิศเล็กกำกับแต่ละช่อง

**Caret nav** (โหมดแบบ toggle เสี่ยงกว่า จึงต้องชัดกว่า)
- ซ่อนวงพิมพ์ทั้งหมด เหลือ indicator รูปกากบาท/D-pad ขนาดราว 120px สี `--mode-caret`
- pill เขียน "CARET" พร้อม glyph R3
- มีป้ายสั้นในแต่ละแขน เช่น LS = ตัวอักษร/บรรทัด และ RS = คำ/ต้น-ท้ายบรรทัด
- ออกด้วย R3 ซ้ำหรือ B และมีคำใบ้ "R3 to exit" บน pill เสมอ
- ไม่ควรมี auto-timeout เพราะโหมดที่หายเองทำให้ผู้ใช้เดาสถานะไม่ได้

## 6. Motion

**ควรมี**
- HUD โผล่และหาย: opacity 80ms ease-out
- เปลี่ยนโหมด: crossfade 140ms
- Commit ตัวอักษร: flash ที่ hub 120ms โดยใช้ opacity บน layer แยก
- Pill เข้า idle: 220ms

**ต้องเป็น 0ms**
- Highlight ของ wedge และตัวอักษรที่ตามคันโยก: ถ้าใส่ easing จะรู้สึกหน่วงและ error rate ขึ้น

**ไม่ควรมี**
- spring/bounce, วงหมุน, stagger
- animate `box-shadow`, `filter`, `backdrop-filter`, ขนาด หรือตำแหน่งแบบ layout

**Latency < 16ms**
- เขียน DOM ครั้งเดียวต่อ event โดยสลับ `data-group` / `data-slot` ที่ root แล้วให้ CSS จัดการ ไม่ต้องใช้ rAF loop
- ใช้ `will-change: opacity` กับ 2–3 layer เท่านั้น
- หน้าต่าง HUD ควรมีขนาดพอดีวง (ราว 420×420) ไม่เต็มจอ เพราะหน้าต่างโปร่งใสต้อง composite ทั้งผืน
- Rust ส่ง event เฉพาะตอน sector เปลี่ยน

**Reduced motion**
```css
@media (prefers-reduced-motion: reduce) {
  * { transition-duration: 0ms !important; animation: none !important; }
}
```

## 7. Accessibility และ DPI

- **Contrast**: ตัวอักษรหลักบน HUD ≥ 7:1 ตัวรอง ≥ 4.5:1 และ Settings ≥ 4.5:1 ส่วน `--text-3` ห้ามใช้กับข้อมูลที่ต้องอ่าน
- **ไม่พึ่งสีอย่างเดียว**: โหมดต้องต่างกันทั้งสี รูปทรง และข้อความ
- **Focus ring**: `:focus-visible { outline: 2px solid var(--accent-text); outline-offset: 2px; }` ห้าม `outline: none`
- **Keyboard**: Tab ข้ามกลุ่ม ลูกศรเดินใน sidebar และ keymap (roving tabindex) Enter แก้ไข Esc ปิด
- **High Contrast**: ใส่ `@media (forced-colors: active)` ให้ขอบและ focus ยังมองเห็น
- **DPI**:
  - ใช้ CSS px ทั้งหมด ไม่คำนวณ device px เอง
  - วาดวงด้วย SVG เพราะคมทุกสเกล เส้นตรง 1px ที่ 125% จะเบลอเล็กน้อย ซึ่งยอมรับได้
  - เก็บ pivot เป็น monitor id + สัดส่วน 0–1 ไม่ใช่ px สัมบูรณ์ จะไม่กระโดดเมื่อ DPI หรือความละเอียดเปลี่ยน
- **ขนาดขั้นต่ำ**: glyph ในวง ≥ 22px (ที่ scale 1.0 ต้อง ≥ 16px) pill ≥ 12px ข้อความ Settings ≥ 13px และเป้าคลิก ≥ 32×32 ส่วน slider ของ text scale ให้จำกัดที่ 1.0–2.0
- **ภาษาไทย**: line-height ≥ 1.5 ห้าม letter-spacing ติดลบ เผื่อ padding-top 2px กันวรรณยุกต์ซ้อน (เช่น "น้ำ", "ปั้น") โดนตัด และห้าม `text-overflow` กลางคำ

## 8. Top 7 things to implement first

ข้อนี้ตอบทั้งหัวข้อ 8 และ Top 7 ปิดท้าย เพราะเป็นรายการเดียวกัน

1. **ย้ายทุกสีและระยะไปเป็น token ชุดเดียว**: ความ "ดูแพง" มาจากความสม่ำเสมอ และข้ออื่นทั้งหมดสร้างบนนี้
2. **แผ่น HUD ทึบ .88 พร้อมขอบสองชั้น**: อ่านได้บนทุกพื้นหลังทันที และถูกที่สุดต่อเฟรม
3. **Active state ใหม่ (fill + arc 3px + dim ตัวอื่น, 0ms)**: หา target เร็วขึ้นโดยตรง และเลิกปัญหา wedge ทับกัน
4. **ระบบโหมดแบบหนึ่งสี หนึ่งรูปทรง หนึ่งป้าย พร้อม badge ของ Shift lock**: ลด mode error ซึ่งเป็นตัวกิน error budget 1.5% มากที่สุด
5. **Pill ใหม่และ idle state**: เป็นส่วนที่ผู้ใช้เห็นนานที่สุดต่อวัน จึงคุ้มที่สุดต่อความรู้สึกพรีเมียม
6. **Settings แบบ sidebar + inset group + keymap แบบวงและ 8 ช่อง**: เปลี่ยนภาพจากเครื่องมือของนักพัฒนาเป็นผลิตภัณฑ์
7. **Motion ชุดเล็ก (80/140ms) พร้อม reduced-motion และ Reset แบบมี Undo**: เก็บรายละเอียดสุดท้ายโดยไม่แตะ latency ของการเลือก
