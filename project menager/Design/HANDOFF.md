---
title: HANDOFF — RniType (อ่านก่อนเริ่ม session ใหม่)
status: v0.2
updated: 2026-11-15
tags: [handoff]
---

# HANDOFF — RniType v0.2 (2026-11-15)

> เอกสารนี้ทำให้ session ใหม่ทำงานต่อได้ใน \~5 นาที
> อ่าน 8 หัวข้อนี้จบ = รู้ทุกอย่างที่จำเป็น

## 1. สิ่งที่ต้องรู้ใน 30 วินาที

| หัวข้อ | ข้อสรุป |
| --- | --- |
| ผลิตภัณฑ์ | Windows background utility: จอย Xbox/DualSense ใช้ **พิมพ์แทนคีย์บอร์ด** และ **แทนเมาส์** มีหน้าตั้งค่าแบบ Logi Options+ |
| ช่องทางขาย | Steam + itch.io (ทั้งคู่รับ .exe/.zip — ไม่ต้องใช้เอนจิน) |
| สถานะ | **P0/P1 ปิด · P2/P3 slice 1 · P4 overlay+typography · P5 slice 1 (Settings→HUD) — 90 tests** |
| สแตก | **Rust** (core) + **Tauri v2** (UI/tray) + **gilrs** (จอย, fallback `sdl3`) + `SendInput` |
| แผนสำรอง | C# .NET 8 + WPF/WinUI (ถอยได้ทุกเมื่อ เอกสารใช้ร่วมกันได้) |
| งานถัดไป | Runtime wiring: จอย → core → injector → HUD ใน `rnitype-app` |

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
14. **Typography แยก 2 ตัวคูณ**: `hudScale` (วง, 1.2) ≠ `hudTextScale` (ตัวหนังสือ, 1.2) · ฟอนต์ไทย **ไม่มีหัว = IBM Plex Sans Thai** (fallback Noto Sans Thai) · ส่งค่าด้วย event `hud:style` ไม่ใช่ HudState · **Settings window (ภาษาอังกฤษ) มี slider + ฟอนต์** — [[16-hud-typography-and-fonts]]
15. **pivot/pointer ใน HudState = พิกัดในหน้าต่าง overlay** (0,0 = virtual desktop) — app แปลงจากพิกัดหน้าจอ ก่อน emit

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
   │  ├─ 02-roadmap-and-smoke-tests.md
   │  ├─ 03-parallel-execution-plan.md
   │  └─ contracts/           ← P1 freeze: hud-state, config, ipc.md (v1.1)
   ├─ 04-decisions/
   │  ├─ 01-open-questions.md ← Q1–Q16 (Q17 ตอบแล้ว)
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
   │  ├─ 14-universal-languages.md  ← ADR-14 (หลายภาษา + Settings window)
   │  ├─ 15-gilrs-xinput-backend.md ← ADR-15 (gilrs XInput)
   │  └─ 16-hud-typography-and-fonts.md ← ADR-16 (ขนาดตัวหนังสือ + ฟอนต์ไม่มีหัว)
   └─ 05-mockup/
      ├─ hud-prototype/        ← ม็อกอัป HUD v0.11 (index.html + hud.js + styles.css)
      └─ hud-shots/            ← screenshot ตรวจรับ (100%/150% DPI, overlay จริง, Settings)
```

### 3.1 ไฟล์โค้ดที่ใช้บ่อย

| path | หน้าที่ |
| --- | --- |
| `crates/rnitype-app/src/main.rs` | overlay window (full virtual desktop, click-through) + tray + Settings + IPC |
| `crates/rnitype-app/src/config.rs` | อ่าน/เขียน `%APPDATA%\RniType\config.json`, `ui_style()`, `validate_ui()` |
| `ui/hud/{index.html,hud.css,hud.js}` | HUD จริง — วาดจาก `hud:state` เท่านั้น · typography จาก `hud:style` |
| `ui/settings/*` | Settings window (P5 slice 1: แท็บ HUD) |
| `ui/shared/{tokens.css,fonts.css,fonts.js,fonts/*.ttf}` | สี/fอนต์ที่ใช้ร่วมกัน (IBM Plex Sans Thai + Noto Sans Thai + OFL) |
| `tools/tests/hud-typography.mjs` | ตรวจ typography + ถ่าย screenshot (24 checks) |
| `tools/tests/browser.mjs` | helper เปิด Chrome headless ผ่าน CDP |

รันม็อกอัป: เปิด `project menager/Design/05-mockup/hud-prototype/index.html` ในเบราว์เซอร์
ทดสอบอัตโนมัติ: `node tools/tests/hud-smoke.mjs` (54 checks, ต้องมี Chrome) · `node tools/tests/hud-typography.mjs` (24 checks + screenshot)

## 4. งานถัดไป — ลำดับที่ต้องทำ

> แผนลงมือแบบละเอียด (7 เฟส · 4 track คู่ขนาน · subagent) อยู่ที่ [[03-parallel-execution-plan]]
> ลำดับย่อ: P0 spike → P1 contracts → P2/P3/P4/P5 ขนาน → P6 วัดผล+แพ็กเกจ
> **P1 ปิดแล้ว 2026-11-15** — contracts อยู่ที่ `Design/03-engineering/contracts/`, ตรวจด้วย `node tools/contracts/validate.mjs`

### 4.1 P0 + P1 — ✅ ปิดแล้ว

- A: `gilrs` **XInput** อ่าน Xbox ครบ 16 ปุ่ม + 8 sectors ([15-gilrs-xinput-backend])
- B: Tauri overlay โปร่งใส คลิกทะลุ ไม่แย่ง focus (2 จอ)
- C: `SendInput` Unicode พิมพ์ไทยลง Notepad ถูก
- contracts + workspace + state player + 54/54 mockup checks

### 4.2 P2/P3/P4 — slice merged

| Track | ทำแล้ว | เหลือ |
| --- | --- | --- |
| P2 core | วงคำสั่ง/วงภาษา/mode/shift/face buttons (50 tests) | repeat, layer 2, double-tap/hold, quickbar, caret nav, mouse math, Nav pivot |
| P3 win | SendInput Unicode/VK/mouse + partial-send guard (34 tests) | scancode/IME, exclusion/UIPI, runtime wiring |
| P4 HUD | renderer ครบ mode + overlay เต็ม virtual desktop/click-through/tray + **typography pass + ฟอนต์ไทยไม่มีหัว** | `hud:state` จาก runtime จริง, 60 Hz coalescing, latency, pivot per-monitor/mixed-DPI |
| P5 Settings | **slice 1: หน้าต่าง Settings + แท็บ HUD (text size/font/reset) + config.json** | แท็บ General/Languages/Keymap, rescan HKL, keymap editor, profiles |

### 4.3 ขั้นถัดไป (เรียง)

1. **Runtime wiring** ใน `rnitype-app`: `gilrs` XInput → `PadState` → `RuntimeEngine` → `SendInputInjector` → `hud:state` (~60 Hz) + `release_all()` เมื่อจอยหลุด/ปิด
2. **P2 slice 3**: repeat/auto-repeat, layer 2, double-tap/hold, quickbar, caret nav
3. **P5 Settings ที่เหลือ** (General/Languages/Keymap) + ตรวจภาษา/สร้าง keymap จาก HKL
4. **P6** วัด WPM/latency, ทดสอบคนจริง, installer/Steam/itch

### 4.4 รัน overlay ตอนนี้ (demo)

```powershell
# overlay + state ตัวอย่าง (วน 9 สถานะ ทุก 3 วินาที)
crates\rnitype-app\run.ps1 -- --demo --seconds 30

# เพิ่ม --settings = เปิดหน้าต่าง Settings ตั้งแต่เริ่ม (ตรวจ typography/ฟอนต์)
crates\rnitype-app\run.ps1 -- --demo --settings --seconds 60
```

- Tray → Settings… / Quit (หรือ `Ctrl+C` ใน terminal)
- ไม่มี `--demo` = HUD idle (รอ runtime จริง)
- path มี apostrophe → ใช้ `run.ps1`; build.rs copy icon ไป `%TEMP%` ให้ `cargo test/check` ทำงานได้เอง
- typography: เปลี่ยนใน Settings → HUD เปลี่ยนทันที (event `hud:style`) · ค่าถูกเก็บที่ `%APPDATA%\RniType\config.json`

### 4.5 UI feedback จาก owner (2026-11-15) — ✅ ปิดแล้ว (ADR-16)

**ปัญหา:** ตัวหนังสือ HUD เล็กและอ่านไม่ชัด โดยเฉพาะบนจอ 3840×1080 และภาษาไทย
**ขอ:** ภาษาไทยใช้ **ฟอนต์ไม่มีหัว (loopless)** และ Settings ต้องปรับขนาด/เลือกฟอนต์ได้

| # | งาน | ผล |
| --- | --- | --- |
| 1 | Typography pass + default `hudScale` 1.2 + text scale แยก | group **13.8** (len-l 11.4) · char **20.4–30** · sub/badge **11** · pill **13.2** · ring 340→408 |
| 2 | ฟอนต์ไทยไม่มีหัว | **IBM Plex Sans Thai** (4 น้ำหนัก) + Noto Sans Thai fallback + OFL ใน `ui/shared/fonts/` · เลิกใช้ Leelawadee เป็นค่าเริ่มต้น |
| 3 | Settings → HUD | slider 0.8–2.0 (default 1.2) + 3 ฟอนต์ + preview + Reset → save ลง `config.json` ทันที |
| 4 | Contract | `ui.hudTextScale` + `ui.hudFontFamily` (config v1) · event **`hud:style`** + command **`hud_style`** (ipc v1.1, additive) — [[16-hud-typography-and-fonts]] |
| 5 | ตรวจรับ | `hud-typography.mjs` **24/24** + screenshot 100%/150% DPI + overlay จริง + Settings → `05-mockup/hud-shots/` · contrast ตัวอักษรจาง **6.87:1** |

**สิ่งที่ owner ควรดูก่อนปล่อย:** เปิด Settings เลื่อน text size 1.0 / 1.2 / 1.5 บนจอจริงของตัวเอง
(3840×1080 + DPI ที่ใช้จริง) แล้วบอกว่า 1.2 พอดีหรืออยากได้ค่าเริ่มต้นอื่น — ค่าเดียวที่ต้องแก้คือ
`ui.hudTextScale` ใน `config.schema.json` + `crates/rnitype-app/src/config.rs` + `hud.css` (`STYLE_DEFAULT`)

### 4.6 เริ่ม session หน้า

1. อ่าน HANDOFF นี้ + [[03-parallel-execution-plan]] §P4/P5
2. งานหลัก: **runtime wiring** (จอย → core → injector → `hud:state`) — typography ปิดแล้ว
3. ถ้าทำ Settings ต่อ: เริ่มจากแท็บ General แล้ว Languages/Keymap (contract อยู่ใน [[ipc]] §3)
4. รัน checks: `cargo test --workspace` · `node tools/contracts/validate.mjs` · `node tools/tests/hud-smoke.mjs` · `node tools/tests/hud-typography.mjs`
5. ถ้าแตะ overlay: รัน `crates\rnitype-app\run.ps1 -- --demo --settings --seconds 30` แล้วเก็บ screenshot/log

## 5. คำถามที่ยังค้าง (เจ้าของต้องตอบก่อนเขียนโค้ดเยอะ)

| # | เรื่อง | ข้อเสนอเริ่มต้น |
| --- | --- | --- |
| Q1 | RS ทำสองหน้าที่ (เลือกตัวอักษร/เลื่อนเมาส์) | แยกด้วย mode ✅ แนะนำ |
| Q2 | "ปล่อยกับที่เดิม" หมายถึงอะไร | ปล่อยกลับศูนย์ + dwell 60 ms ✅ แนะนำ |
| Q4 | Alt อยู่ปุ่มไหน | hold-Y = Alt (หรือ View/Share) |
| Q5 | X/Y/A/B | A=Space, B=Backspace, X=Enter, Y=Tab ✅ แนะนำ |
| Q7 | layout ไทย | ความถี่ (ที่ออกแบบไว้) ก่อน, เลือก Kedmanee ได้ |
| Q10 | รองรับจอยอื่นไหม | XInput สูงสุด 4 ตัว; SDL3 ภายหลังถ้าต้องการ DualSense เต็ม |
| Q17 | ฟอนต์ไทย/ขนาดเริ่มต้น | **ปิดแล้ว** — IBM Plex Sans Thai (loopless) + text scale 1.2 + ring scale 1.2 ([[16-hud-typography-and-fonts]]) |

ที่เหลือ (Q3, Q6, Q8, Q9, Q11, Q12, Q13, Q14) มีข้อเสนอชัดเจนแล้วใน [[01-open-questions]] — ใช้ค่าเริ่มต้นได้เลย

## 6. ข้อจำกัดของ environment (สำหรับ agent คนถัดไป)

- โหมด Build ใน vault: แก้ไฟล์บนดิสก์โดยตรง (ไม่ใช้ nznote-write block)

- โฟลเดอร์ vault = `D:\Erin's Works\Programming\RniType` — เขียนได้เฉพาะในนี้
- MCP `nizylanote`: เรียก `vault_info` ก่อนเขียน, path เป็น relative + `/`
- ตอบเป็นภาษาไทย, เก็บรูปแบบ Markdown/ตารางเดิม, ห้ามลบโน้ตโดยไม่ถูกขอ
- **Git**: commit ล่าสุด `062f700` (docs) — งานรอบนี้ยังไม่ commit
  (`.nznote/`, `.opencode/` อยู่ใน `.gitignore` แล้ว)
- **เทอร์มินัลเป็น cp874** → log/ข้อความไทยใน console อาจเป็น `???` (ไม่ใช่บั๊ก) · ไฟล์ยังเป็น UTF-8
- **Chrome headless**: `C:\Users\User\AppData\Local\ms-playwright\chromium-1208\chrome-win64\chrome.exe`
  (ตั้ง `CHROME_PATH` ถ้าย้าย) · เครื่องนี้มี 2 จอ virtual desktop = 3840×1080 เริ่มที่ x = -1920

## 7. นิยาม "เสร็จ" ของ session ถัดไป

1. ทุก check ผ่านและมีตัวเลขแนบ (ไม่ใช่ "น่าจะได้"):
   `cargo test --workspace` · `node tools/contracts/validate.mjs` · `node tools/tests/hud-smoke.mjs` · `node tools/tests/hud-typography.mjs`
2. งานที่แตะ runtime ต้องมี log จริงจาก `run.ps1 -- --demo --settings --seconds N` + screenshot
3. อัปเดตตารางวัดผลใน [[02-roadmap-and-smoke-tests]] (คอลัมน์ build/WPM/latency)
4. ถ้าเปลี่ยนการตัดสินใจใด → เขียน ADR ใหม่ใน `04-decisions/` (อย่าแก้ ADR เก่า)

## 8. ตัวเลขอ้างอิงที่ใช้ตัดสินใจ

- TwoStick (จอย 2 แกน): 13–14.9 WPM หลัง 20 session
- Selection keyboard บนจอ: 12.9 WPM
- Daisywheel: ผู้ใช้รายงาน 20–30 WPM (ยังไม่มีงานวิจัยควบคุม)
- เป้าเรา: **20 WPM EN / 12 WPM TH**, error < 1.5%, latency < 16 ms

