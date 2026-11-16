---
title: HANDOFF — RniType (อ่านก่อนเริ่ม session ใหม่)
status: v1.1
updated: 2026-11-16
tags: [handoff]
---

# HANDOFF — RniType v1.1 (2026-11-16)

> เอกสารนี้ทำให้ session ใหม่ทำงานต่อได้ใน \~5 นาที · อ่านจบ = รู้ทุกอย่างที่จำเป็น

## 1. สิ่งที่ต้องรู้ใน 30 วินาที

| หัวข้อ | ข้อสรุป |
| --- | --- |
| ผลิตภัณฑ์ | Windows background utility: จอย Xbox/DualSense ใช้ **พิมพ์แทนคีย์บอร์ด** และ **แทนเมาส์** มีหน้าตั้งค่าแบบ Logi Options+ |
| ช่องทางขาย | Steam + itch.io (ทั้งคู่รับ .exe/.zip — ไม่ต้องใช้เอนจิน) |
| สถานะ | **P0/P1 ปิด · P2 slice 4 · P3 slice 2 · P4/P5 slice 1–3 · P6 slice 1 · visual refresh (ADR-22) — 213 tests** |
| สแตก | **Rust** (core) + **Tauri v2** (UI/tray) + **gilrs** (จอย, XInput) + `SendInput` (auto scancode/Unicode) |
| แผนสำรอง | C# .NET 8 + WPF/WinUI (ถอยได้ทุกเมื่อ เอกสารใช้ร่วมกันได้) |
| งานถัดไป | **universal QWERTY mapping** (RU/EL) หรือ **P6 ที่เหลือ** (error rate/CPU, installer) |
| ค้างยืนยัน | owner retest §4.8 — quick bar/caret/sticky drag, exclusion/scancode, Settings, WPM, **Keymap editor** |

## 2. การตัดสินใจที่ล็อกแล้ว (ห้ามถามซ้ำ)

1. **พิมพ์** = LS เลือก 8 กลุ่ม × RS เลือก 1 ใน 8 (64 ช่อง/ชั้น) — ยืนยันด้วยการปล่อย RS กลับศูนย์ · **เลื่อน LS เปลี่ยนกลุ่มได้ขณะดันค้าง** ([[09-ls-switch-group]])
2. **ปุ่มหน้า**: A = Space, B = Backspace, X = Enter, Y = Tab (ค้าง = Esc)
3. **LT = Shift (กดครั้งเดียว = ล็อกตัวพิมพ์ใหญ่ · UI แสดง `ตัวใหญ่`), LB = Ctrl** — [[13-shift-toggle]]; RB = คลิกซ้าย, RT = คลิกขวา
4. **D-pad ← = วงคำสั่งในวงใน 8 ช่อง** (แตะ = เปิด/ปิด) — **เลือกด้วย RS, ปล่อย RS = ยืนยัน** (A = ทำ, B = ยกเลิก, LS ไม่มีผลในวง) — [[10-command-wheel-rs]]
5. **วงใน (ตอนพิมพ์) = สิ่งที่ LS เลือก · วงนอก = สิ่งที่ RS เลือก**
6. **HUD ตรึงที่ pivot ไม่ตามเมาส์** — Nav = โหมดจัดตำแหน่ง UI (LS หยาบ · RS ละเอียด · R3/R กลับตำแหน่ง) — [[11-hud-pivot-nav]]
7. **ภาษาไทย**: วรรณยุกต์ ่ ้ ๊ ๋ อยู่กลุ่ม ↘ เดียวกัน; สระล่าง ุ ู มี **ขีดเส้นประ** — [[12-thai-tone-and-below-vowel]]
8. **Universal languages (v0.11)**: ตรวจ layout ในเครื่องก่อน → เครื่องไทย = EN+TH; EN/TH ใช้ตารางออกแบบ, อื่นๆ map จาก QWERTY + แก้ได้; **IME = ส่งคีย์เหมือนคีย์บอร์ดจริง** — [[14-universal-languages]]
9. **Settings = หน้าต่างแยก ภาษาอังกฤษ** (General/Languages/Keymap/HUD) + Reset all to defaults
10. **กด RS = pointer กลับกลางจอหลัก** — เฉพาะโหมดเมาส์
11. **ไม่ใช้ Godot** — [[02-engine-choice-godot]] · 12. **ภาษาหลัก = Rust + Tauri v2** — [[03-stack-rust]]
13. ไฟล์ออกแบบอยู่ใน `Design/` + sub-folder (ตาม canvas)
14. **Typography แยก 2 ตัวคูณ**: `ui.hudScale` (วง, **1.0 = 340px**) ≠ `ui.hudTextScale` (ตัวหนังสือ, **1.4**) · ฟอนต์ไทย **ไม่มีหัว = IBM Plex Sans Thai** (fallback Noto Sans Thai) · ส่งค่าด้วย event `hud:style` (ไม่ใช่ HudState) · Settings มี slider **Text size / Ring size** + ฟอนต์ 3 ตัว — [[16-hud-typography-and-fonts]] · [[17-hud-ring-geometry-and-active-animation]]
15. **pivot/pointer ใน HudState = พิกัดในหน้าต่าง overlay** (0,0 = virtual desktop) — app แปลงจากพิกัดหน้าจอก่อน emit
16. **ป้ายกลุ่มไทยยาว ≥ 7 grapheme = 2 บรรทัด** · ขีดสระล่าง = เส้นประ (ไม่ใช่คำ) · **active เน้นด้วยสี/stroke/ขนาดฟอนต์เท่านั้น — ห้าม `transform: scale`** (wedge 45° จะทับกันจนวงเพี้ยน) — [[17-hud-ring-geometry-and-active-animation]] · [[19-revert-sector-scaling-and-shift-labels]]
17. **`shiftLock` เปิด → ตัวอักษร a–z บนวงเป็น A–Z** (แอปแปลงก่อนส่ง) + `status.text = "ตัวใหญ่"` — [[13-shift-toggle]] · [[19-revert-sector-scaling-and-shift-labels]]
18. **HUD ต้องได้ state เสมอ**: resend ตอน handshake + heartbeat 2 วิ · **ห้ามเปิดแอปซ้ำ** (named mutex) — [[20-hud-state-delivery]]
19. **Ctrl + ตัวอักษร ASCII = VirtualKey** (ไม่ใช่ Unicode) ไม่งั้น Copy/Paste/Undo/Redo ไม่ทำงาน — [[18-runtime-wiring]]
20. **P2 slice 3**: Mouse = สูตร deadzone 0.15/gamma 1.7/vmax 1600/smoothing 0.5 (LT ×0.3 · LB ×1.8) · Nav LS 900 / RS 320 px/s · auto-repeat 350/90 ms (ดัน RS กลับทิศเดิมหลัง commit) · layer 2 = RB/RT (ค้าง = ชั่วคราว · แตะ < 280 ms โดยไม่พิมพ์ = ล็อก) · A double-tap 280 ms = Enter (Space แรกออกแล้ว) · B hold 400 ms = Ctrl+Backspace · Y hold = Esc — [[21-mouse-nav-repeat-layer2]]
21. **Visual language (ADR-22)**: accent เดียว **Apple blue `#0A84FF`** · mode = Type blue / Mouse orange / Nav purple · HUD = แผ่นทึบ `.88` + ขอบสองชั้น **ไม่มี `backdrop-filter`** · active wedge = fill + stroke 2px + dim อื่น **0 ms** · pill 28px + dot สีโหมด + idle 3 วิ · Settings = sidebar 200px + inset group (760×640) · motion 80/140 ms + reduced-motion — [[05-visual-language]] · [[22-visual-refresh-apple-style]]
22. **P2 slice 4 (ADR-23)**: **Quick bar = LB ค้าง 150 ms** (แตะสั้นไม่เปิด · ปล่อย LB = ปิด · ดัน LS = ปิดแล้วพิมพ์ Ctrl+ตัวอักษร) ช่อง ↑Copy ↗Paste →Cut ↘Undo ↓Redo ↙Select All ←Save ↖Alt+Tab · **caret = R3** (RS = ลูกศร 350/90 ms · R3×2 < 280 ms = Shift+ลูกศร · ไม่มีวง เหลือกากบาท) · **sticky drag = RB 2 แตะ** ใน Mouse (แตะอีกครั้ง = ปล่อย; session ปล่อยปุ่มให้เมื่อสลับ mode) · **pivot clamp ฝั่งแอป** margin 172×`hudScale` จากขอบ virtual desktop · `KeyPress` มี `alt` แล้ว — [[23-quickbar-caret-sticky-drag-pivot-clamp]]
23. **P3 slice 2 (ADR-24)**: **exclusion/UIPI guard** ตรวจ foreground ทุก 250 ms → exe ใน `system.exclusion` หรือ target elevated → หยุด `session.step`, HUD `disabled` + เหตุผล, กลับมาเป็น mode เดิม · **scancode path อัตโนมัติ** (`VkKeyScanExW` + `MapVirtualKeyExW` ตาม HKL ของ foreground) → เกม/IME เห็นเหมือนคีย์บอร์ดจริง, พิมพ์ไม่ได้ = Unicode fallback, `--no-scancode` ปิดได้ · **`timeBeginPeriod(1)`** ระหว่างรัน loop — [[24-target-guard-scancode-timer]]
24. **P5 slice 2 (ADR-25)**: **General** = status (`get_status`/`app:status`) + Pause + **exclusion list UI** · **Languages** = `GetKeyboardLayoutList` → EN/TH built-in แสดงเสมอ, EN = base (locked), ภาษาอื่นต้องมี mapping, เปิดได้ ≤ 8 · **live reload** `config_rev` → runtime เปลี่ยนภาษา/pivot clamp ภายใน 250 ms · Reset แยก section (general/hud/languages/all) — [[25-settings-general-languages-live-reload]]
25. **P6 slice 1 (ADR-26)**: **WPM = 5 ตัวอักษร/คำ ในหน้าต่าง 60 วิ** (นับเฉพาะ `Key::Char`) · **action latency** = เวลา `session.step` (avg/max ต่อวินาที) · log `[stats]` ทุก 10 วิ + การ์ด Usage ใน Settings General (`get_stats`) — [[26-usage-stats-wpm-latency]]
26. **P5 slice 3 (ADR-27)**: **Keymap editor** (แท็บจริง) — override รายช่องใน `languages[].overrides` key `<layer>-<group>-<sector>` (รูปเก่า `<g>-<s>` = layer 1) · core override ชนะตาราง static ทั้งพิมพ์จริง/preview · `get_keymap`/`set_keymap_cell`/`reset_keymap` + live reload · แก้ได้เฉพาะ EN/TH (ภาษาอื่นรอ universal QWERTY) — [[27-keymap-editor-overrides]]

## 3. โครงสร้างไฟล์

```
project menager/
├─ Requirment.canvas / Research.canvas     ← โจทย์ตั้งต้น (ใน vault)
└─ Design/
   ├─ HANDOFF.md              ← ไฟล์นี้
   ├─ README.md               ← ดัชนี + เป้าหมาย + TL;DR
   ├─ 2D/                     ← ภาพจอย Xbox/PS5
   ├─ 01-research/01-prior-art.md
   ├─ 02-design/
   │  ├─ 01-interaction-model.md
   │  ├─ 02-keymap-en-th.md   ← ตาราง 8×8 EN/TH (สำคัญที่สุด)
   │  ├─ 03-mouse-and-navigation.md
   │  ├─ 04-state-machine-spec.md  ← ค่าเวลา/pseudocode
   │  ├─ 05-visual-language.md     ← สรุป UX/UI ที่ใช้จริง (ADR-22)
   │  └─ 05a-claude-ux-consult.md  ← คำตอบดิบจาก Claude (2026-11-16)
   ├─ 03-engineering/
   │  ├─ 01-architecture.md
   │  ├─ 02-roadmap-and-smoke-tests.md   ← ตารางวัดผล
   │  ├─ 03-parallel-execution-plan.md
   │  └─ contracts/           ← P1 freeze: hud-state.schema, config.schema, ipc.md (v1.1), samples
   ├─ 04-decisions/           ← ADR-01…ADR-22 (ดูรายการใน README)
   │  ├─ 16-hud-typography-and-fonts.md      ← ADR-16 typography + `hud:style`
   │  ├─ 17-hud-ring-geometry-and-active-animation.md ← ADR-17 geometry (§5 ถูกยกเลิก)
   │  ├─ 18-runtime-wiring.md ← ADR-18 จอย → core → injector → HUD
   │  ├─ 19-revert-sector-scaling-and-shift-labels.md ← ADR-19 เลิก scale + Shift→A-Z
   │  ├─ 20-hud-state-delivery.md ← ADR-20 resend/heartbeat/single instance
   │  ├─ 21-mouse-nav-repeat-layer2.md ← ADR-21 mouse/Nav math · repeat · layer 2 · double-tap/hold
   │  ├─ 22-visual-refresh-apple-style.md ← ADR-22 tokens/HUD plate/Settings sidebar
   │  ├─ 23-quickbar-caret-sticky-drag-pivot-clamp.md ← ADR-23 quick bar/caret/sticky drag/pivot clamp
   │  ├─ 24-target-guard-scancode-timer.md ← ADR-24 exclusion/UIPI · scancode · 1 ms timer
   │  ├─ 25-settings-general-languages-live-reload.md ← ADR-25 General/Languages · exclusion UI · live reload
   │  ├─ 26-usage-stats-wpm-latency.md ← ADR-26 WPM 60 วิ · action latency · Usage cards
   │  └─ 27-keymap-editor-overrides.md ← ADR-27 Keymap editor · override รายช่อง (EN/TH)
   └─ 05-mockup/
      ├─ hud-prototype/        ← ม็อกอัป HUD v0.11 (อ้างอิงดีไซน์ ไม่ใช่ของจริง)
      └─ hud-shots/            ← screenshot ตรวจรับ (100%/150% DPI, overlay จริง, Settings, runtime)
```

### 3.1 ไฟล์โค้ดที่ใช้บ่อย

| path | หน้าที่ |
| --- | --- |
| `crates/rnitype-app/src/main.rs` | overlay window + tray + Settings + IPC + **runtime loop** |
| `crates/rnitype-app/src/session.rs` | `Session`: engine → injector → `HudState` (ทดสอบด้วย `NullInjector` ได้) |
| `crates/rnitype-app/src/input.rs` | `PadSource`: `GilrsSource` (จอยจริง) + `ScriptSource` (`--fake-pad`) |
| `crates/rnitype-app/src/config.rs` | `%APPDATA%\RniType\config.json` + `ui_style()` / `hud_scale()` / `validate_ui()` |
| `crates/rnitype-core/src/{runtime,typing,keymap,gesture,normalizer}.rs` | state machine, keymap EN/TH (ชั้น 1+2), sector tracker, auto-repeat |
| `crates/rnitype-core/src/pointer.rs` | Mouse math (deadzone/gamma/vmax/smoothing) · Nav pivot · scroll |
| `crates/rnitype-win/src/plan.rs` | `Action` → SendInput plan (**Ctrl+ASCII = VK** · `plan_scancode`) |
| `crates/rnitype-win/src/sendinput.rs` | Windows sender: Unicode/VK/**scancode** ของ HKL เป้าหมาย |
| `crates/rnitype-win/src/target.rs` | foreground exe + elevated (exclusion/UIPI guard) |
| `crates/rnitype-win/src/timer.rs` | `timeBeginPeriod(1)` แบบ RAII |
| `crates/rnitype-win/src/language.rs` | `GetKeyboardLayoutList` → code/label/kind (layout vs IME) |
| `crates/rnitype-core/src/keymap.rs` | ตาราง EN/TH 8×8 + ชั้น 2 + ชื่อกลุ่ม (static default) |
| `crates/rnitype-core/src/typing.rs` | gesture → key lookup + **override รายช่อง** (`BTreeMap`) |
| `crates/rnitype-app/src/stats.rs` | WPM (5 ตัวอักษร/คำ) · format latency/uptime |
| `ui/hud/{index.html,hud.css,hud.js}` | HUD จริง — วาดจาก `hud:state` · typography จาก `hud:style` |
| `ui/settings/*` | Settings window — General (status/pause/exclusion/usage) · Languages (rescan) · Keymap (editor) · HUD · Reset |
| `ui/shared/{tokens.css,fonts.css,fonts.js,fonts/*.ttf}` | สี + ฟอนต์ (IBM Plex Sans Thai + Noto Sans Thai + OFL) |
| `tools/tests/{browser,hud-smoke,hud-typography,settings-smoke}.mjs` | CDP · 54 mockup · 36 HUD · 19 Settings checks |

## 4. สถานะงานและลำดับถัดไป

> แผนลงมือละเอียด (7 เฟส · 4 track) อยู่ที่ [[03-parallel-execution-plan]]

### 4.1 P0 + P1 — ✅ ปิดแล้ว

- A: **gilrs XInput** อ่าน Xbox ครบ + 8 sectors · ยืนยันซ้ำ 2026-11-15 (probe 20 วิ, "Xbox Controller"):
  LSx ±0.99 · LSy ±1.00 · RSx ±0.99 · RSy ±1.00 · idle_peak 0.000 · **LZ/RZ = 0.00** (LT/RT เป็นดิจิทัล)
- B: overlay โปร่งใส คลิกทะลุ ไม่แย่ง focus (2 จอ) · C: SendInput Unicode ไทยลง Notepad ถูก
- contracts + workspace + state player + 54/54 mockup checks

### 4.2 Track ที่ทำแล้ว / เหลือ

| Track | ทำแล้ว | เหลือ |
| --- | --- | --- |
| P2 core | วงคำสั่ง/วงภาษา/mode/shift/face buttons + **slice 3: mouse math, Nav pivot, auto-repeat, layer 2, double-tap/hold** + **slice 4: quick bar, caret nav, sticky drag, pivot clamp** (98 tests) | — (P2 ปิด; เหลือแค่ mixed-DPI) |
| P3 win | SendInput Unicode/VK/mouse + **HWheel** + Ctrl+ASCII→VK + **Alt** + **scancode auto** + **exclusion/UIPI guard** + **1 ms timer** (56 tests) | Task Scheduler/"Run as admin" (P6) |
| P4 HUD | renderer ครบ mode + overlay + typography + **visual refresh ADR-22** + **quickbar/caret mode** + `HudState` จริง (+ `layer`/`repeat`/`pointer`) (36 checks) | latency, pivot per-monitor/mixed-DPI |
| Runtime | `input.rs` + `session.rs` + tray Pause + resend/heartbeat + single instance + **mouse/Nav recenter/reset** + **sticky-drag safety release** + **pivot clamp** + **guard/pause คืน mode เดิม** + **live config reload** + **WPM/commit stats** | ทดสอบ mouse/Nav/quickbar/caret กับจอยจริง |
| P5 Settings | slice 1: หน้าต่าง + HUD tab + persist · slice 2: **General + Languages + Reset + live reload** · slice 3: **Keymap editor (EN/TH, override รายช่อง)** (28 checks) | universal QWERTY mapping (ภาษาอื่น) · profiles · autostart/runAsAdmin จริง |
| P6 measurement | slice 1: **WPM 60 วิ + action latency + uptime** (`[stats]` log + Usage cards) | error rate, CPU, คนจริง, installer/Steam/itch |

### 4.3 ขั้นถัดไป (เรียง)

1. **universal QWERTY mapping** — `ToUnicodeEx` ตาม HKL ของ layout ที่ติดตั้ง → ตาราง 8×8 สำหรับ RU/EL ฯลฯ แล้วเปิด toggle ใน Languages/Keymap
2. **P6 ที่เหลือ** — error rate (reference text/diff), CPU/RAM, ทดสอบคนจริง, installer/Steam/itch, Task Scheduler/"Run as admin"
3. **P2 ที่เหลือ (เล็ก)** — pivot clamp แบบ per-monitor/mixed-DPI (ตอนนี้ clamp ด้วย virtual desktop เดียว)

### 4.4 รัน / ตรวจ

```powershell
crates\rnitype-app\run.ps1 --seconds 120              # จอยจริง
crates\rnitype-app\run.ps1 --fake-pad --seconds 20    # จอยปลอม (จะพิมพ์ลงหน้าต่างที่ focus)
crates\rnitype-app\run.ps1 --pad-log --seconds 60     # log ค่าจอยทุก 250 ms
crates\rnitype-app\run.ps1 --no-scancode --seconds 30 # บังคับ Unicode path (ถ้าเกม/IME มีปัญหา)
crates\rnitype-app\run.ps1 --demo --settings --seconds 30  # HUD จาก state ตัวอย่าง
```

```powershell
cargo test --workspace                      # 213 tests
node tools/contracts/validate.mjs           # 12/12
node tools/tests/hud-smoke.mjs              # 54/54 (ม็อกอัป)
node tools/tests/hud-typography.mjs         # 36/36 + screenshot
node tools/tests/settings-smoke.mjs         # 28/28 + screenshot Settings
```

- Tray: **คลิกซ้าย = Settings** · คลิกขวา = Pause typing / Settings… / Quit
- typography/ขนาดวง: Settings → เปลี่ยนแล้ว HUD เปลี่ยนทันที · เก็บที่ `%APPDATA%\RniType\config.json`

### 4.5 Feedback รอบ 1–2 (2026-11-15) — ✅ ปิดแล้ว (ADR-16 · ADR-17)

| เรื่อง | ผล |
| --- | --- |
| ตัวหนังสือเล็ก/อ่านไม่ชัด + ไทยต้องไม่มีหัว | ฟอนต์ **IBM Plex Sans Thai** + text scale (group 16px, char 25px, pill 14px) · contrast ตัวอักษรจาง **6.87:1** |
| Settings ต้องปรับขนาด/ฟอนต์ได้ | slider Text size + Ring size + ฟอนต์ 3 ตัว + Reset · persist |
| "ขีดล่าง" ใต้ ุ ู | เปลี่ยนเป็น **เส้นประ** เหนือสระล่าง |
| วงใหญ่ไปนิด | 408 → **340px** · วงในกว้างขึ้น + ป้ายไทยยาวแบ่ง 2 บรรทัด |
| เปิด Settings ตรงไหน | คลิกซ้าย tray / คลิกขวา → Settings… / `--settings` |

บั๊กที่เจอระหว่างทาง: `getBBox()` ไม่ serialize ผ่าน CDP → test เดิมรายงาน "ป้ายไม่ชน" ทั้งที่ชนกัน 6% → เปลี่ยนไปวัด **ink box** (30 checks)

### 4.6 Feedback รอบ 3 (2026-11-15) — ✅ ปิดแล้ว (ADR-19 · ADR-20)

| แจ้ง | สาเหตุ | ผล |
| --- | --- | --- |
| Animation ทำให้วงเพี้ยน | `scale` wedge 45° → กินมุม 48° ทับกัน | ยกเลิก transform · active = สี/stroke/ขนาดฟอนต์ · มี check กัน regression |
| กด Shift ตัวอักษร EN ไม่เป็นตัวใหญ่ | แอปส่ง label ดิบ (ตัวเล็ก) | แปลง `a–z→A–Z` ที่ฝั่งแอป + `status.text = "ตัวใหญ่"` |
| UI ค้างจอตลอดเวลา | state ชุดแรกยิงก่อน HUD พร้อม + dedupe → ไม่มี resend | `hud_ready` บังคับ resend + heartbeat 2 วิ + single instance |

### 4.7 UX/UI refresh (2026-11-16) — ✅ ปิดแล้ว (ADR-22)

| เรื่อง | ผล |
| --- | --- |
| Owner ขอ UX/UI ทันสมัยแบบ Apple/macOS | ปรึกษา Claude Fable 5.2 (medium) ผ่าน CLI → [[05a-claude-ux-consult]] · สรุปที่ [[05-visual-language]] |
| Design tokens กระจัดกระจาย | `ui/shared/tokens.css` เป็นแหล่งเดียว: Settings/HUD/mode/radius/spacing/elevation/motion + alias เดิม |
| HUD อ่านยากบนพื้นหลังสว่าง | แผ่นทึบ `.88` + ขอบสองชั้น (ขาวใน/ดำนอก) · **เอา `backdrop-filter` ออก** |
| Active state ต้องเร็ว | fill accent + stroke 2px + dim glyph อื่น .45 · highlight **0 ms** · ไม่ scale wedge |
| Mode สับสน | Type = blue · Mouse = orange · Nav = purple · pill มี dot สีโหมด + badge Shift/Layer |
| Pill ดูเป็น dev tool | 28px · radius 999 · 12–14px/600 · idle 3 วิ → opacity .6 (heartbeat 2 วิ ไม่รีเซ็ต timer) |
| Settings ดูเก่า | sidebar 200px + inset group + row 44px + preview ใหม่ · window 760×640 (min 680×480) |

### 4.8 งานค้างยืนยัน (เริ่ม session หน้าด้วยข้อนี้)

1. **owner retest หลัง ADR-20**: `run.ps1 --seconds 120` → เปิด Notepad → LS↑ + RS↑ = `ก` → **ปล่อยจอยแล้ววงต้องหาย** (เหลือ pill)
   - ถ้ายังค้าง: เก็บ `--pad-log` + ต่อ CDP ดู DOM (สูตรอยู่ §6) แล้วรายงาน `data-rings` กับ `#innerRing` child count
2. **จอยจริง**: ต่อได้ + พิมพ์ได้จริงแล้ว (`sent=11` ใน log) แต่ยังไม่ได้ตรวจ LT toggle / RT คลิกขวา / View / Menu ครบทุกปุ่ม
3. **LT/RT ดิจิทัล** (LZ/RZ = 0): precision hold ในโหมดเมาส์ต้องอ่านจากปุ่ม ไม่ใช่ analog
4. **P2 slice 3 (ใหม่)**: ตรวจกับจอยจริง — D-pad ← → Mouse: RS เลื่อน pointer, RB คลิกซ้าย (กดค้างลากได้), RT คลิกขวา, RS click กลับกลางจอ, LS scroll
   - Nav (`↘ Place UI`): LS/RS ขยับ pivot, RS click กลับจุดเริ่มต้น
   - Type: แตะ RB = ล็อก layer 2 (ตัวอักษรเป็น A–H) · กด RB ค้างแล้วพิมพ์ = ชั่วคราว · ค้าง RS ทิศเดิมหลังพิมพ์ = repeat (HUD `×n`)
   - A แตะ 2 ครั้ง = Enter · กด B ค้าง 400 ms = ลบทั้งคำ · กด Y ค้าง = Esc
5. **Visual refresh (ADR-22)**: เปิด Settings → ตรวจ sidebar/inset group/preview · เปิด HUD แล้วดูว่าแผ่น `.88` อ่านชัดบนทั้งพื้นมืด/สว่าง · pill มี dot สีโหมดและจางหลัง idle 3 วิ
   - ถ้าอยากให้แผ่นบางลง/เข้มขึ้น ปรับ `--hud-plate` ที่ `ui/shared/tokens.css` แล้วแจ้งค่าที่ชอบ
6. **Quick bar (ADR-23)**: Type → กด LB ค้าง ~150 ms → แถบ 8 ช่อง (Copy/Paste/Cut/Undo/Redo/All/Save/Switch) → RS ↑ แล้วปล่อย = Ctrl+C · ปล่อย LB = ปิดไม่มีอะไรเกิดขึ้น · กด LB + ดัน LS = พิมพ์ Ctrl+ตัวอักษร
7. **Caret nav (ADR-23)**: Type → กด R3 = pill `Caret · ลูกศร` (ไม่มีวง) → RS ←/→ เลื่อน caret · ค้าง = repeat · กด R3 เร็ว ๆ 2 ครั้ง = `เลือกข้อความ` แล้ว RS = Shift+ลูกศร · กด R3 ครั้งเดียว (พ้น 280 ms) = ปิด
8. **Sticky drag (ADR-23)**: Mouse → แตะ RB 2 ครั้งเร็ว ๆ → badge `Drag` → RS ลาก → แตะ RB อีกครั้ง = ปล่อย (ตรวจว่าไม่มีเมาส์ค้าง)
9. **Pivot clamp (ADR-23)**: Nav → ดัน LS จนสุดขอบจอ → วงต้องหยุดก่อนล้นขอบ (margin 172×hudScale) · R3 = กลับจุดเริ่มต้น
10. **Scancode (ADR-24)**: เปิด Notepad (layout EN) → พิมพ์ `a` → ต้องได้ `a` · สลับ layout เป็นไทยแล้วพิมพ์ `ก` → ต้องได้ `ก` · ถ้าเกมอ่านคีย์ไม่ได้ ให้ลอง `run.ps1 --no-scancode` เพื่อเทียบ
11. **Exclusion guard (ADR-24)**: เพิ่ม `"system": { "exclusion": ["notepad.exe"] }` ใน `%APPDATA%\RniType\config.json` → โฟกัส Notepad → pill ต้องเป็น `Paused · แอปนี้อยู่ในรายการกัน` และพิมพ์ไม่ออก · สลับไปหน้าต่างอื่น → กลับมาพิมพ์ได้ mode เดิม
12. **UIPI (ADR-24)**: เปิด Task Manager (elevated) แล้วโฟกัส → pill `Paused · ต้องรันแบบ admin` · ปิด Task Manager → กลับมาทำงาน
13. **Timer (ADR-24)**: เปิด `--pad-log` ต้องเห็น `[timer] 1 ms resolution: เปิด` และ `[pad-log]` อัปเดตทุก ~250 ms
14. **Settings General (ADR-25)**: เปิด Settings → Controller ต้องขึ้นชื่อจอย · ติ๊ก Pause → HUD/pill เป็น Paused · เพิ่ม `notepad.exe` ใน Exclusion → โฟกัส Notepad แล้วพิมพ์ไม่ออก pill บอกเหตุผล · ลบออกแล้วพิมพ์ได้
15. **Settings Languages (ADR-25)**: เปิดแท็บ Languages → ต้องเห็น EN (Detected/Locked) + TH (Built-in) · ปิด TH → log `[config] runtime reload: languages=0` และวงภาษาไม่มี TH · เปิดกลับ → มี TH
16. **Reset (ADR-25)**: Reset languages → config `languages: []` (EN/TH กลับมา) · Reset general **ต้องไม่ลบ exclusion** · Reset all คืนทั้งไฟล์
17. **Usage stats (ADR-26)**: เปิด Settings → General → พิมพ์ด้วยจอยจริง ~1 นาที → Characters/WPM ต้องขยับ · log ต้องมี `[stats] chars=… wpm=… step avg=… max=… uptime=…` ทุก 10 วิ · รายงาน WPM ไทย/อังกฤษเทียบเป้า 12/20
18. **Keymap editor (ADR-27)**: Settings → Keymap → เลือก TH → กลุ่ม 1 → คลิกช่อง A (บนกลาง) → พิมพ์ `z` → Enter → จุดฟ้า + พิมพ์ด้วยจอยได้ `z` · Reset group → กลับ `ก` · สลับ Base/Layer 2 แล้วแก้ layer 2 ได้ · Restart แอปแล้วค่าต้องอยู่

### 4.9 P2 slice 4 (2026-11-16) — ✅ ปิดแล้ว (ADR-23)

| เรื่อง | ผล |
| --- | --- |
| Quick bar (LB ค้าง 8 ช่อง) | `WheelKind::QuickBar` — เปิดหลังค้าง 150 ms · RS เลือก · ปล่อย RS = คีย์ลัด · ปล่อย LB/D-pad ←/B = ยกเลิก · LS = Ctrl+ตัวอักษร · HUD เป็นแถบแนวนอน 8 ช่อง · Alt+Tab ใช้ `alt` ใหม่ใน `KeyPress` |
| Caret nav (R3) | 350/90 ms ตาม state machine §4 · R3×2 = Shift+ลูกศร · ไม่มีวง (pill `ลูกศร`/`เลือกข้อความ`) · A/B ออกจากโหมดโดยไม่พิมพ์ทับ selection · หน้าต่าง double-tap รีเซ็ตเมื่อออกจาก Type |
| Sticky drag (RB 2 แตะ) | 280 ms window · แตะครั้งที่ 3 = ปล่อย · `Session` ปล่อยปุ่มซ้ายให้ถ้า engine ล้างค่า (สลับ mode/emergency) · badge `Drag` |
| Pivot clamp | `Session::set_pivot_clamp` — margin 172×`hudScale` รอบ virtual desktop · clamp ตอน Move/Reset/เริ่มต้น |
| Contract | `mode` เพิ่ม `quickbar`/`caret` · sample +2 → **12/12** · HUD แสดง quickbar ด้วย `inner.kind = command` |
| ตัวเลข | cargo **166** (core 98 · win 37 · app 31) · typography **36/36** · hud-smoke 54/54 |

### 4.10 P3 slice 2 (2026-11-16) — ✅ ปิดแล้ว (ADR-24)

| เรื่อง | ผล |
| --- | --- |
| Exclusion / UIPI guard | `target.rs`: foreground exe (basename, wildcard `*?`) + token elevated · ตรวจทุก 250 ms · ถูกกัน = ไม่เรียก `session.step`, HUD `disabled` + `แอปนี้อยู่ในรายการกัน` / `ต้องรันแบบ admin` · กลับมาแล้วคืน mode เดิม |
| Scancode path | `plan_scancode` + `VkKeyScanExW`/`MapVirtualKeyExW` ตาม HKL ของ foreground · Ctrl/Alt chord + NamedKey ยังเป็น VK · พิมพ์ไม่ได้ = Unicode fallback · `--no-scancode` ปิด |
| Timer | `timeBeginPeriod(1)` แบบ RAII ใน runtime thread · log `[timer] 1 ms resolution: เปิด` |
| Bug fix | tray Pause/unpause คืน mode เดิม (เดิม unpause แล้วพิมพ์ต่อไม่ได้) + ไม่รีเซ็ต deadman ระหว่าง emergency |
| ตัวเลข | cargo **188** (core 98 · win 53 · app 37) · log จริง: `[timer] เปิด`, `[inject] auto scancode`, พิมพ์ไทยผ่าน |

### 4.11 P5 slice 2 (2026-11-16) — ✅ ปิดแล้ว (ADR-25)

| เรื่อง | ผล |
| --- | --- |
| General tab | status grid (controller/typing/version) จาก `get_status` + `app:status` · Pause toggle (`set_paused`) · exclusion list เพิ่ม/ลบผ่าน `save_config` · Reset general/all |
| Languages tab | `language.rs` อ่าน HKL จริง · EN/TH เป็น **built-in** แสดงเสมอ (EN locked = base) · ภาษาอื่น/IME แสดง badge + ปิด toggle จนมี mapping · เปิดได้ ≤ 8 |
| Live reload | `AppState.config_rev` → loop reload `set_enabled_languages` + pivot clamp ทุก 250 ms · log `[config] runtime reload: languages=N hudScale=X` |
| Reset | `reset_config { section }` — general ไม่ล้าง exclusion · languages กลับ EN/TH · hud เฉพาะ typography · all = default |
| ตัวเลข | cargo **201** (core 98 · win 56 · app 47) · settings-smoke **19/19** · E2E WebView จริง: ปิด TH → `languages=0`, เปิด → `languages=1`; exclusion add/remove เขียน config ถูก |

### 4.12 P6 slice 1 (2026-11-16) — ✅ ปิดแล้ว (ADR-26)

| เรื่อง | ผล |
| --- | --- |
| WPM | `Session` เก็บ commit 60 วิ (เฉพาะ `Key::Char`) → WPM = chars/5 · ไม่นับปุ่ม named/เมาส์/คีย์ลัด |
| Action latency | วัดเวลา `session.step` (state machine + SendInput) เป็น avg/max ต่อวินาที |
| Log | `[stats] chars=15 wpm=3.0 actions=15 step avg=0.005 ms max=0.748 ms uptime=10s` (ทุก 10 วิ) |
| Settings | General → Usage 4 ช่อง (Characters / WPM · 60 s / Action latency / Uptime) ผ่าน `get_stats` ทุก 1 วิ |
| E2E จริง | WebView อ่านการ์ดได้ `chars=75 wpm=15.0 latency=0.004/0.85 ms uptime=51s` (fake-pad) |
| ตัวเลข | cargo **205** (core 98 · win 56 · app 51) · settings-smoke **20/20** |

### 4.13 P5 slice 3 (2026-11-16) — ✅ ปิดแล้ว (ADR-27)

| เรื่อง | ผล |
| --- | --- |
| Override storage | `languages[].overrides` key `<layer>-<group>-<sector>` (รูปเก่า `<g>-<s>` = layer 1) · value 1 ตัวอักษร, `""` = ช่องว่าง (layer 2) |
| Core | `TypingPipeline` เก็บ `BTreeMap<(Lang,layer,group,sector), Option<Key>>` · override ชนะ static ทั้งพิมพ์จริงและ HUD preview · แยกตามภาษา |
| Runtime | `build_session` + live reload ใช้ `all_keymap_overrides` → แก้แล้วมีผลภายใน 250 ms |
| Commands | `get_keymap` · `set_keymap_cell` · `reset_keymap` (+ `validate_keymaps` ใน `save_config`) |
| UI | แท็บ Keymap จริง: 8 กลุ่ม + กริด 3×3 ตามทิศ RS + Base/Layer 2 + input inline (Enter/Esc) + จุด modified + Reset group/language |
| E2E จริง | `[keymap] th L1 G0 S0 = "z"` → runtime พิมพ์ `[type] z` → reset → กลับ `[type] ก` |
| ตัวเลข | cargo **213** (core 100 · win 56 · app 57) · settings-smoke **28/28** · schema 12/12 |

## 5. คำถามที่ยังค้าง

| # | เรื่อง | ข้อเสนอ |
| --- | --- | --- |
| Q4 | Alt อยู่ปุ่มไหน | hold-Y = Alt (หรือ View/Share) |
| Q7 | layout ไทย | ความถี่ (ที่ออกแบบ) ก่อน, เลือก Kedmanee ได้ |
| Q10 | รองรับจอยอื่น | XInput สูงสุด 4 ตัว; SDL3 ภายหลัง |

ที่เหลือ (Q3, Q6, Q8, Q9, Q11–Q14) มีข้อเสนอชัดเจนใน [[01-open-questions]] — ใช้ค่าเริ่มต้นได้เลย
Q1, Q2, Q5, Q15, Q16, Q17 = ตัดสินแล้ว

## 6. Environment & เคล็ดลับ debug

- vault = `D:\Erin's Works\Programming\RniType` — เขียนเฉพาะในนี้ · MCP `nizylanote` เรียก `vault_info` ก่อนเขียน
- **Git**: commit ก่อน session นี้ `4cba441` (handoff v0.8) · P5 slice 2 = `d3f17a2` · P6 slice 1 = `c3a7404` · P5 slice 3 = `741a60c` (+ docs commit ถัดจากนั้น) · commit งานเป็นชุด `feat(...)` + `docs(...)` (`.nznote/`, `.opencode/` ถูก ignore)
- **เทอร์มินัล cp874** → log ไทยใน console เป็น `???` (ไม่ใช่บั๊ก) · ไฟล์เป็น UTF-8
- **PowerShell กิน `--` ตัวแรก** → `run.ps1` เติมให้แล้ว ใช้ `run.ps1 --seconds 30`
- `Chrome_WidgetWin_0 ... 1412` ตอนปิด = WebView2 noise ไม่มีผล
- **scancode path** ใช้ HKL ของหน้าต่างที่ focus — ถ้าเกม/IME พิมพ์เพี้ยน ใช้ `--no-scancode` แล้วรายงาน HKL กับชื่อเกม
- **guard** ตรวจทุก 250 ms — ระหว่างถูกกัน `[pad-log]` ยังเดินแต่ไม่มี `[type]` ออก
- **Settings → runtime**: เปลี่ยนภาษา/ขนาดวงมีผลภายใน 250 ms ผ่าน `config_rev` · ถ้าไม่เห็น log `[config] runtime reload` แปลว่า command ไม่ได้เขียน config
- **Languages บนเครื่องนี้**: Windows มี layout EN ตัวเดียว → TH ขึ้นเป็น `Built-in` (ตารางในแอป) ไม่ใช่ `Detected`
- **WPM จาก `--fake-pad`** (~15–17) เป็นแค่การพิสูจน์ท่อ ไม่ใช่ตัวเลขผู้ใช้จริง — owner ต้องวัดกับจอย
- **latency** วัดฝั่งเรา (`session.step`) ไม่รวมเวลา OS/เกมประมวลผลคีย์
- **Keymap editor** แก้ได้เฉพาะ EN/TH (มีตารางใน core) · ภาษาอื่นยัง `Needs mapping` จนมี universal QWERTY
- override เก็บเป็น diff (`<layer>-<g>-<s>`) ไม่ใช่ตารางเต็ม — config ที่เขียนมือต้องใช้รูปแบบนี้
- **Chrome headless**: `C:\Users\User\AppData\Local\ms-playwright\chromium-1208\chrome-win64\chrome.exe` (`CHROME_PATH`)
- เครื่องนี้ 2 จอ: virtual desktop 3840×1080 เริ่มที่ x = −1920 · จอหลัก 1920×1080 (pivot เริ่มที่กลางจอหลัก)

**ต่อ CDP เข้า HUD/แอปจริง** (ใช้ไล่บั๊ก UI):

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9224"
crates\rnitype-app\run.ps1 --seconds 120
# แล้วเปิด http://127.0.0.1:9224/json/list → ได้ webSocketDebuggerUrl ของ hud/settings
```

- target ของ Settings = URL ที่ลงท้าย `settings/index.html` — `Runtime.evaluate` สั่งคลิกแท็บ/switch ได้จริง
  (ใช้ตรวจ ADR-25: ปิด TH แล้วอ่าน `[config] runtime reload` ใน log ของแอป)

อ่านสถานะ HUD ที่แท้จริง (ใช้ `Runtime.evaluate`):

```js
const h = document.getElementById('hud');
({ state: h.dataset.state, rings: h.dataset.rings, mode: h.dataset.mode,
   visible: h.dataset.visible, inner: document.querySelectorAll('#innerRing > *').length,
   ringOpacity: getComputedStyle(document.querySelector('.hud__ring')).opacity })
// idle ที่ถูกต้อง = { rings:'off', inner:0, ringOpacity:'0', visible:'true' }
```

## 7. นิยาม "เสร็จ" ของ session ถัดไป

1. ทุก check ผ่านและมีตัวเลขแนบ: `cargo test --workspace` · contracts · hud-smoke · hud-typography
2. งานที่แตะ runtime/overlay ต้องมี log จริง + screenshot/หลักฐาน (ไม่ใช่ "น่าจะได้")
3. อัปเดตตารางวัดผลใน [[02-roadmap-and-smoke-tests]]
4. เปลี่ยนการตัดสินใจ → เขียน ADR ใหม่ (อย่าแก้ ADR เก่า) + อัปเดต HANDOFF นี้

## 8. ตัวเลขอ้างอิงที่ใช้ตัดสินใจ

- TwoStick (จอย 2 แกน): 13–14.9 WPM หลัง 20 session · Selection keyboard บนจอ: 12.9 WPM
- Daisywheel: ผู้ใช้รายงาน 20–30 WPM (ยังไม่มีงานวิจัยควบคุม)
- เป้าเรา: **20 WPM EN / 12 WPM TH**, error < 1.5%, latency < 16 ms
