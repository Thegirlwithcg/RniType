---
title: ADR-18 — Runtime wiring: จอย → core → injector → HUD
status: accepted (P4 slice 5 / P2-P3 runtime)
updated: 2026-11-15
tags: [decision, runtime, injector, hud, tray]
---

# ADR-18 — ต่อของจริง: จอย → core → injector → `hud:state`

## บริบท

จนถึง ADR-17 แอปมี HUD จริง + Settings จริง แต่ **ยังไม่มีอะไรขับ HUD**
(`--demo` วน state จากไฟล์ตัวอย่าง) และ core/injector ยังไม่ถูกเรียกจากแอปเลย
งานนี้คือ "runtime wiring" ตาม HANDOFF §4.3 ข้อ 1

## การตัดสินใจ

### 1. แยก 2 โมดูลใหม่ใน `rnitype-app`

| ไฟล์ | หน้าที่ | รู้จัก |
| --- | --- | --- |
| `input.rs` | `PadSource` trait + `GilrsSource` (จอยจริง) + `ScriptSource` (จอยปลอม) | gilrs |
| `session.rs` | `Session`: engine + injector + pivot + สร้าง `HudState` | core + win (ไม่รู้จัก Tauri/gilrs) |

`trait PadSource { fn sample(&mut self, now_ms) -> Option<PadState>; fn name(&self) -> String }`
ทำให้ loop เหมือนกันทั้งจอยจริงและสคริปต์ และ **ทดสอบได้ด้วย `NullInjector`**

### 2. Loop เดียว 4 ขั้น

```
อ่านจอย (gilrs XInput) → RuntimeEngine::tick(dt_ms) → Action → injector → HudState (≤60 Hz, เมื่อเปลี่ยน)
```

- **~250 Hz**: `sleep(3ms)` + `dt` วัดจากนาฬิกาจริง (ไม่ผูกกับความแม่นของ sleep)
- **HUD ≤ 60 Hz** และส่งเฉพาะเมื่อ payload เปลี่ยน (เทียบ JSON string) — ตรงกับ ipc.md §1
- `dt` ถูก clamp 1–50 ms (เครื่องหยุดชั่วคราวไม่ทำให้ state machine กระโดด)

### 3. `Action` ที่ไม่เข้ากล่อง injector

| Action | ทำอะไร |
| --- | --- |
| `Command::Copy/Paste/Undo/Redo` | แปลงเป็น `Key(Char('c'/'v'/'z'/'y'), ctrl: true)` |
| `Command::Mouse/TypeEn/PlaceUi/Language` · `SetMode` · `SetLang` | engine เปลี่ยน mode/ภาษาเองแล้ว → ไม่ส่งถึง OS |
| `MovePivot { dx, dy }` | ขยับ pivot ของ HUD (Nav math ยัง TODO ใน core) |
| อื่น ๆ | ส่งเข้า injector ตรง ๆ |

### 4. `Ctrl + ตัวอักษร ASCII` = VirtualKey (แก้ `plan`)

เดิม `plan()` ส่ง `Char` ทาง Unicode เสมอ → `Ctrl+Unicode('c')` **ไม่ถูกตีความเป็น Ctrl+C**
โดยแอปปลายทาง (คีย์ลัดใช้ VK) ตอนนี้ `plan()` สลับไปส่ง VK เมื่อ `ctrl` เปิดและตัวอักษรเป็น
`a–z A–Z 0–9` · ตัวอักษรอื่น (ไทย/สัญลักษณ์) ยังเป็น Unicode ตามเดิม
→ test เดิม 3 ตัวถูกปรับให้ตรงพฤติกรรมใหม่ + เพิ่ม `ctrl_with_non_ascii_stays_on_the_unicode_path`

### 5. จอยหลุด / กลับมา

- หลุด: `injector.release_all()` (กัน modifier ค้าง) + `engine.set_mode(Disabled)` → HUD แสดง `Paused`
- กลับมา: resume เป็น `Type` **เฉพาะ** เมื่อ `disabled_remaining_ms == 0`
  (ถ้าถูกปิดด้วย emergency LB+RB ต้องรอ deadman 3 s ตาม spec)
- `Disabled` ที่ไม่มี `disabled_remaining_ms` = พักถาวรจนกว่าจะมีคนสั่ง (ใช้กับการหลุด/พัก)

### 6. Tray: `Pause / Resume typing`

- `AppState.paused: AtomicBool` · loop อ่านทุกเฟรม
- ตอนพัก: ส่ง `PadState` กลาง (ไม่มีการเคลื่อนไหว) + `release_all()` ครั้งเดียว → พิมพ์ไม่ได้จนกว่าจะกดกลับ
- เหตุผล: จอยอ่านอยู่ตลอดเวลา ถ้าไม่มีการพัก การเล่นเกมจะพิมพ์ตัวอักษรลงเกม
  (exclusion list/UIPI ยังเป็นงาน P3 ที่เหลือ)

### 7. `--fake-pad` = จอยปลอมสำหรับตรวจ

`ScriptSource` เล่นท่าพิมพ์ซ้ำ (LS → RS → ปล่อย) ทำให้ตรวจทั้งท่อได้โดยไม่ต้องมีจอย:
`[type] ก` ใน log + ตัวอักษรจริงในหน้าต่างที่ focus + วง HUD ขยับตาม
**เตือนใน log**: จะพิมพ์ลงหน้าต่างที่ focus อยู่

### 8. ค่าจาก config ที่ runtime ใช้

| config | ใช้ทำอะไร |
| --- | --- |
| `defaults.typing.commitDwellMs` (40–120) | `GestureConfig.commit_dwell_ms` |
| `defaults.typing.mode` (`en`/`th`) | ภาษาเริ่มต้น |
| `languages[].enabled` | ภาษาที่เปิด (ว่าง → EN+TH) → สร้าง slot ภาษาอื่นให้ engine |
| `ui.hudScale` / `ui.hudOpacity` | `HudState.scale/opacity` (อ่านทุกครั้งที่ emit) |

> `languages` ของ engine = **ภาษาอื่นที่ไม่ใช่ EN** (ตาม core) — คงภาษาปัจจุบันไว้ในลิสต์
> ไม่งั้น `set_languages` จะตีกลับเป็น EN

### 9. `HudState` สร้างที่ฝั่งแอปที่เดียว

`Session::hud_state(style, origin)` แปลง engine → JSON ตาม contract v1:
mode (`type-en/type-th/cmd/lang/mouse/nav/disabled`), state (`idle/group/char/commit`),
`inner` (group/command/language), `outer` (char labels ของกลุ่มที่เลือก), preview,
`pivot/pointer` เป็น **พิกัดหน้าต่าง** (ลบ origin — ตาม ADR-16 §15)
มี unit test เทียบ field ที่ contract บังคับครบ

## ผลที่ตามมา

- `crates/rnitype-app/src/{input,session}.rs` ใหม่ · `main.rs` มี `spawn_runtime` / `run_loop`
- core: `TypingPipeline::group/sector`, `RuntimeEngine::typing_group/typing_sector`,
  `wheel_highlight_dir` ใช้ได้ทั้งสองวง (`wheel_dir`), `keymap::group_names/label_of`
- win: `plan` ส่ง VK เมื่อ Ctrl+ASCII · `vk_for_ctrl_char`
- tray: `Pause / Resume typing`
- ทางรัน: `run.ps1` (ไม่มี flag) = จอยจริง · `run.ps1 --fake-pad` = จอยปลอม

## หลักฐาน (2026-11-15)

| หลักฐาน | ผล |
| --- | --- |
| `cargo test --workspace` | **108 passed** (app 20 · core 52 · win 36) |
| session unit tests | พิมพ์ตัวอักษรถึง injector · A→Space · Copy→Ctrl+C · HUD mapping · พิกัดหน้าต่าง · จอยหลุด |
| `--fake-pad` (แอปจริง) | log `[type] …` 21 ครั้ง/9 วิ + ตัวอักษรเข้าหน้าต่างที่ focus จริง + HUD วาดวงไทย |
| screenshot | `05-mockup/hud-shots/app-runtime-typing.png` (HUD จาก engine จริง ไม่ใช่ demo state) |
| จอยจริง | **ยังไม่ได้ทดสอบ** — เครื่องนี้ไม่พบจอย (probe: pads 0) · `read_pad` ใช้โค้ดชุดเดียวกับ spike A ที่ผ่านบน Xbox |

## ความเสี่ยง / งานที่ยังไม่ทำ

| เรื่อง | สถานะ |
| --- | --- |
| Mouse mode (pointer math/accel) · Nav pivot | ยังไม่มีใน core (P2 slice ถัดไป) — mouse actions ส่งผ่าน injector แล้วถ้ามี |
| auto-repeat · ปุ่มหน้า double-tap/hold · layer 2 · quickbar | P2 ถัดไป |
| exclusion list / UIPI (เกมบางตัวบล็อก SendInput) | P3 ที่เหลือ — ตอนนี้ใช้ tray Pause แทน |
| IME/scancode path | P3 ที่เหลือ (Unicode ยังใช้ได้กับ layout ไทย) |
| จอยที่ต่อตอนแอปรันอยู่แล้วกดค้าง | `pump` เก็บ event ทุกเฟรม; ตรวจด้วยจอยจริงก่อน release |
| **LT/RT ของจอยที่ทดสอบเป็นดิจิทัล** (probe: LZ/RZ = 0.00) | `pad.buttons.lt/rt` ยังใช้ได้ (Shift toggle / right-click) · `lt_analog`/`rt_analog` = 0 → precision hold ใน Mouse mode ต้องอ่านจากค่าดิจิทัล หรือรอจอยที่ส่ง analog |
| 250 Hz ด้วย `sleep(3ms)` | Windows sleep ละเอียด ~15 ms ได้ → dt วัดจริงจึงยังถูก; ถ้าต้องการแม่นขึ้นใช้ `timeBeginPeriod` ใน P6 |
