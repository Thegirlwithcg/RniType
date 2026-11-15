---
title: HANDOFF — RniType (อ่านก่อนเริ่ม session ใหม่)
status: v0.1
updated: 2026-11-15
tags: [handoff]
---

# HANDOFF — RniType v0.1 (2026-11-15)

> เอกสารนี้ทำให้ session ใหม่ทำงานต่อได้ใน \~5 นาที
> อ่าน 8 หัวข้อนี้จบ = รู้ทุกอย่างที่จำเป็น

## 1. สิ่งที่ต้องรู้ใน 30 วินาที

| หัวข้อ | ข้อสรุป |
| --- | --- |
| ผลิตภัณฑ์ | Windows background utility: จอย Xbox/DualSense ใช้ **พิมพ์แทนคีย์บอร์ด** และ **แทนเมาส์** มีหน้าตั้งค่าแบบ Logi Options+ |
| ช่องทางขาย | Steam + itch.io (ทั้งคู่รับ .exe/.zip — ไม่ต้องใช้เอนจิน) |
| สถานะ | **P1 ปิดแล้ว · Spike C ผ่าน (SendInput ไทย) · A/B ยังไม่ผ่าน — ยังไม่มีแอปจริง** |
| สแตก | **Rust** (core) + **Tauri v2** (UI/tray) + **gilrs** (จอย, fallback `sdl3`) + `SendInput` |
| แผนสำรอง | C# .NET 8 + WPF/WinUI (ถอยได้ทุกเมื่อ เอกสารใช้ร่วมกันได้) |
| งานถัดไป | Owner รัน spike A/B/C → เปิด P2–P5 ขนาน (4 track) |

## 2. การตัดสินใจที่ล็อกแล้ว (ห้ามถามซ้ำ)

1. **พิมพ์** = LS เลือก 8 กลุ่ม × RS เลือก 1 ใน 8 (64 ช่อง/ชั้น) — ยืนยันด้วยการปล่อย RS กลับศูนย์ · **เลื่อน LS เปลี่ยนกลุ่มได้ขณะดันค้าง** ([[09-ls-switch-group]])
2. **ปุ่มหน้า**: A = Space, B = Backspace, X = Enter, Y = Tab (ค้าง = Esc)
3. **LT = Shift (กดครั้งเดียว = ล็อกตัวพิมพ์ใหญ่, กดอีกครั้ง = ตัวเล็ก · UI แสดง `ตัวใหญ่`), LB = Ctrl** — [[13-shift-toggle]]; RB = คลิกซ้าย, RT = คลิกขวา
4. **D-pad ← = วงคำสั่งในวงใน 8 ช่อง** (แตะ = เปิด/ปิด): Type-TH / Type-EN / Mouse / Nav / Copy / Paste / Undo / Redo — **เลือกด้วย RS (อนาล็อกขวา), ปล่อย RS = ยืนยัน** (A = ทำ, B = ยกเลิก; LS ไม่มีผลในวง) — [[10-command-wheel-rs]]
5. **วงใน (ตอนพิมพ์) = สิ่งที่ LS เลือก · วงนอก = สิ่งที่ RS เลือก** — เมนูกลุ่มอักษรใช้ LS (ข้อยกเว้น: วงคำสั่งใช้ RS)
6. **HUD ตรึงที่ pivot ไม่ตามเมาส์** — Nav = โหมดจัดตำแหน่ง UI (LS หยาบ · RS ละเอียด · R3/R กลับตำแหน่ง) — [[11-hud-pivot-nav]]
7. **ภาษาไทย**: วรรณยุกต์ ่ ้ ๊ ๋ อยู่กลุ่ม ↘ เดียวกัน; HUD มีขีดกำกับสระล่าง ุ ู — [[12-thai-tone-and-below-vowel]]
8. **Universal languages (v0.11)**: **ตรวจ layout ในเครื่องก่อน** → เครื่องไทย = EN+TH; layout อื่นเพิ่มเป็นเมนู; EN/TH ใช้ตารางออกแบบ, อื่นๆ map จาก QWERTY + แก้ได้; **IME = ส่งคีย์เหมือนคีย์บอร์ดจริง** — [[14-universal-languages]] · **ช่องภาษาอื่น**: 1 ภาษา = สลับทันที, หลายภาษา = Lang, ภาษาเดียว = ไม่มีช่อง
9. **Settings = หน้าต่างแยก ภาษาอังกฤษ** (General/Languages/Keymap/HUD) + Reset all to defaults
10. **กด RS = pointer กลับกลางจอหลัก** (ตาม canvas) — เฉพาะโหมดเมาส์
11. **ไม่ใช้ Godot** — [[02-engine-choice-godot]]
12. **ภาษาหลัก = Rust + Tauri v2** — [[03-stack-rust]]
13. ไฟล์ออกแบบอยู่ใน `Design/` + sub-folder (ตาม canvas)

## 3. โครงสร้างไฟล์

```
project menager/
├─ Requirment.canvas          ← โจทย์ตั้งต้น (แอป, background, Logi Options+)
├─ Research.canvas            ← โจทย์การออกแบบ (mapping, mouse, agent)
└─ Design/
   ├─ HANDOFF.md              ← ไฟล์นี้
   ├─ README.md               ← ดัชนี + เป้าหมาย + TL;DR
   ├─ 2D/                     ← ภาพจอย Xbox/PS5 (ของเดิมใน vault)
   ├─ 01-research/
   │  └─ 01-prior-art.md      ← งานวิจัย + WPM จริง + บทเรียน
   ├─ 02-design/
   │  ├─ 01-interaction-model.md
   │  ├─ 02-keymap-en-th.md   ← ตาราง 8×8 EN/TH (สำคัญที่สุด)
   │  ├─ 03-mouse-and-navigation.md
   │  └─ 04-state-machine-spec.md  ← ค่าเวลา/pseudocode
   ├─ 03-engineering/
   │  ├─ 01-architecture.md
   │  └─ 02-roadmap-and-smoke-tests.md
   ├─ 04-decisions/
   │  ├─ 01-open-questions.md ← Q1–Q16
   │  ├─ 02-engine-choice-godot.md
   │  ├─ 03-stack-rust.md
   │  ├─ 04-en-vowel-and-8mode.md  ← ADR-04
   │  ├─ 05-rs-arrow-and-selection.md ← ADR-05
   │  ├─ 06-dpad-right-edit-menu.md  ← ADR-06 (ถูกแทนที่)
   │  ├─ 07-dpad-left-command-wheel.md ← ADR-07 (วงคำสั่งวงใน 8 ช่อง)
   │  ├─ 08-hud-declutter-variant-b.md ← ADR-08 (HUD v0.9)
   │  ├─ 09-ls-switch-group.md      ← ADR-09 (LS เปลี่ยนกลุ่มได้)
   │  ├─ 10-command-wheel-rs.md     ← ADR-10 (วงคำสั่งเลือกด้วย RS)
   │  ├─ 11-hud-pivot-nav.md        ← ADR-11 (HUD ตรึง pivot / Nav)
   │  ├─ 12-thai-tone-and-below-vowel.md ← ADR-12 (วรรณยุกต์ + ขีดกำกับ)
   │  ├─ 13-shift-toggle.md         ← ADR-13 (Shift toggle)
   │  └─ 14-universal-languages.md  ← ADR-14 (หลายภาษา + Settings window)
   └─ 05-mockup/hud-prototype/  ← ม็อกอัป HUD v0.11 (index.html + hud.js + styles.css)
```

รันม็อกอัป: เปิด `project menager/Design/05-mockup/hud-prototype/index.html` ในเบราว์เซอร์
ทดสอบอัตโนมัติ: `node tools/tests/hud-smoke.mjs` (54 checks, ต้องมี Chrome)

## 4. งานถัดไป — ลำดับที่ต้องทำ

> แผนลงมือแบบละเอียด (7 เฟส · 4 track คู่ขนาน · subagent) อยู่ที่ [[03-parallel-execution-plan]]
> ลำดับย่อ: P0 spike → P1 contracts → P2/P3/P4/P5 ขนาน → P6 วัดผล+แพ็กเกจ
> **P1 ปิดแล้ว 2026-11-15** — contracts อยู่ที่ `Design/03-engineering/contracts/`, ตรวจด้วย `node tools/contracts/validate.mjs`

### ขั้น 4.1 Rust spike (2–3 วัน) — เกณฑ์ผ่านอยู่ใน [[03-stack-rust]]

| # | ทดสอบ | ผ่านเมื่อ |
| --- | --- | --- |
| A | `gilrs` อ่าน Xbox + DS5 | เห็นปุ่มครบ 8 + trigger + แกน 4 ทิศ, deadzone ถูก |
| B | Tauri overlay โปร่งใส | 2 จอ + scaling 150%, คลิกทะลุ, ไม่แย่ง focus |
| C | `SendInput` ภาษาไทย | พิมพ์ "กขคง" ลง Notepad ถูกต้อง (Unicode path) |

ถ้า A ไม่ผ่าน → เปลี่ยนชั้นจอยเป็น `sdl3` · ถ้า B ไม่ผ่าน → egui · ถ้าทั้งคู่พัง → ถอยไป C#

### ขั้น 4.2 M0 — โครงโปรเจกต์

- `cargo new rnitype --bin` + workspace (`core`, `app`, `ui`)
- โครง module ตาม [[01-architecture]]: device → normalizer → gesture → mapper → injector
- `tools/smoke/probe-pad.ps1` (ตัวแรกที่ต้องมี)

### ขั้น 4.3 M1 — Typing core

state machine ตาม [[04-state-machine-spec]] + keymap EN ตาม [[02-keymap-en-th]] + harness วัด WPM

## 5. คำถามที่ยังค้าง (เจ้าของต้องตอบก่อนเขียนโค้ดเยอะ)

| # | เรื่อง | ข้อเสนอเริ่มต้น |
| --- | --- | --- |
| Q1 | RS ทำสองหน้าที่ (เลือกตัวอักษร/เลื่อนเมาส์) | แยกด้วย mode ✅ แนะนำ |
| Q2 | "ปล่อยกับที่เดิม" หมายถึงอะไร | ปล่อยกลับศูนย์ + dwell 60 ms ✅ แนะนำ |
| Q4 | Alt อยู่ปุ่มไหน | hold-Y = Alt (หรือ View/Share) |
| Q5 | X/Y/A/B | A=Space, B=Backspace, X=Enter, Y=Tab ✅ แนะนำ |
| Q7 | layout ไทย | ความถี่ (ที่ออกแบบไว้) ก่อน, เลือก Kedmanee ได้ |
| Q10 | รองรับจอยอื่นไหม | ทุกตัวที่ SDL3 รู้จัก แต่ QA เฉพาะ Xbox + DS5 |

ที่เหลือ (Q3, Q6, Q8, Q9, Q11, Q12, Q13, Q14) มีข้อเสนอชัดเจนแล้วใน [[01-open-questions]] — ใช้ค่าเริ่มต้นได้เลย

## 6. ข้อจำกัดของ environment (สำหรับ agent คนถัดไป)

- โหมด Build ใน vault: แก้ไฟล์บนดิสก์โดยตรง (ไม่ใช้ nznote-write block)

- โฟลเดอร์ vault = `D:\Erin's Works\Programming\RniType` — เขียนได้เฉพาะในนี้
- MCP `nizylanote`: เรียก `vault_info` ก่อนเขียน, path เป็น relative + `/`
- ตอบเป็นภาษาไทย, เก็บรูปแบบ Markdown/ตารางเดิม, ห้ามลบโน้ตโดยไม่ถูกขอ
- **Git**: ยังไม่ commit — ทุกไฟล์เป็น untracked (`.nznote/`, `.opencode/` ก็ untracked)
  → ก่อน commit ต้องตัดสินใจว่า 2 โฟลเดอร์นั้นควรอยู่ใน repo ไหม

## 7. นิยาม "เสร็จ" ของ session ถัดไป

1. spike A/B/C ผ่านครบ และมีตัวเลข/log จริงแนบ (ไม่ใช่ "น่าจะได้")
2. มี `Cargo.toml` + module เปล่าที่ compile ผ่าน + `probe-pad.ps1` รันได้
3. อัปเดตตารางวัดผลใน [[02-roadmap-and-smoke-tests]] (คอลัมน์ build/WPM/latency)
4. ถ้าเปลี่ยนการตัดสินใจใด → เขียน ADR ใหม่ใน `04-decisions/` (อย่าแก้ ADR เก่า)

## 8. ตัวเลขอ้างอิงที่ใช้ตัดสินใจ

- TwoStick (จอย 2 แกน): 13–14.9 WPM หลัง 20 session
- Selection keyboard บนจอ: 12.9 WPM
- Daisywheel: ผู้ใช้รายงาน 20–30 WPM (ยังไม่มีงานวิจัยควบคุม)
- เป้าเรา: **20 WPM EN / 12 WPM TH**, error < 1.5%, latency < 16 ms

