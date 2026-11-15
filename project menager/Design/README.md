---
title: RniType — Design Index
status: draft v0.1
updated: 2026-11-15
---

# RniType — เอกสารออกแบบ

เกมแพด (Xbox / DualSense) ทำหน้าที่เป็น **คีย์บอร์ด** และ **เมาส์** ผ่านแอปที่รันอยู่เบื้องหลัง
พร้อมหน้าตั้งค่าแบบ Logi Options+

## เป้าหมายที่วัดผลได้

| เป้าหมาย | ตัวชี้วัด |
| --- | --- |
| พิมพ์งานได้จริง | ≥ 20 WPM ภาษาอังกฤษ, ≥ 12 WPM ภาษาไทย หลังฝึก 10 session |
| ใช้งานแทนเมาส์ได้ | ชี้เป้า ±8 px บน 1080p, ข้ามจอได้, latency < 16 ms |
| ใช้งานลื่นไหล | พิมพ์ผิด (uncorrected error) < 1.5% |
| ตั้งค่าได้ | profile ต่อแอป, keymap แก้ได้, ตรวจชนิดจอยอัตโนมัติ |

## หลักการออกแบบ 5 ข้อ

1. **สองแกน = สองการตัดสินใจ** — แกนซ้ายเลือก "วงใน" (กลุ่ม / วงคำสั่ง), แกนขวาเลือก "วงนอก" (ตัวอักษรในกลุ่ม)
   ไม่ต้องจำลำดับการกด ไม่ต้องเลื่อนเมนู
2. **ปล่อยแล้วจึงพิมพ์** — ยืนยันด้วยการปล่อยแกนขวากลับเข้าศูนย์ (ย้อนกลับได้ก่อนปล่อย)
3. **ปุ่มที่ใช้ทุกวินาทีต้องไม่ต้องจำ** — Space / Backspace / Enter อยู่บนปุ่มหน้า (A/B/X)
   ไม่ต้องผ่านแกน
4. **ยึดตำแหน่งที่สมองรู้อยู่แล้ว** — ตัวอักษรในกลุ่มเรียงตามความถี่ และอยู่ใกล้ทิศที่นิ้วโป้งถนัด
5. **ทุกอย่างต้องมีทางออก** — mode ชั่วคราว, HUD บอกสถานะ, ปุ่มปิดฉุกเฉิน, ค่าที่ปรับได้

## สารบัญ

> เปลี่ยน session? อ่าน [[HANDOFF]] ก่อน — สรุปทุกอย่างในไฟล์เดียว

| ไฟล์ | เนื้อหา |
| --- | --- |
| [[01-prior-art]] | งานวิจัย/ผลิตภัณฑ์ที่มีอยู่, ตัวเลข WPM จริง, บทเรียนที่นำมาใช้ |
| [[01-interaction-model]] | โมเดลการใช้งาน, mode, ปุ่มหน้า, ปุ่มผสม, HUD |
| [[02-keymap-en-th]] | ตารางแมป 8×8 ภาษาอังกฤษ + ภาษาไทย, ชั้นที่ 2 |
| [[03-mouse-and-navigation]] | สูตรเลื่อน pointer, ความเร็ง, หลายจอ, คลิก/ลาก/สกรอล |
| [[04-state-machine-spec]] | state machine, ค่าเวลาเริ่มต้น, pseudocode |
| [[01-architecture]] | เทคโนโลยี, โมดูล, การส่งคีย์ (SendInput), config |
| [[02-roadmap-and-smoke-tests]] | แผนงาน M0–M5, smoke script, เกณฑ์ผ่าน |
| [[03-parallel-execution-plan]] | แผนลงมือ 7 เฟส (P0–P6), 4 track คู่ขนาน, การแบ่ง subagent |
| [[01-open-questions]] | ข้อที่ยังต้องตัดสินใจ + ข้อขัดแย้งที่พบใน canvas |
| [[02-engine-choice-godot]] | ADR: ทำไมไม่ใช้ Godot เป็นแกน + ข้อควรรู้เรื่องขายบน Steam/itch |
| [[03-stack-rust]] | ADR: เปลี่ยนแกนเป็น Rust + Tauri, crates, เกณฑ์ไปต่อ/ถอย |
| [[04-en-vowel-and-8mode]] | ADR: กลุ่มสระ EN, RB/RT = ชั้น 2, D-pad 8 ช่อง |
| [[05-rs-arrow-and-selection]] | ADR: RS = ลูกศรเมื่อไม่ดัน LS, RB+RS = ลากเลือก, R3×2 = selection |
| [[06-dpad-right-edit-menu]] | ADR: D-pad → = Copy/Paste/Undo/Redo (ถูกแทนที่โดย ADR-07) |
| [[07-dpad-left-command-wheel]] | ADR: วงคำสั่งวงใน 8 ช่อง (โหมด + Copy/Paste/Undo/Redo) บน D-pad ← (v0.10 เลือกด้วย RS) |
| [[08-hud-declutter-variant-b]] | ADR: HUD v0.9 ลดความรก — ล็อก variant B + โหมดเรียนรู้ |
| [[09-ls-switch-group]] | ADR: เลื่อน LS เปลี่ยนกลุ่มได้ขณะดันค้าง |
| [[10-command-wheel-rs]] | ADR: วงคำสั่งเลือกด้วยอนาล็อกขวา (RS) |
| [[11-hud-pivot-nav]] | ADR: HUD ตรึงที่ pivot + Nav ใช้จัดตำแหน่ง |
| [[12-thai-tone-and-below-vowel]] | ADR: รวมวรรณยุกต์กลุ่มเดียว + ขีดกำกับสระ ุ ู |
| [[13-shift-toggle]] | ADR: Shift/LT กดครั้งเดียวล็อกตัวพิมพ์ใหญ่ + UI แสดง |
| [[14-universal-languages]] | ADR: รองรับทุกภาษา (layout + IME), Settings window แยก, English UI, Reset |
| [[15-gilrs-xinput-backend]] | ADR: WGI ไม่ส่ง event บนเครื่อง owner → ใช้ gilrs XInput backend |
| [[16-hud-typography-and-fonts]] | ADR: ขนาดตัวหนังสือแยกจากขนาดวง + ฟอนต์ไทยไม่มีหัว (IBM Plex Sans Thai) + Settings→HUD |
| [[17-hud-ring-geometry-and-active-animation]] | ADR: วง 340px/ตัวอักษร 16px, วงในกว้างขึ้น, ป้าย 2 บรรทัด, ขีดสระล่าง, active animation, Ring size slider |
| [[18-runtime-wiring]] | ADR: จอย → core → injector → HUD จริง, tray Pause, `--fake-pad` |
| [[19-revert-sector-scaling-and-shift-labels]] | ADR: ยกเลิก transform บน sector (วงเพี้ยน) + Shift เปลี่ยนตัวอักษรบนวงเป็นตัวใหญ่ |
| [[20-hud-state-delivery]] | ADR: HUD ต้องได้ state เสมอ (resend ตอน handshake + heartbeat 2 วิ) + ห้ามเปิดแอปซ้ำ |
| [[21-mouse-nav-repeat-layer2]] | ADR: Mouse/Nav math, auto-repeat 350/90, layer 2 (RB/RT), A double-tap, B/Y hold |
| [[hud-v0.9]] | บันทึก HUD: ภาพเทียบ A/B (v0.9) + สรุป v0.10 |
| `05-mockup/hud-prototype/` | ม็อกอัป HUD v0.11 (เน้น EN+TH, หลายภาษา, Settings window, Keymap editor, pivot) |
| `05-mockup/hud-shots/` | screenshot ตรวจรับ typography (100%/150% DPI), overlay จริง, Settings |
| [[HANDOFF]] | สรุปสำหรับส่งต่อ session: สถานะ, งานถัดไป, ข้อจำกัด |

## TL;DR

| ส่วน | ข้อสรุป |
| --- | --- |
| การพิมพ์ | LS = 8 กลุ่ม (64 ช่อง) — เลื่อนเปลี่ยนกลุ่มได้ระหว่างดันค้าง, RS = 1 ใน 8, ยืนยันเมื่อ RS ปล่อยกลับศูนย์ |
| ตัวปรับ | LT = Shift (กดครั้งเดียวล็อกตัวใหญ่), LB = Ctrl, A = Space, B = Backspace, X = Enter, Y = Tab |
| เมาส์ | RS = pointer (มี accel), กด RS = กลับกลางจอหลัก, RB = คลิกซ้าย, RT = คลิกขวา — pointer แสดงเฉพาะโหมดนี้ |
| Nav | ใช้ย้าย **pivot** (ตำแหน่ง UI): LS หยาบ · RS ละเอียด · R3/R กลับตำแหน่ง |
| mode + แก้ข้อความ | **D-pad ←** เปิดวงคำสั่งวงใน 8 ช่อง (EN, Language, Mouse, Place UI, Copy/Paste/Undo/Redo) — **เลือกด้วย RS, ปล่อย = ยืนยัน**; วงในขณะพิมพ์ยังเป็น LS |
| ภาษา | ทุก layout ที่ติดตั้งใน Windows — EN/TH ใช้ตารางออกแบบ, อื่นๆ map จาก QWERTY แล้วแก้ได้; IME ส่งคีย์ให้ OS ประกอบ · **ช่องภาษาอื่น**: 1 ภาษา = สลับทันที, หลายภาษา = `Lang`, ไม่มีภาษา = ซ่อน |
| ตั้งค่า | หน้าต่างแยก ภาษาอังกฤษ (General · Languages · Keymap · HUD) + Reset all to defaults |
| HUD | ตรึงที่ **pivot** (ไม่ตามเมาส์) — Nav = โหมดจัดตำแหน่ง UI |
| Typography | ฟอนต์ไทย **ไม่มีหัว** = IBM Plex Sans Thai (fallback Noto Sans Thai) · **text size 1.4** และ **ring size 1.0 (340px)** = ค่าเริ่มต้นที่ owner ยืนยัน · ช่องที่ active เน้นด้วยสี/stroke/ขนาด (ไม่ใช้ transform) · Shift → A-Z ([[19-revert-sector-scaling-and-shift-labels]]) |
| ภาษาไทย | วรรณยุกต์ ่ ้ ๊ ๋ อยู่กลุ่ม ↘ เดียวกัน · สระล่าง ุ ู มีขีดกำกับใน HUD |
| หลักการ HUD | วงใน = สิ่งที่ LS เลือก · วงนอก = สิ่งที่ RS เลือก · idle เหลือ pill สถานะ (เปิดวงตลอดได้) |
| เทคโนโลยี | Rust + Tauri v2 + gilrs/SDL3 + SendInput (แผนสำรอง: C# .NET 8) — ดู [[03-stack-rust]] |
| สถานะ | **P0/P1 ปิด · P2 slice 3 · P3/P4/P5 slice 1 · runtime จอยจริง** — 144 tests |

## หมายเหตุการทำงาน

- งานนี้ทำโดย agent เดียว (DeepSeek V4.1-flash) ในรอบนี้ — ไม่ได้เรียก Claude CLI
  เพราะ harness ห้ามเรียก AI CLI อื่น; ข้อที่จะปรึกษาได้ถูกเขียนเป็นคำถามใน [[01-open-questions]]
- ข้อขัดแย้งที่พบใน canvas (RS ถูกใช้ทั้งเลือกตัวอักษรและเลื่อนเมาส์) ถูกแก้ด้วยระบบ mode
  ดู [[01-interaction-model]]
