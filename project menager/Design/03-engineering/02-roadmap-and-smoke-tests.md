---
title: Roadmap และ Smoke tests
status: draft v0.1
updated: 2026-11-15
tags: [plan, testing]
---

# 03.2 — Roadmap, Smoke test และเกณฑ์ผ่าน

## 1. หลักการทำงาน (Loop Engineering)

```
ออกแบบ → implement 1 ก้อนเล็ก → smoke run → วัดผล → ปรับ → บันทึก
                                   ↑__________________________|
```

- 1 loop ไม่เกิน 1 วัน: ถ้าเกินแปลว่าก้อนงานใหญ่เกิน ให้ผ่า
- **ทุก loop ต้องมีตัวเลข**: WPM, error rate, latency, CPU — ไม่มีตัวเลข = ยังไม่เสร็จ
- งานที่ต้องทำซ้ำ ≥ 3 ครั้ง → เขียนเป็น script (`tools/smoke/*`)
- งานที่ไม่ขึ้นต่อกัน (เช่น Thai keymap กับ mouse engine) แยกให้ agent คนละตัวทำขนานได้
  โดยยึดสัญญาในเอกสารนี้เป็น interface

## 2. Milestones

| Milestone | ขอบเขต | เกณฑ์ผ่าน (exit criteria) | ประมาณ |
| --- | --- | --- | --- |
| **M0 Spike** | อ่านค่าจอย, ตรวจชนิดจอย, log แกนดิบ | เห็นค่า LS/RS/trigger/ปุ่มครบ **Xbox** · **DualSense เลื่อนไปทดสอบก่อน release** (ยังไม่มีฮาร์ดแวร์) · ระบุรุ่นได้ | 1–2 วัน |
| **M1 Typing core** | normalizer + state machine + keymap EN + injector | พิมพ์ประโยค 20 คำใน Notepad ได้ถูกต้อง 100%, วัด WPM ได้ | 3–4 วัน |
| **M2 Mouse** | pointer + accel + คลิก + recenter หลายจอ | ข้ามจอ 1920 px < 1.5 s, ชี้ target 32 px สำเร็จ > 80% | 2–3 วัน |
| **M2.5 HUD polish** | variant B (idle เหลือ pill, วงโผล่เมื่อดัน LS), วงคำสั่งวงใน, เปลี่ยนกลุ่มด้วย LS | mockup v0.9 ผ่าน 35 checks; ผู้ใช้ใหม่เปิดวงคำสั่ง/สลับโหมดได้ < 2 s; ไม่มี feedback ซ้ำซ้อน | 1–2 วัน |
| **M3 HUD** | overlay 2 ring + preview + pill สถานะ + idle pill | เปิด/ปิดได้, ไม่แย่ง focus, latency HUD < 16 ms | 3 วัน |
| **M4 Config UI** | tray, หน้าตั้งค่า, profile ต่อแอป, autostart | ตั้งค่าแล้วมีผลทันที, reload ได้, crash-free 1 ชม. | 5 วัน |
| **M5 Thai + polish** | keymap TH, layer 2, quick bar, exclusion list | 12 WPM ไทย, 20 WPM อังกฤษ, error < 1.5% | 5+ วัน |

## 3. Smoke scripts (แผน — ยังไม่สร้าง)

| Script | ทำอะไร | ใช้บ่อยแค่ไหน |
| --- | --- | --- |
| `tools/smoke/probe-pad.ps1` | list จอย, live แกน/ปุ่ม, ตรวจ deadzone/drift | ทุก loop (M0–M5) |
| `tools/smoke/test-typing.ps1` | รัน `--selftest --phrase <ชุด>` → พิมพ์ลง Notepad → diff + คำนวณ WPM/error | ทุก loop ที่แตะ engine |
| `tools/smoke/test-mouse.ps1` | วัดความเร็ว/ความแม่นที่ vmax ต่างๆ, ตรวจข้ามจอ | loop ของ M2 |
| `tools/smoke/test-hud.ps1` | วัดเวลา HUD ขึ้นหลัง state เปลี่ยน (จาก log timestamp) | loop ของ M3 |
| `tools/smoke/run-all.ps1` | เรียกทั้ง 4 ตัว + สรุปตารางเดียว | ก่อน commit ทุกครั้ง |

### 3.1 ที่มีอยู่จริงแล้ว (2026-11-15)

| Script | ทำอะไร | ผลล่าสุด |
| --- | --- | --- |
| `tools/contracts/validate.mjs` | ตรวจ `hud-state.samples.json` + `config.sample.json` กับ schema | **10/10** |
| `tools/tests/hud-smoke.mjs` | ขับม็อกอัป v0.11 ด้วย CDP (วง/คำสั่ง/โหมด/สถานะ) | **54/54** |
| `tools/tests/hud-typography.mjs` | ตรวจขนาดขั้นต่ำ/การชนกัน/contrast/emphasis + screenshot 100%–150% DPI | **30/30** |
| `cargo test --workspace` | core 52 · win 36 · app 21 (config/input/session) | **109/109** |

**ชุดประโยคทดสอบ**: ใช้ 500 phrases ของ MacKenzie & Soukoreff (มาตรฐานงานวิจัย text entry)
→ ผลของเราจะเทียบกับงานวิจัย (TwoStick 13–14.9 WPM) ได้ตรงๆ
ชุดภาษาไทย: ประโยคภาษาไทย 50 ประโยค + คำที่พบบ่อย 200 คำ

## 4. ตารางวัดผล (กรอกทุก loop)

| วันที่ | build | WPM EN | WPM TH | error % | latency (ms) | หมายเหตุ |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-11-15 | M0 | — | — | — | — | P0 ผ่าน: A XInput 16 ปุ่ม · B click-through · C SendInput ไทย (DualSense เลื่อน) |
| 2026-11-15 | P4 slice 3 | — | — | — | — | Typography: group 13.8 / char 20.4–30 / pill 13.2 px · contrast 6.87:1 · 24/24 checks · ยังไม่มี WPM (รอ runtime) |
| 2026-11-15 | P4 slice 4 | — | — | — | — | Ring 340px + text 16px (วงเล็กลง 17%) · active animation · ขีดสระล่าง · 29/29 checks |
| 2026-11-15 | runtime | — | — | — | — | จอยจริง → core → injector → HUD · `--fake-pad` พิมพ์จริง 21 คีย์/9 วิ · 108 tests · ยังไม่ได้ทดสอบจอยจริง |
| 2026-11-15 | probe A | — | — | — | — | Xbox Controller: LS/RS ±1.00 · idle_peak 0.000 · LT/RT **ดิจิทัล** (LZ/RZ = 0.00) → `lt_analog` ใช้ไม่ได้กับจอยนี้ |
| 2026-11-15 | UI r3 | — | — | — | — | ยกเลิก scale sector (วงเพี้ยน) · Shift→A-Z บนวง · 109 tests · typography 30/30 |

## 5. ความเสี่ยงและทางถอย

| ความเสี่ยง | โอกาส | ผลกระทบ | ทางรับมือ |
| --- | --- | --- | --- |
| 8×8 จำยากเกินไป | กลาง | สูง | มี Variant B (4 ทิศ × 16) + HUD ตลอด |
| HUD เงียบเกินไป (idle ไม่มีวง) ทำให้นักเรียนรู้ไม่ทัน | กลาง | กลาง | pill mode + onboarding + เปิด "แสดงวงตลอดเวลา" ([[08-hud-declutter-variant-b]]) |
| HUD ตรึง pivot แล้วไปทับข้อความ | กลาง | กลาง | Nav ย้าย UI ได้ทันที · จำตำแหน่งล่าสุด · R3/R กลับตำแหน่ง ([[11-hud-pivot-nav]]) |
| เปลี่ยนกลุ่มพลาดจากการสั่นของ LS | กลาง | กลาง | hysteresis 9° + ต้องผ่าน 0.35 ([[09-ls-switch-group]]) |
| นิ้วโป้งเมื่อยในการพิมพ์ยาว | กลาง | กลาง | dwell สั้น, repeat, เตือนพักทุก 20 นาที |
| เกม/แอปบล็อก SendInput | สูง | ต่ำ | exclusion list, เอกสารข้อจำกัด |
| Thai IME ตีกับ scan code | สูง | กลาง | Unicode path (มีอยู่แล้วในสถาปัตยกรรม) |
| latency สูงจาก overlay | ต่ำ | กลาง | แยกเธรด, วาดเฉพาะตอนเปลี่ยน |
| ผู้ใช้อยากได้ layout Kedmanee | กลาง | ต่ำ | Variant C ใน [[02-keymap-en-th]] |

## 6. แผนงาน UI v0.9 → M3 (ทำต่อจากม็อกอัป)

| # | งาน | เสร็จเมื่อ |
| --- | --- | --- |
| 1 | ล็อกดีไซน์ variant B + ADR-08 | ✅ ม็อกอัป v0.9 + ADR-08/09 |
| 2 | เขียน onboarding 3 ขั้น (ดัน LS → วงใน · D-pad ← → วงคำสั่ง · ปล่อย = ยืนยัน) | มีข้อความ/ภาพจริงในม็อกอัป และเปิดครั้งแรกได้ใน < 30 s |
| 3 | ทำ overlay จริง (Tauri) ตามสเปก HUD v0.10 (pivot, วงคำสั่ง RS, ไทยมีขีด) | เปิด/ปิดได้, click-through, latency < 16 ms |
| 4 | วัดผล UI กับคน 3–5 คน | เวลาเปิดวงคำสั่ง < 2 s, เปลี่ยนกลุ่มถูกใน < 1.2 s, คะแนน "รก" ลดลง vs v0.7 |
| 5 | ปรับ pill สถานะให้ครบ (mode/selection/layer/shift/arrow) | ไม่มีสถานะไหนต้องเดา |
| 6 | Onboarding เรื่อง pivot/Nav | ผู้ใช้ย้าย UI ได้เองโดยไม่ต้องเปิดเอกสาร |
| 7 | **Language support (v1)**: ตรวจ layout, QWERTY default, override, IME passthrough | เปิด RU/EL พิมพ์ได้, IME ประกอบข้อความเหมือนคีย์บอร์ดจริง |
| 8 | **Settings window แยก** (English) + Reset all | ตั้งค่า/รีเซ็ตได้โดยไม่แตะ overlay |
| 8a | Settings แท็บ HUD: text size / font / reset + persist | ✅ P5 slice 1 — [[16-hud-typography-and-fonts]] |
| 8b | Settings แท็บ General / Languages / Keymap | P5 ถัดไป |

## 7. สิ่งที่ต้องทำก่อนเริ่ม M0

1. ตอบคำถามใน [[01-open-questions]] (อย่างน้อย Q1, Q2, Q5)
2. ยืนยันว่าแอปเป็น Windows-only (อ้างอิง Logi Options+ → ใช่)
3. ยืนยันว่าจะเก็บ config ที่ `%APPDATA%` และไม่ต้อง sync cloud ใน v1
4. **Rust spike 2–3 วัน** ตามเกณฑ์ใน [[03-stack-rust]] (DS5, overlay click-through, พิมพ์ไทย)
   — ถ้าไม่ผ่านให้ถอยไป C# ก่อนเริ่ม M0
