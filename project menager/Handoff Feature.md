---
title: Handoff Feature — Fast mode (Ticket 02)
status: beta 0.3.7 — Fast mode v2 + รอบ 7–10 (A หลังปล่อย · ไฮไลต์เคลียร์เฉพาะตอนไม่โยกจอย)
updated: 2026-11-17
tags: [handoff, feature, ticket-02, fast-mode]
---

# Handoff Feature — Fast mode (Ticket 02)

> แหล่งงาน: [[Ticket02]] · เอกสารนี้บันทึกทุกครั้งที่ทำงานถึงไหน (ตาม ticket)
> เวอร์ชันที่ส่งมอบ: **0.3.7** · ติดตั้งได้จาก `dist\RniType_0.3.7_x64-setup.exe` หรือ `dist\RniType-portable-0.3.7.zip`
> เวอร์ชันก่อนหน้า: 0.3.6 · 0.3.5 · 0.3.4 · 0.3.3 · 0.3.2 · 0.3.0 (Fast mode v2 / ADR-33) · 0.2.5 · 0.2.4 · 0.2.3 · 0.2.2 · 0.2.1 · 0.2.0

## 11. รอบ 10 — กลับฐาน 0.3.5: A หลังปล่อย + ไฮไลต์ไม่ค้างเมื่อไม่โยก (0.3.7)

| อาการ | สาเหตุ | ที่แก้ |
| --- | --- | --- |
| 0.3.6 **เลือกอักษรข้ามหมวดไม่ได้** (ปัญหาเก่ากลับมา) | 0.3.6 ล้าง `sector` ทุกครั้งที่กด A + ใส่ `suppress` กันทิศเดิม → หลังพิมพ์ต้องออกทิศ/ออกโซนก่อน ไฮไลต์จึงกลับมา | **ย้อนตรรกะกลับไป 0.3.5** — ไม่ล้างตอนดันค้าง · ไม่มี `suppress` |
| 0.3.5 ปล่อย LS แล้วกด A ไม่พิมพ์ (กรณียังไม่เคยพิมพ์เลย) | `hide()` ล้างช่อง และ `last_commit` ว่างเพราะยังไม่เคยกด A | เพิ่ม **`last_pick`** = ช่องที่ไฮไลต์ล่าสุด → A ใน `Hidden` พิมพ์ได้แม้ยังไม่เคย commit |
| กด A แล้วไฮไลต์ค้างที่ตัวเดิม ตอนไม่โยกจอย | commit ไม่ล้างช่อง | **ถ้า `r < 0.30` (จอยอยู่ศูนย์/ไม่โยก) → ล้าง `sector`** หลังพิมพ์ · ถ้าดันค้างอยู่ = คงไฮไลต์เหมือน 0.3.5 |

**Smoke**: `cargo test --workspace` **271 passed** (core 142 · win 58 · app 71)

**เทสต์ที่ล็อกพฤติกรรม**: `a_clears_the_highlight_only_when_the_stick_is_released` · `a_after_release_prints_a_selection_that_was_never_committed` · `can_change_group_and_pick_another_character_after_printing`

**ไฟล์**: `crates/rnitype-core/src/gesture.rs`

## 10. รอบ 9 — A ต้องพิมพ์ได้ตลอด + UI หายทันที (0.3.5)

| อาการ | สาเหตุ | ที่แก้ |
| --- | --- | --- |
| ปล่อย Analog แล้วกด A ยังไม่พิมพ์ (แม้ 0.3.4 เก็บช่องไว้ 1.5 s) | หลัง `hide()` (1.5 s) ล้าง group/sector → `FastZone::Hidden` ไม่มีอะไรให้ A พิมพ์ | เพิ่ม `last_commit` ใน `FastGesture` — จำ `(group, sector)` ที่ A พิมพ์ล่าสุด · **A ใน `Hidden` = พิมพ์ตัวเดิมซ้ำ** จนกว่าจะเลือกใหม่ |
| UI ค้างหลังปล่อย Analog | ต้องรอ center 120 ms และโหมด Select ทำให้วงไม่หาย | `FAST_CENTER_RESET_MS` **120 → 40 ms** · `fast_rings` ไม่สนใจ `fast_select` (โชว์เฉพาะ LS ในโซน หรือ RS กำลังชี้) |
| A ถูกกลืนในเฟรมเดียวกับลูกศร/B | 1 เฟรมส่งได้ 1 action แล้วทิ้งที่เหลือ | `tick_fast` ใช้คิว action (`fast_queued`): A มาก่อน แล้วค่อย B/X/Y/ลูกศร |

**หลักฐาน**: `cargo test --workspace` **268 passed** (core 139 · win 58 · app 71) · contracts 13/13 · hud 54/54 · typography 36/36 · settings 37/37 · setup 0.3.5 ติดตั้งทับ, PE subsystem = 2 (GUI ไม่มี terminal)

**ไฟล์**: `gesture.rs` (`last_commit`, `FAST_CENTER_RESET_MS`, test `a_keeps_working_after_the_ui_hides`) · `runtime.rs` (คิว action / A repeat) · `session.rs` (`fast_ui_hidden`)

## 0. รอบ 7 — feedback จาก owner (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| **ปล่อย Gamepad แล้ว UI ต้องหายไป** | `FAST_CENTER_RESET_MS` 250 → **120 ms** · `fast_rings` ไม่ค้างเพราะโหมด Select อีก (โชว์เฉพาะตอน LS อยู่ในโซน หรือ RS กำลังชี้ทิศ) · HUD หายทันทีเมื่อปล่อย |
| **Analog ขวา กด 2 ครั้งแล้วเลื่อน = highlight คำ** | คงพฤติกรรม R3×2 = โหมดเลือก (Shift+ลูกศร) · วงล้อโชว์เฉพาะตอนขยับ RS แล้วหายเมื่อปล่อย (ไม่ค้างบังข้อความ) |
| **A ค้าง = Spacebar ค้าง · B ค้าง = Backspace ค้าง** | เพิ่ม auto-repeat ของปุ่มหน้า: หลังค้าง 400 ms → **A ส่ง Space ซ้ำ / Fast mode ส่งตัวอักษรซ้ำ**, **B ส่ง Backspace ซ้ำ** ทุก 90 ms (ยกเลิก Ctrl+Backspace เดิม) |
| **Default ภาษาเริ่มที่ EN เสมอ** | `default_config.typing.mode = "en"` + `build_session` ไม่ activate ภาษาจาก config อีก (สลับได้ด้วยเมนู Lang ตามปกติ) |
| **Fast mode ไม่ต้องแสดง UI กลาง (pill)** | `.hud[data-fast="on"] .hud__status { display: none; }` — ไม่มี pill ใน Fast mode |
| **เปิด RniType ไม่ต้องมีหน้าต่าง terminal** | `installer/build-installer.ps1` build **ไม่ใส่ `--features console`** → `windows_subsystem = "windows"` · ตรวจ PE subsystem = **2 (GUI)** |

## 0.1 รอบ 6 — feedback จาก owner (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| **UI วาดไม่ตรงตำแหน่ง** — วง gauge (เส้นน้ำเงิน) ลอยอยู่ใต้ล้อ | `.fast__value` หมุนด้วย `rotate(-90deg)` โดย default `transform-box: view-box` → Chrome หมุนรอบมุม viewBox ทำให้วงเลื่อนลง ~340px · แก้เป็น **`transform-box: fill-box`** · ตรวจ rect: วงอยู่กลาง stage (442,142) ตรงกับ `fastArc` (430,130) |
| **กลางวงให้เป็นพื้นใส + เอาตัวอักษรกลางออก** | Fast mode: เจาะรูกลาง `.hud__glass` ด้วย `mask: radial-gradient(... transparent 0 28px ...)` + ซ่อน `.hud__center` (จุด/วงกลาง) และ `.hud__preview` (ตัวอักษรกลาง) |
| **D-pad กลับไปใช้พฤติกรรมเดิมของ Mode ปกติ** | `D-pad ←` = เปิด/ปิดวงคำสั่งเหมือนโหมดปกติ (รวมถึงตอนเมนู Fast เปิด) · `D-pad ↑/↓/→` **ไม่** เลือกเมนู/ช่องในโหมด push (ใช้ได้เฉพาะ fallback `letterPick=dpad`) · วงคำสั่งกลับไปเลือกด้วย RS + A ยืนยัน (เอา D-pad stepping ออก) |

## 0.2 รอบ 5 — feedback จาก owner (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| ปล่อย Analog แล้วเลือกหมวด C ไม่ได้ — วงในค้าง “ให้พฤติกรรมเหมือน Mode ธรรมดา” | **ยกเลิกการล็อกหมวด**: LS = เลือกหมวดได้ตลอดเหมือน Mode ธรรมดา · **D-pad = เลือกช่องตัวอักษร** (`↑` ถอย · `↓/→` ไปข้างหน้า) · **A = พิมพ์/accept** · เปลี่ยนหมวดแล้วไฮไลต์ช่องตามทิศ LS ให้กด A ต่อได้ทันที |
| D-pad: อยากเปิด UI ค้างไว้ แล้วใช้ D-pad เลือกเมนู + A เลือก Mode | วงคำสั่งรับ **D-pad ↑/↓/→ เพื่อเลื่อนไฮไลต์** และ **A ยืนยัน** (เดิมต้องใช้ RS) · `D-pad ←` เปิด/ปิดวง · `B` ยกเลิก |

## 0.3 รอบ 4 — feedback (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| วงในค้างแล้วจะเปลี่ยนไปหมวดอื่นยังไง | **L3 (กดอนาล็อกซ้ายลง) = กลับไปเลือกหมวด** · pill แสดงคำใบ้ `L3 เปลี่ยนหมวด` ตอนอยู่จังหวะ Char และ `ปล่อย LS = ล็อก` ตอนเลือกหมวด · วง/ไฮไลต์ไม่หายระหว่างสลับ |
| รัสเซียยังโผล่ใน Win+Space และแก้เองไม่ได้ (น่าจะมาจากคำสั่งของเรา) | ตรวจพบ layout **0419 (RU) ถูกโหลดค้างใน session** จาก `LoadKeyboardLayoutW("00000419")` ตอนสร้าง keymap (ADR-28 + test) → **unload ออกแล้ว** และแก้ `build_keymap` ให้ **unload layout ที่เพิ่งโหลดทุกครั้ง** · ตรวจ `GetKeyboardLayoutList` เหลือ `0409 (EN) + 041E (TH)` |

## 0.4 รอบ 3 — feedback (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| ปล่อย **Analog ขวา** กลับศูนย์แล้ว **วงในกลับตำแหน่งเดิม + UI หาย** | วงล้อ Fast mode **ค้างไว้** หลังมีการเลือกกลุ่ม (`group.is_some()`) — ปล่อย RS/ไม่มี input ก็ยังเห็นวง + ไฮไลต์กลุ่ม/ตัวอักษรเดิม ไม่รีเซ็ต |
| เอา **เมนูเลือก mode (D-pad)** กลับมา | `D-pad ←` เปิด/ปิดวงคำสั่ง (EN/TH/Lang/Mouse/UI…) เหมือนโหมดปกติ · ใน Fast mode `D-pad ↑ = ถอย`, `↓/→ = ไปข้างหน้า` เลื่อนไฮไลต์ · ปุ่ม View ยังเปิดวงได้เป็นทางเลือก |

## 0.5 รอบ 2 — feedback (2026-11-17)

| สิ่งที่ owner แจ้ง | สิ่งที่แก้ |
| --- | --- |
| Fast mode ต้องแสดง **วงล้อตอนกด Analog ขวา** | `Session` คำนวณ `state` ของ HUD จาก Fast mode เอง: มีการเลือก (LS/D-pad) หรือ **RS กำลังชี้ทิศ** = `group`/`char` → HUD วาดวง (รอบ 3 ปรับให้ค้างไว้หลังเลือกกลุ่มแล้ว) |
| เอา **ภาษารัสเซีย** ออก | เพิ่ม `config::is_retired_language("ru")` — ซ่อน RU จาก Settings Languages + ไม่โหลดเข้า runtime + `strip_retired_languages()` ตัด entry ออกจาก config ตอน load/save + เอา RU ออกจาก dropdown Practice |
| pill `Type TH · Fast` ใหญ่/บังข้อความ | HUD ตั้ง `data-fast="on"` → CSS ย่อ pill ใน Fast mode (สูง 20px · ฟอนต์ 10px · badge 9px · ระยะชิดขึ้น) — ไม่ได้ซ่อนทั้งอัน |

## 1. สรุป 30 วินาที

| Feature | สถานะ | พฤติกรรม |
| --- | --- | --- |
| 1 · Fast mode | ✅ ทำเสร็จ | **LS ดันเบา = หมวด · ดันสุด = latch หมวด + เข้าโซนตัวอักษร** (ADR-33) → **A = พิมพ์** · เปลี่ยนหมวดได้โดยไม่ต้องผ่านศูนย์ (ดึงกลับ < 0.70) |
| 2 · HUD เกาะ caret | ✅ ทำเสร็จ (best-effort) | HUD ย้ายไปเกาะตำแหน่ง caret ที่กำลังพิมพ์ · **ไม่มีจุดฟ้า/มาร์กเกอร์** · แอปที่ไม่ประกาศ caret = คง pivot เดิม |
| 3 · RS = ลูกศร + highlight | ✅ ทำเสร็จ (owner ยืนยัน "ใช่ Analog ขวา" 2026-11-17) | RS = ลูกศร · **R3 สองครั้งติดกัน = โหมดเลือกข้อความ** (Shift+ลูกศร) |
| Output · installer | ✅ | smoke เฉพาะ feature ที่แก้ → เลื่อน version เป็น 0.3.2 → build NSIS + portable zip |

เปิด/ปิด Fast mode ได้ที่ **Settings → General → Fast mode** (ค่าเริ่มต้น = เปิด)
config key: `defaults.typing.fastMode` (boolean)

## 2. การตัดสินใจในรอบนี้

1. **จังหวะของ Fast mode** — **LS = หมวด** (เหมือน Mode ธรรมดา เปลี่ยนได้ตลอด) · **D-pad = ช่องตัวอักษร** · **A = พิมพ์** (รอบ 5 เปลี่ยนจาก LS ล็อกหมวด)
2. **A = พิมพ์ / accept** — ไม่ส่ง Space ในโหมดนี้ (กันพิมพ์ผิด) · **D-pad = ช่องตัวอักษร** (`↑` ถอย · `↓/→` ไปข้างหน้า · `←` = วงคำสั่ง/เมนูโหมด) และ **A = accept D-pad**
   - เปลี่ยนหมวดด้วย LS = ล้างช่องเดิม (ต้องเลือกช่องของหมวดใหม่ก่อนพิมพ์)
   - L3 = ล้างช่องที่เลือก (ไม่เปลี่ยนหมวด)
3. **B = ล้างไฮไลต์ก่อน** — ถ้าไม่มีไฮไลต์จึงเป็น Backspace · X = Enter · Y = Tab/Esc เหมือนเดิม
4. **ปุ่ม View = วงคำสั่ง (ทางเลือก)** — รอบ 3 คืน `D-pad ←` เป็นเมนูโหมดตามเดิมแล้ว · View ยังใช้เปิดวงได้
5. **พิมพ์ซ้ำ** — ไฮไลต์ช่องค้างไว้ กด A ซ้ำ = พิมพ์ตัวเดิม (ตัด auto-repeat จาก LS ออก เพราะ LS ใช้เลือกหมวด)
6. **RS = ลูกศร 4 ทิศ** (repeat 350/90 ms) · **R3×2 = สลับโหมดเลือกข้อความ** → ลูกศรมี Shift ติดไปด้วย
7. **Feature 2** — ตำแหน่ง HUD ใน Fast mode มาจาก caret ของหน้าต่างที่ focus (`GetGUIThreadInfo` + `ClientToScreen`) อัปเดตทุก 250 ms และยังผ่าน pivot clamp เดิม · **ไม่มีการวาดจุด pivot ในโหมดพิมพ์** (จุดมีเฉพาะโหมด Nav)
8. **ค่าเริ่มต้นเปิด** — เพื่อให้ทดสอบได้ทันทีหลังติดตั้ง และมี toggle ให้ปิดใน Settings
9. **`fastMode` ไม่บังคับใน schema** — config เก่าที่ยังไม่มี key ยังใช้ได้ (อ่านเป็นเปิด)
10. **รอบ 2 · วงล้อตาม input** — Fast mode ไม่ใช้ `GestureState` ของท่อปกติ (always Idle) จึงคำนวณ `state` ใหม่: `group`/`char` เมื่อ `fast_rings` = มีกลุ่มที่เลือก หรือ RS กำลังชี้ทิศ หรือโหมดเลือกข้อความ · ไม่งั้น `idle`
11. **รอบ 2 · RU = retired** — ไม่ลบ keymap/ADR-28 ออกจาก core (ยังใช้ทดสอบ universal layout ได้) แต่ UI/runtime/config ไม่รับ RU อีก · EL ยังอยู่
12. **รอบ 2 · pill** — ย่อเฉพาะเมื่อ `fast.on` (ไม่กระทบโหมดปกติ/ม็อกอัป) · ค่า: block-size 20px, font 10px, badge 9px, `bottom: -24px`
13. **รอบ 3 · วงค้าง** — `fast_rings = fast && (arrows || select || group.is_some())` → ล็อกกลุ่มแล้ววงไม่หายจนกว่าจะปิด Fast mode/สลับโหมด · RS ไม่แตะ `FastGesture` จึงไม่รีเซ็ตกลุ่ม
14. **รอบ 3 · D-pad ← = เมนูโหมด** — คืนพฤติกรรมเดิม (toggle วงคำสั่ง) · ไฮไลต์ใช้ `↑` ถอย, `↓/→` ไปข้างหน้า · View เป็นทางเลือก
15. **รอบ 4 · L3 = เปลี่ยนหมวด** — วงค้างไม่ใช่ทางตัน: L3 พากลับไปจังหวะ Group โดยเก็บกลุ่มเดิมไว้ · pill มีคำใบ้สั้น ๆ เพื่อให้ค้นพบได้เอง
16. **รอบ 4 · ห้ามทิ้ง layout ใน Windows** — `build_keymap` ต้อง `UnloadKeyboardLayout` เป้าหมายที่เพิ่งโหลด (best-effort) · RU ไม่อยู่ในลิสต์ผู้ใช้ Windows แต่ถูก "โหลดค้าง" จาก API นี้
17. **รอบ 5 · LS = หมวด, D-pad = ช่อง** — owner ต้องการให้เปลี่ยนหมวดได้แบบ Mode ธรรมดา · ตัดแนวคิด "ล็อกหมวด" ออก (`FastGesture` เหลือ `group` + `sector`) · เปลี่ยนหมวด = ตั้ง sector ตามทิศ LS แล้ว D-pad ปรับต่อได้
18. **รอบ 5 · วงคำสั่งใช้ D-pad + A** — `Edge` ของ D-pad ↑/↓/→ เลื่อน index ในวงคำสั่ง/วงภาษา · A ยืนยัน · RS ยังใช้ได้เหมือนเดิม (ไม่ breaking)
19. **รอบ 6 · gauge ต้องหมุนรอบตัวเอง** — `.fast__value` ใช้ `transform-box: fill-box` (default view-box ทำให้ Chrome หมุนรอบมุม viewBox → วงลอยลง 340px)
20. **รอบ 6 · กลางวงโปร่ง** — mask เจาะรู `.hud__glass` (r 28) + ซ่อน `.hud__center`/`.hud__preview` เฉพาะ `data-fast="on"`
21. **รอบ 6 · D-pad = Mode ปกติ** — D-pad ← toggle วงคำสั่งเสมอ · D-pad อื่นไม่ทำอะไรในโหมด push (`letterPick=dpad` ยังใช้ได้) · วงคำสั่งกลับไปใช้ RS เลือก
22. **รอบ 7 · ปล่อยจอย = UI หาย** — `FAST_CENTER_RESET_MS` = 120 ms · ไม่มีเงื่อนไข `fast_select` ค้างวง (select ยังทำงานแต่ UI โชว์เฉพาะตอนขยับ RS)
23. **รอบ 7 · auto-repeat ปุ่มหน้า** — A ค้าง 400 ms → Space/ตัวอักษรซ้ำ · B ค้าง 400 ms → Backspace ซ้ำ ทุก `repeat_rate_ms` (90 ms) · เอา Ctrl+Backspace ออก
24. **รอบ 7 · เริ่มที่ EN** — ค่า default `typing.mode = "en"` และไม่ activate จาก config ตอนบูต (ผู้ใช้สลับภาษาเองได้)
25. **รอบ 7 · Fast mode ไม่มี pill** — ซ่อน `.hud__status` เมื่อ `data-fast="on"`
26. **รอบ 7 · build ไม่มี console** — release build ไม่ใส่ feature `console` → PE subsystem 2 (GUI)

## 3. ไฟล์ที่แก้

| ไฟล์ | สิ่งที่แก้ |
| --- | --- |
| `crates/rnitype-core/src/gesture.rs` | `FastGesture` v2 (push-through, ADR-33): โซน `hidden/category/letter`, latch, ownership LS/D-pad, A lockout/lookback · **รอบ 6:** D-pad ไม่ถูกใช้ในโหมด push |
| `crates/rnitype-core/src/typing.rs` | `TypingPipeline::tick_fast` + `set_fast`/`fast_*` · **รอบ 6:** `fast_push()` accessor |
| `crates/rnitype-core/src/runtime.rs` | v2 wiring + **รอบ 7:** auto-repeat A/B, `FAST_CENTER_RESET_MS` 120, `b_repeat_ok`, tests (`holding_a_repeats_space`, `b_hold_repeats_backspace`, `fast_mode_holding_a_repeats_the_character`) |
| `crates/rnitype-core/src/lib.rs` | export `FastCommit`, `FastGesture`, `FastNav`, `FastPhase` |
| `crates/rnitype-win/src/caret.rs` (ใหม่) | `caret_point()` — caret ของหน้าต่างที่ focus (คืน `None` เมื่อไม่มี) |
| `crates/rnitype-win/src/lib.rs` | `pub mod caret` |
| `crates/rnitype-win/Cargo.toml` | feature `Win32_Graphics_Gdi` (ClientToScreen) |
| `crates/rnitype-app/src/config.rs` | `defaults.typing.fastMode` + `fast_mode()` + test · **รอบ 2:** `is_retired_language` / `strip_retired_languages` + 2 tests, กรอง RU จาก `language_rows` |
| `crates/rnitype-win/src/language.rs` | **รอบ 4:** `build_keymap` unload layout เป้าหมายหลังสร้างตาราง (กัน RU ค้างใน Win+Space) |
| `crates/rnitype-app/src/session.rs` | `HudState.fast { on, phase, select, zone, r }` · **รอบ 6:** ไม่เปลี่ยน (HUD ฝั่ง JS/CSS) |
| `crates/rnitype-app/src/main.rs` | ต่อ config → engine ทั้งตอนเริ่ม/live reload · HUD เกาะ caret · **รอบ 7:** ไม่ activate ภาษาจาก config ตอนบูต (เริ่ม EN) |
| `crates/rnitype-app/src/config.rs` | default `typing.mode = "en"` + test |
| `crates/rnitype-app/src/input.rs` | `fast_typing_script` (push-through) + `push_typing_script` สำหรับ `--fake-pad` + test |
| `ui/hud/hud.js` | badge `Fast` / `Select` · `data-fast` · `data-fast-zone` · gauge arc/marker จาก `fast.r` |
| `ui/hud/hud.css` | pill ย่อใน Fast mode · **รอบ 6:** `.fast__value` `transform-box: fill-box` (แก้ gauge ลอย) · เจาะกลางโปร่ง + ซ่อนจุด/ตัวอักษรกลาง |
| `ui/hud/index.html` | **ADR-33:** `<svg id="fastArc">` (track/value/marker) |
| `ui/settings/index.html` · `settings.js` | toggle Fast mode ใน General + reset/preview · **รอบ 2:** เอา RU ออกจาก `PHRASES` (Practice) |
| `tools/tests/settings-smoke.mjs` | +2 checks (toggle มีจริง/เปิดโดยค่าเริ่มต้น/สลับได้) → 35/35 |
| `project menager/Design/03-engineering/contracts/hud-state.schema.json` | optional `fast { on, phase, select }` |
| `project menager/Design/03-engineering/contracts/hud-state.samples.json` | +sample `type-th/char` ที่เปิด Fast mode (ใช้กับ state player ได้) |
| `project menager/Design/04-decisions/32-fast-mode.md` | ADR-32 บันทึกการตัดสินใจ/ทางเลือกที่ไม่เลือก/ผลข้างเคียง |
| `project menager/Design/README.md` · `HANDOFF.md` · `02-roadmap-and-smoke-tests.md` · `03-parallel-execution-plan.md` | อัปเดตดัชนี ADR-32, HANDOFF #32, ตารางวัดผล 0.2.5, P2 slice 5 |
| `project menager/Design/03-engineering/contracts/config.schema.json` | `defaults.typing.fastMode` · **ADR-33:** `defaults.typing.letterPick` (`push`/`dpad`) |
| `Cargo.toml` · `crates/rnitype-app/tauri.conf.json` | version 0.1.0 → 0.2.0 → … → 0.3.0 → **0.3.2** |
| `installer/build-installer.ps1` | ชื่อ zip ตาม version + เลือก setup ตาม version (เดิมหยิบไฟล์เก่า) · **0.3.2** |
| `installer/README-beta.txt` | 0.3.2 |

## 4. Smoke run (เฉพาะ feature ที่แก้)

| ชุด | ผล |
| --- | --- |
| `cargo test --workspace` | **244 passed** (core 118 · win 58 · app 68) |
| `node tools/contracts/validate.mjs` | **13/13** (มี sample Fast mode) |
| `node tools/tests/hud-smoke.mjs` | **54/54** |
| `node tools/tests/hud-typography.mjs` | **36/36** |
| `node tools/tests/settings-smoke.mjs` | **37/37** (รวม `letterPick`) |
| Windows input list | `GetKeyboardLayoutList` = **0409 (EN) · 041E (TH)** เท่านั้น — ไม่มี 0419 (RU) ค้างหลังรัน test ทั้งชุด |
| HUD gauge rect | `.fast__value` อยู่ที่ (442,142) ตรงกลาง `fastArc` (430,130) หลังแก้ `transform-box` |
| installer 0.3.2 | build ผ่าน · `setup /S` ติดตั้งทับ → `%LOCALAPPDATA%\RniType\RniType.exe` = **0.3.2** |

## 5. ข้อจำกัด / งานที่ยังไม่ได้ทำ

- **ยังไม่ได้ทดสอบกับจอยจริงในรอบนี้** — ต้องให้ owner กด LS/L3/D-pad/R3/RS ด้วยมือ (smoke ที่รันได้คือ fake-pad + unit tests)
- **Feature 2 เกาะ caret ได้เฉพาะแอปที่ประกาศ caret ให้ Windows** (Notepad, Word, browser ส่วนใหญ่) — แอปเกม/custom UI จะไม่เห็น caret → HUD คง pivot ล่าสุด
- caret ที่ได้เป็น **มุมซ้ายล่างของ caret** และยังโดน pivot clamp (margin 172 × hudScale) ที่ขอบจอ
- **Fast mode v2 (ADR-33) = push-through**: ดัน LS เบา = หมวด · ดันสุด = ตัวอักษร · ปล่อย/ศูนย์ 250 ms = UI หาย · **D-pad ทำตัวเหมือน Mode ปกติ** (← = วงคำสั่งเท่านั้น; `letterPick=dpad` คือ fallback)
- **กลางวงใน Fast mode โปร่งใส** (ไม่มีจุด/ตัวอักษรกลาง) — โหมดอื่นยังมีจุดกลางตามดีไซน์
- **RU ไม่ได้อยู่ในรายการภาษาของ Windows** (User Profile = en-US, th) — ที่เห็นใน Win+Space คือ layout ที่ถูก *โหลดค้าง* · ถ้า flyout ยังค้างอยู่ ให้ปิด/เปิดเมนูใหม่หรือ restart Explorer
- ยังไม่ **code-sign** · ประวัติ version ก่อนหน้า (0.2.0–0.2.5) ไม่ได้เขียนเป็น ADR แยก
- DualSense ยังไม่ได้ทดสอบ (ไม่มีฮาร์ดแวร์)

## 6. ขั้นถัดไป

1. owner ติดตั้ง **0.3.4** แล้วลอง: LS ดันเบา→ดันสุด→**ปล่อยแล้วกด A** (ต้องพิมพ์ได้) · B ตอน UI ซ่อน = Backspace · gauge อยู่วงนอกพอดี · D-pad ← เปิดเมนูโหมด
2. calibrate threshold 0.85 กับจอยจริง (ถ้าดันไม่ถึง ให้ใช้ `letterPick=dpad` ใน Settings)
3. วัด WPM ในแท็บ Practice เทียบโหมดเดิม (เป้า 12 WPM ไทย / 20 WPM อังกฤษ)
4. อัปเดต [[HANDOFF]] + [[03-parallel-execution-plan]] + [[02-roadmap-and-smoke-tests]] เมื่อ owner ยืนยันผล

## 7. Fast mode v2 (ADR-33) — สรุปข้อเสนอ

Owner ขอ: **Analog ซ้ายตัวเดียวเลือกทั้งวงในและวงนอก** · ปล่อย/กลับศูนย์ = reset เมนู + UI หาย · D-pad เลือกเมนูด้วย A ตกลง / B Cancel · พิมพ์ด้วย A
ข้อสรุปจากคำปรึกษา (Claude Opus CLI):

- **A ห้ามทำสองหน้าที่** → ใช้ **ระยะเอียง** แยกโซน: 0.30–0.75 = หมวด · ≥ 0.85 = latch ตัวอักษร · A พิมพ์เฉพาะโซนตัวอักษร (A = ตกลงเฉพาะตอนเมนูขับด้วย D-pad)
- **reset เมื่อศูนย์ ≥ 250 ms** (ไม่ใช่ทันที) + ละเว้นทิศ 80 ms กัน spring-back สร้างทิศผี
- **เจ้าของเมนู = ตัวที่ขยับล่าสุด** · LS แย่งกลับเมื่อ `r > 0.5` + ข้าม sector
- **B = ย้อน 1 ระดับ** · ไม่มีเมนู + LS ศูนย์ = Backspace ทันที
- ทางเลือก 1 (push-through) เป็นค่าเริ่มต้น · เก็บ 0.2.5 ไว้เป็น fallback `letterPick: "dpad"`

**Task ที่แตกไว้ (รายละเอียด/acceptance 12 ข้ออยู่ใน ADR-33)**

| # | slice | ไฟล์หลัก |
| --- | --- | --- |
| S1 | config `letterPick` + Settings + schema | `config.rs` · settings · contracts |
| S2 | core state machine v2 (โซน/latch/ownership/lockout) + 12 tests | `gesture.rs` · `typing.rs` |
| S3 | runtime wiring (D-pad ตาม state, B/Backspace, fallback dpad) | `runtime.rs` |
| S4 | HUD arc/tick/คำใบ้ + `HudState` | `hud.js` · `hud.css` · `session.rs` · contracts |
| S5 | fake-pad v2 + log `[fast]` | `main.rs` · `input.rs` |
| S6 | smoke + 0.3.0 + installer | `Cargo.toml` · `tauri.conf.json` · `installer/` |

> ✅ ทำครบ S1–S6 ใน **0.3.0** — ดู §8

## 8. รอบ 7 — Fast mode v2 (ADR-33) ลงโค้ด 0.3.0

**ทำครบ S1–S6** (ทำงานเองทั้งหมด — ทุก slice แตะไฟล์ชุดเดียวกัน)

| slice | ผล |
| --- | --- |
| S1 | `defaults.typing.letterPick` (`push`/`dpad`) + Settings → General (segmented) + schema |
| S2 | `FastGesture` v2: โซน `Hidden/Category/Letter`, latch 60 ms, owner LS/D-pad, B ย้อนชั้น, A lockout 100 / lookback 40 ms + unit tests |
| S3 | runtime: D-pad = วงคำสั่งเท่านั้น (ไม่แตะวง Fast) · B/Backspace · fallback `dpad` ใช้ ↑/↓/→ เลื่อนช่อง |
| S4 | HUD: `fast.zone` + `fast.r` + arc/marker รอบวง · schema/sample อัปเดต |
| S5 | `--fake-pad` ใช้ท่าดันสุดอัตโนมัติ · log `[config] fast mode … pick=push` |
| S6 | version **0.3.0** · installer + portable zip |

**Smoke**: `cargo test --workspace` **245 passed** (core 119 · win 58 · app 68) · contracts **13/13** · settings **37/37** · typography 36/36 · hud 54/54 · `--fake-pad` พิมพ์ 5 ตัว/รอบ (`ก ะ โ ็ ต`)

**ยังไม่ได้ทำ**: calibrate threshold กับจอยจริง · threshold ยังไม่เปิดเป็น config · code-sign

## 9. รอบ 8 — bug fix: กด A ไม่ได้ (0.3.4)

| อาการ | สาเหตุ | ที่แก้ |
| --- | --- | --- |
| ปล่อย Analog แล้วกด A ไม่พิมพ์ | 0.3.3 ซ่อน UI **และล้างหมวด/ช่อง** ทันทีที่ศูนย์ 120 ms → A เจอโซน Hidden | แยก "ซ่อน UI" (120 ms) ออกจาก "ล้างการเลือก" (**1500 ms**) · ระหว่างนั้น **A ยังพิมพ์ตัวที่เลือกได้** |
| B ตอน UI ซ่อนไป cancel แทน Backspace | B เช็คแค่ว่า "มีเมนู" | UI ซ่อนอยู่ = ถือว่าไม่มีเมนู → **B = Backspace ทันที** (ค้าง = ซ้ำ) |
| D-pad ← | — | ยังเปิด/ปิดวงคำสั่งเหมือนเดิม |

**Smoke**: `cargo test --workspace` **250 passed** (core 123 · win 58 · app 69) · contracts 13/13 · settings 37/37

**ไฟล์**: `gesture.rs` (`FAST_RESET_MS`, `ui_hidden`), `typing.rs`/`runtime.rs` (proxy + B rule), `session.rs` (`fast_rings` ไม่โชว์ตอนซ่อน), tests ใหม่ 3 ตัว
