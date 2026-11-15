---
title: HANDOFF — RniType (อ่านก่อนเริ่ม session ใหม่)
status: v0.6
updated: 2026-11-16
tags: [handoff]
---

# HANDOFF — RniType v0.6 (2026-11-16)

> เอกสารนี้ทำให้ session ใหม่ทำงานต่อได้ใน \~5 นาที · อ่านจบ = รู้ทุกอย่างที่จำเป็น

## 1. สิ่งที่ต้องรู้ใน 30 วินาที

| หัวข้อ | ข้อสรุป |
| --- | --- |
| ผลิตภัณฑ์ | Windows background utility: จอย Xbox/DualSense ใช้ **พิมพ์แทนคีย์บอร์ด** และ **แทนเมาส์** มีหน้าตั้งค่าแบบ Logi Options+ |
| ช่องทางขาย | Steam + itch.io (ทั้งคู่รับ .exe/.zip — ไม่ต้องใช้เอนจิน) |
| สถานะ | **P0/P1 ปิด · P2 slice 3 · P3/P4/P5 slice 1 · runtime จอยจริงต่อได้และพิมพ์ได้ — 145 tests** |
| สแตก | **Rust** (core) + **Tauri v2** (UI/tray) + **gilrs** (จอย, XInput) + `SendInput` |
| แผนสำรอง | C# .NET 8 + WPF/WinUI (ถอยได้ทุกเมื่อ เอกสารใช้ร่วมกันได้) |
| งานถัดไป | **P2 slice 4**: quick bar (LB) + caret nav (R3) + sticky drag (RB 2 taps) + pivot clamp |
| ค้างยืนยัน | owner ต้อง retest หลังแก้ "UI ค้างจอ" (ADR-20) — ดู §4.7 |

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
   │  └─ 04-state-machine-spec.md  ← ค่าเวลา/pseudocode
   ├─ 03-engineering/
   │  ├─ 01-architecture.md
   │  ├─ 02-roadmap-and-smoke-tests.md   ← ตารางวัดผล
   │  ├─ 03-parallel-execution-plan.md
   │  └─ contracts/           ← P1 freeze: hud-state.schema, config.schema, ipc.md (v1.1), samples
   ├─ 04-decisions/           ← ADR-01…ADR-21 (ดูรายการใน README)
   │  ├─ 16-hud-typography-and-fonts.md      ← ADR-16 typography + `hud:style`
   │  ├─ 17-hud-ring-geometry-and-active-animation.md ← ADR-17 geometry (§5 ถูกยกเลิก)
   │  ├─ 18-runtime-wiring.md ← ADR-18 จอย → core → injector → HUD
   │  ├─ 19-revert-sector-scaling-and-shift-labels.md ← ADR-19 เลิก scale + Shift→A-Z
   │  ├─ 20-hud-state-delivery.md ← ADR-20 resend/heartbeat/single instance
   │  └─ 21-mouse-nav-repeat-layer2.md ← ADR-21 mouse/Nav math · repeat · layer 2 · double-tap/hold
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
| `crates/rnitype-win/src/plan.rs` | `Action` → SendInput plan (**Ctrl+ASCII = VK**) |
| `ui/hud/{index.html,hud.css,hud.js}` | HUD จริง — วาดจาก `hud:state` · typography จาก `hud:style` |
| `ui/settings/*` | Settings window (แท็บ HUD: Text size / Ring size / Font / Reset) |
| `ui/shared/{tokens.css,fonts.css,fonts.js,fonts/*.ttf}` | สี + ฟอนต์ (IBM Plex Sans Thai + Noto Sans Thai + OFL) |
| `tools/tests/{browser,hud-smoke,hud-typography}.mjs` | ตัวช่วย CDP · 54 checks (ม็อกอัป) · 30 checks (HUD จริง + screenshot) |

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
| P2 core | วงคำสั่ง/วงภาษา/mode/shift/face buttons + **slice 3: mouse math, Nav pivot, auto-repeat, layer 2, double-tap/hold** (84 tests) | quickbar, caret nav, sticky drag, pivot clamp/DPI |
| P3 win | SendInput Unicode/VK/mouse + **HWheel** + Ctrl+ASCII→VK (36 tests) | scancode/IME, exclusion/UIPI |
| P4 HUD | renderer ครบ mode + overlay + typography + `HudState` จริง (+ `layer`/`repeat`/`pointer`) (30 checks) | latency, pivot per-monitor/mixed-DPI |
| Runtime | `input.rs` + `session.rs` + tray Pause + resend/heartbeat + single instance + **mouse/Nav recenter/reset** | ทดสอบ mouse/Nav กับจอยจริง |
| P5 Settings | slice 1: หน้าต่าง + แท็บ HUD + persist config | แท็บ General/Languages/Keymap, rescan HKL, profiles |

### 4.3 ขั้นถัดไป (เรียง)

1. **P2 slice 4** — quick bar (LB ค้าง 8 ช่อง) · caret nav (R3 toggle + repeat) · sticky drag (RB 2 แตะ) · pivot clamp ตามขอบจอ
2. **P3 ที่เหลือ** — exclusion list/UIPI (กันเกม), scancode/IME path, `timeBeginPeriod(1)`
3. **P5 Settings ที่เหลือ** — General/Languages/Keymap + rescan HKL + keymap editor
4. **P6** — วัด WPM/latency, ทดสอบคนจริง, installer/Steam/itch

### 4.4 รัน / ตรวจ

```powershell
crates\rnitype-app\run.ps1 --seconds 120              # จอยจริง
crates\rnitype-app\run.ps1 --fake-pad --seconds 20    # จอยปลอม (จะพิมพ์ลงหน้าต่างที่ focus)
crates\rnitype-app\run.ps1 --pad-log --seconds 60     # log ค่าจอยทุก 250 ms
crates\rnitype-app\run.ps1 --demo --settings --seconds 30  # HUD จาก state ตัวอย่าง
```

```powershell
cargo test --workspace                      # 145 tests
node tools/contracts/validate.mjs           # 10/10
node tools/tests/hud-smoke.mjs              # 54/54 (ม็อกอัป)
node tools/tests/hud-typography.mjs         # 30/30 + screenshot
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

### 4.7 งานค้างยืนยัน (เริ่ม session หน้าด้วยข้อนี้)

1. **owner retest หลัง ADR-20**: `run.ps1 --seconds 120` → เปิด Notepad → LS↑ + RS↑ = `ก` → **ปล่อยจอยแล้ววงต้องหาย** (เหลือ pill)
   - ถ้ายังค้าง: เก็บ `--pad-log` + ต่อ CDP ดู DOM (สูตรอยู่ §6) แล้วรายงาน `data-rings` กับ `#innerRing` child count
2. **จอยจริง**: ต่อได้ + พิมพ์ได้จริงแล้ว (`sent=11` ใน log) แต่ยังไม่ได้ตรวจ LT toggle / RT คลิกขวา / View / Menu ครบทุกปุ่ม
3. **LT/RT ดิจิทัล** (LZ/RZ = 0): precision hold ในโหมดเมาส์ต้องอ่านจากปุ่ม ไม่ใช่ analog
4. **P2 slice 3 (ใหม่)**: ตรวจกับจอยจริง — D-pad ← → Mouse: RS เลื่อน pointer, RB คลิกซ้าย (กดค้างลากได้), RT คลิกขวา, RS click กลับกลางจอ, LS scroll
   - Nav (`↘ Place UI`): LS/RS ขยับ pivot, RS click กลับจุดเริ่มต้น
   - Type: แตะ RB = ล็อก layer 2 (ตัวอักษรเป็น A–H) · กด RB ค้างแล้วพิมพ์ = ชั่วคราว · ค้าง RS ทิศเดิมหลังพิมพ์ = repeat (HUD `×n`)
   - A แตะ 2 ครั้ง = Enter · กด B ค้าง 400 ms = ลบทั้งคำ · กด Y ค้าง = Esc

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
- **Git**: commit ก่อน session นี้ `72ed7dc` (handoff v0.5) · commit งานเป็นชุด `feat(...)` + `docs(...)` (`.nznote/`, `.opencode/` ถูก ignore)
- **เทอร์มินัล cp874** → log ไทยใน console เป็น `???` (ไม่ใช่บั๊ก) · ไฟล์เป็น UTF-8
- **PowerShell กิน `--` ตัวแรก** → `run.ps1` เติมให้แล้ว ใช้ `run.ps1 --seconds 30`
- `Chrome_WidgetWin_0 ... 1412` ตอนปิด = WebView2 noise ไม่มีผล
- **Chrome headless**: `C:\Users\User\AppData\Local\ms-playwright\chromium-1208\chrome-win64\chrome.exe` (`CHROME_PATH`)
- เครื่องนี้ 2 จอ: virtual desktop 3840×1080 เริ่มที่ x = −1920 · จอหลัก 1920×1080 (pivot เริ่มที่กลางจอหลัก)

**ต่อ CDP เข้า HUD/แอปจริง** (ใช้ไล่บั๊ก UI):

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9224"
crates\rnitype-app\run.ps1 --seconds 120
# แล้วเปิด http://127.0.0.1:9224/json/list → ได้ webSocketDebuggerUrl ของ hud/settings
```

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
