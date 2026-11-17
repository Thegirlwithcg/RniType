---
title: ADR-32 — Fast mode (Ticket 02): LS = หมวด, D-pad = ตัวอักษร, A = พิมพ์
status: accepted (2026-11-17 · ผ่าน feedback 5 รอบ)
updated: 2026-11-17
tags: [decision, core, typing, hud, fast-mode]
---

# ADR-32 — Fast mode (Ticket 02)

## บริบท

Ticket 02 (`Ticket02.canvas`) ขอปรับการพิมพ์ให้เร็วและต่อเนื่องขึ้น 3 ข้อ
และขอผลลัพธ์เป็นตัวติดตั้งหลัง smoke เฉพาะ feature ที่แก้

1. **Feature 1** — LS เลือกกลุ่มวงในก่อนแล้วล็อกไว้ จากนั้น LS ตัวเดิมเลือกตัวอักษร แล้ว **A = พิมพ์** · โหมดนี้ A ต้อง accept D-pad ได้ด้วย
2. **Feature 2** — HUD อยู่ตำแหน่งเดียวกับ caret ที่กำลังพิมพ์ และ **ไม่มีจุดฟ้า/มาร์กเกอร์**
3. **Feature 3** — RS เป็นปุ่มทิศทาง และ **R3 สองครั้งติดกันแล้วเลื่อนทิศ = highlight/เลือกหลายตัว** (owner ยืนยัน "ใช่ Analog ขวา" 2026-11-17)

ของเดิม: LS = กลุ่ม · RS = ตัวอักษร · ปล่อย RS = commit — ต้องสลับนิ้ว LS ↔ RS ทุกตัวอักษร

หลังทดลองจริง owner ให้ feedback 5 รอบ การตัดสินใจสุดท้ายจึงต่างจาก ticket ข้อ 1
(ดู §วิวัฒนาการ) — **ADR นี้บันทึกสถานะที่รับจริงใน 0.2.5**

## การตัดสินใจ (final)

| เรื่อง | ค่าที่ใช้ | เหตุผล |
| --- | --- | --- |
| หมวด (วงใน) | **LS = หมวด เปลี่ยนได้ตลอดเหมือน Mode ธรรมดา** (ไม่มีล็อก) | feedback รอบ 5: ล็อกแล้วเปลี่ยนหมวดไม่ได้ รู้สึกติด |
| เปลี่ยนหมวด | ตั้ง `sector` ตามทิศ LS ให้กด A ต่อได้ทันที · D-pad ปรับต่อได้อิสระ | พิมพ์ต่อเนื่องโดยไม่ต้องเลือกซ้ำ |
| ตัวอักษร (วงนอก) | **D-pad** `↑` = ถอย · `↓`/`→` = ไปข้างหน้า | นิ้วโป้งขวาไม่ต้องแตะ RS |
| พิมพ์ | **A = commit ช่องที่ไฮไลต์** · กด A ซ้ำ = พิมพ์ตัวเดิม (ไม่มี auto-repeat) | A ไม่เป็น Space ในโหมดนี้ |
| `D-pad ←` | **เปิด/ปิดวงคำสั่ง (เมนูโหมด)** เหมือนโหมดปกติ | feedback รอบ 3 |
| วงคำสั่ง | **D-pad ↑/↓/→ เลื่อนไฮไลต์ + A ยืนยัน** · B ยกเลิก · RS ยังใช้ได้ | feedback รอบ 5 |
| L3 | ล้างช่องที่เลือก (หมวดเดิมยังอยู่) | ทางออกเมื่อเลือกช่องผิด |
| B | **ล้างไฮไลต์ก่อน** · ไม่มีไฮไลต์จึงเป็น Backspace | กันลบโดยไม่ตั้งใจ |
| X / Y | Enter / Tab (ค้าง = Esc) | ไม่เปลี่ยน |
| RS | ลูกศร 4 ทิศ (repeat 350/90 ms) | Feature 3 |
| R3 ×2 | สลับโหมดเลือกข้อความ → ลูกศรมี Shift | Feature 3 · double-tap 280 ms เดิม |
| Feature 2 | pivot ของ HUD = caret ของหน้าต่างที่ focus (`GetGUIThreadInfo` + `ClientToScreen`) ทุก 250 ms · ไม่เจอ caret = คง pivot เดิม | "UI focus ที่ตำแหน่งเดียวกับ Cursor" |
| ไม่มีมาร์กเกอร์ | โหมดพิมพ์ไม่วาดจุด pivot (จุดมีเฉพาะ Nav) | Feature 2 |
| วงโชว์เมื่อไหร่ | `fast_rings = fast && (RS กำลังชี้ทิศ หรือ โหมดเลือก หรือ group.is_some())` · วงค้างหลังเลือกหมวด | feedback รอบ 2–3: ปล่อย RS แล้ววงหาย |
| pill | `data-fast="on"` → pill ย่อ (20px/10px) · คำใบ้ `LS หมวด · D-pad เลือก` (จังหวะกลุ่ม) และ `A พิมพ์ · D-pad เลือกช่อง` (จังหวะเลือกช่อง) | feedback รอบ 2 |
| ค่าเริ่มต้น | `defaults.typing.fastMode = true` + toggle Settings → General | ทดสอบได้ทันทีหลังติดตั้ง |
| สัญญา | `HudState.fast { on, phase, select }` เป็น **optional** | state/config เก่าใช้ได้ |

## วิวัฒนาการ (feedback 5 รอบ)

| รอบ | ที่ owner แจ้ง | ที่แก้ |
| --- | --- | --- |
| 1 | ticket แรก | LS ล็อกกลุ่ม → LS ตัวอักษร → A พิมพ์ (0.2.0) |
| 2 | วงล้อไม่โชว์ตอนกด RS · RU โผล่ · pill ใหญ่ | คำนวณ `state` เองจาก Fast mode · `is_retired_language("ru")` · ย่อ pill (0.2.1) |
| 3 | ปล่อย RS แล้ววงกลับ+UI หาย · อยากได้เมนูโหมด D-pad | วงค้างหลังเลือกกลุ่ม · `D-pad ←` = วงคำสั่ง (0.2.2) |
| 4 | วงค้างแล้วเปลี่ยนหมวดไม่ได้ · RU ค้างใน Win+Space | L3 = กลับไปเลือกหมวด · `build_keymap` unload layout ที่โหลด (0.2.3–0.2.4) |
| 5 | อยากให้เปลี่ยนหมวดเหมือน Mode ธรรมดา · D-pad เลือกเมนูได้ | **ยกเลิกการล็อกหมวด** · D-pad = ช่องตัวอักษร · วงคำสั่งรับ D-pad + A (0.2.5) |

## ทางเลือกที่ไม่เลือก

- **ล็อกหมวดด้วยการปล่อย LS (ticket ข้อ 1 แบบตรงตัว)** — ทดลองแล้วเปลี่ยนหมวดยาก owner จึงสั่งให้ถอย (รอบ 5)
- **ล็อกหมวดด้วย dwell 300–400 ms** — ช้ากว่าเป้าหมาย "เร็วขึ้น"
- **RS = ตัวอักษรเหมือนเดิม** — ซ้ำกับ Feature 3 ที่ RS ต้องเป็นลูกศร
- **auto-repeat ด้วย LS ค้าง** — ตัดออกเมื่อ LS กลับไปทำหน้าที่เลือกหมวด
- **ย้าย HUD ตาม caret ทุกเฟรม (250 Hz)** — เปลือง CPU และ caret กระตุกกว่า
- **วาดจุด pivot จาง ๆ ใน Fast mode** — ticket ห้ามมาร์กเกอร์

## ผลข้างเคียงที่ยอมรับ

1. เปลี่ยนหมวด = ไฮไลต์ช่องกระโดดตามทิศ LS — ต้องกด D-pad ต่อถ้าต้องการช่องอื่น (ตั้งใจให้เร็วที่สุดสำหรับช่องทิศเดียวกัน)
2. **B ครั้งแรกไม่ลบ** เมื่อมีไฮไลต์ — HUD ต้องเห็นไฮไลต์ชัด
3. `D-pad ←` ถูกใช้เป็นเมนูโหมด จึงใช้เลื่อนช่องไม่ได้ (ใช้ ↑/↓/→ แทน)
4. Feature 2 ใช้ได้เฉพาะแอปที่ประกาศ caret (Notepad/Word/browser) — เกม/custom UI คง pivot เดิม
5. pivot ที่ได้ยังผ่าน clamp เดิม (margin 172 × hudScale) จึงอาจเยื้องจาก caret ที่ขอบจอ
6. RU ถูกถอดจาก UI/runtime/config (ยังเก็บ keymap ไว้ใน core สำหรับ test universal layout)

## ไฟล์ที่เกี่ยวข้อง

| ชั้น | ไฟล์ |
| --- | --- |
| core | `gesture.rs` (`FastGesture`), `typing.rs` (`tick_fast`), `runtime.rs` (`tick_fast`, `tick_fast_arrows`, `tick_wheel` + D-pad) |
| Windows | `rnitype-win/src/caret.rs`, `language.rs` (unload layout) |
| app | `config.rs` (`fast_mode`, retired RU), `session.rs` (`HudState.fast`, `fast_rings`), `main.rs` (เกาะ caret), `input.rs` (`fast_typing_script`) |
| HUD | `ui/hud/hud.js` (badge `Fast`/`Select`, `data-fast`), `hud.css` (pill ย่อ) |
| Settings | `ui/settings/index.html` + `settings.js` (toggle) |
| contract | `hud-state.schema.json` (+sample `type-th/char`), `config.schema.json` |

## การตรวจ (0.2.5)

- `cargo test --workspace` — **235 passed** (core 110 · win 58 · app 67)
- contracts **13/13** · hud-smoke 54/54 · typography 36/36 · settings-smoke 35/35
- `--fake-pad` (Fast mode เปิด) พิมพ์จริง · `GetKeyboardLayoutList` เหลือ EN + TH
- ยังไม่ทดสอบกับจอยจริง — งานของ owner (ดู [[Handoff Feature]])

## อ้างอิง

[[Ticket02]] · [[Handoff Feature]] · [[04-state-machine-spec]] · [[02-keymap-en-th]] · [[23-quickbar-caret-sticky-drag-pivot-clamp]] · [[28-universal-qwerty-mapping]]
