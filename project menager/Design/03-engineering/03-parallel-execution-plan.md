---
title: แผนทำงานหลังม็อกอัป — M0–M5 และ P0–P6
status: active v0.8 — P0/P1 ปิด · P2/P3/P4 slice 1 · P4 overlay merged
updated: 2026-11-15
tags: [plan, execution, parallel, agents]
---

# 03.3 — แผนทำงานหลังม็อกอัป v0.11

> ทบทวนจาก [[02-roadmap-and-smoke-tests]] โดยยึดสถานะโค้ดจริง ณ 2026-11-15
> เป้าหมาย: ลดเวลาปฏิทินจาก ~7 สัปดาห์ (ทำเรียง) เหลือ ~4–5 สัปดาห์ ด้วยงานคู่ขนาน 4 track
>
> **v0.2:** ตัด big-pickle ออก · เพิ่มข้อตกลง M ↔ P ให้เหลือแหล่งความจริงเดียว
> **v0.3 (2026-11-15):** เลือกแผน P เป็นแผนลงมือ · **P1 ปิดแล้ว** (contracts + workspace + state player)

## 0. สรุป 1 นาที

| คำถาม | คำตอบ |
| --- | --- |
| กี่เฟส | **7 เฟส (P0–P6)** |
| M กับ P ต่างกันไหม | **ไม่ใช่อันเดียวกัน** — M = ปลายทาง/เกณฑ์ผ่าน · P = ลำดับ/การขนาน (ดู §0.5) |
| เฟสที่ต้องทำเรียง | **P0 (spike gate) → P1 (contract freeze)** เท่านั้น |
| เฟสที่ขนานได้ | **P2 · P3 · P4 · P5 = 4 track พร้อมกัน** หลัง P1 ปิด |
| เฟสปิดท้าย | **P6** รวมวัดผล + ไทย + แพ็กเกจ |
| critical path | `P0 → P1 → P3 (Windows e2e) → P6` |
| งานที่เริ่มได้เร็วที่สุดหลัง P1 | **HUD overlay (P4)** ใช้ state ปลอม ไม่ต้องรอแอปจริง |
| จำนวน agent แนะนำ | **4 coding session + 1 lead** (lead ไม่ถืองาน implement พร้อมกัน) |

**หลักเดียวที่ทำให้ขนานได้จริง:** P1 ต้อง freeze สัญญา 4 อย่างก่อน —
`Action` enum · `hud-state` schema · `config` schema · IPC/event list
ถ้าไม่ freeze = 4 คนแก้ interface กันไปมา แพงกว่าเขียนเรียง.

---

## 0.5 M กับ P ต่างกันไหม? — ต่างกัน แต่เป็นแผนเดียวกันคนละมุม

**ไม่ใช่งานคนละชุด** — P คือ M ที่ถูก "ผ่าใหม่" เพื่อจัดลำดับและทำขนาน

| แกน | **M = Milestone** | **P = Execution phase** |
| --- | --- | --- |
| ใช้ตอบคำถาม | "เสร็จแล้วแปลว่าอะไร" | "ทำเมื่อไหร่ ใครทำ ขนานกับอะไร" |
| เนื้อหา | ขอบเขต + เกณฑ์ผ่าน + ตัวเลข (WPM, latency) | ลำดับงาน + gate + เจ้าของไฟล์ |
| ชุดตัวเลข | M0–M5 (+ release) | P0–P6 |
| เปลี่ยนได้ไหม | **เกณฑ์ผ่านต้องนิ่ง** เปลี่ยนได้เฉพาะผ่าน ADR | ปรับลำดับได้ถ้า dependency เปลี่ยน |
| ใช้ที่ไหน | [[HANDOFF]], [[README]], canvas, ตารางวัดผล | เอกสารนี้ + brief ของ subagent |

**ข้อตกลงเพื่อกันสองแหล่งความจริง**

1. **นับ "เสร็จ" ที่ M เท่านั้น** — P ไม่มีเกณฑ์ผ่านของตัวเอง ใช้เกณฑ์ของ M ที่มันไปถึง
2. **P ใช้บอกลำดับ/การขนาน** — ถามว่า "เริ่มอะไรก่อน" ให้ดู P; ถามว่า "เสร็จหรือยัง" ให้ดู M
3. ถ้าเอกสารขัดกัน → **scope/เกณฑ์ผ่านใช้ M · ลำดับ/เจ้าของงานใช้ P**
4. จำนวน P ไม่เท่า M และไม่ต้องเท่า: P1 (contracts) ไม่มีใน M เดิม, P3 รวม M1+M2 บางส่วน, P4 รับทั้ง M2.5+M3

**สรุปสั้น:** M = "ปลายทาง" · P = "เส้นทางและคิว"; งานจริงมีชุดเดียว

---

## 1. สถานะจริง (ไม่ใช่แผน)

| ของ | หลักฐาน | สถานะ |
| --- | --- | --- |
| เอกสารออกแบบ + ADR 14 ฉบับ | `Design/` | ✅ |
| ม็อกอัป HUD v0.11 | `node tools/tests/hud-smoke.mjs` → **54/54** | ✅ |
| core + win crate | `cargo test --workspace` → **145 passed** (core 84 + win 36 + app 25) | ✅ P2/P3 |
| contracts (HUD/config/IPC) | `node tools/contracts/validate.mjs` → **10/10** | ✅ P1 |
| workspace split | `crates/rnitype-{core,win,app,probe}` + `ui/` | ✅ P1 |
| state player | `node tools/state-player/serve.mjs` → HUD ไม่ต้องมีแอป | ✅ P1 |
| probe จอย (Spike A) | XInput: แกน ±1.0 · 8 sectors · ปุ่ม 16/16 | ✅ ผ่าน 2026-11-15 · DualSense เลื่อน |
| Spike B (Tauri overlay) | ใส ✅ · ย้ายจอ ✅ · click-through ✅ · focus ✅ (`match=true` 4/4) | ✅ ผ่าน 2026-11-15 |
| Spike C (SendInput ไทย) | พิมพ์ `กขคง Hello ไทย 123` ลง Notepad ถูก | ✅ ผ่าน 2026-11-15 |
| injector (SendInput) | P3 slice 1: Unicode/VK/mouse + HWheel + partial-send guard | ✅ 36 tests |
| app shell / tray / overlay | P4 slice 2: full virtual desktop + click-through + tray | ✅ |
| HUD overlay จริง | Tauri window โหลด `ui/hud` + `hud:state` demo | ✅ |
| Settings window + config | P5 slice 1: หน้าต่าง + แท็บ HUD + persist | ✅ |
| ตรวจภาษาที่ติดตั้ง / QWERTY map | — | ❌ |
| วงคำสั่ง / วงภาษา / repeat / layer 2 / mouse / Nav | core + runtime + HUD (P2 slice 1–3) | ✅ |
| แพ็กเกจ / installer / autostart | — | ❌ |

**สรุปความคืบหน้าตาม M:** M0 ~70% · M1 ~35% · M2 0% · M2.5 0% (มีแต่ mockup) · M3 0% · M4 0% · M5 0%

> ข้อควรระวัง: HANDOFF เขียนว่า "M0 พร้อม" — จริงเฉพาะโครงสร้าง/โค้ด ยังไม่ผ่านเกณฑ์ฮาร์ดแวร์
> และ `rnitype-core` ยังไม่มี พฤติกรรมวงคำสั่ง/วงภาษา/repeat/layer 2 ตาม [[04-state-machine-spec]]

---

## 2. หลักการแบ่งเฟส (5 ข้อ)

1. **Gate ก่อนขนาน** — P0 ต้องมีตัวเลขจริงก่อนเขียน app; ถ้าไม่ผ่านให้ถอย C# ทันที ไม่ใช่ฝืน
2. **Contract first** — P1 เขียน schema + enum + fake implementation ให้ครบ; ห้ามขนานก่อนปิด
3. **1 crate = 1 owner** — กัน merge conflict และกัน agent แก้ไฟล์เดียวกัน
4. **Fake ที่ขอบ** — fake pad / fake injector / fake clock / state player ทำให้ UI และ core ทำงานได้โดยไม่ต้องมีฮาร์ดแวร์
5. **ทุก loop มีหลักฐาน** — คำสั่งที่รัน + ผลจริง; คำว่า "น่าจะได้" ไม่นับ

---

## 3. เฟสทั้ง 7

### P0 — Spike gates · 2–3 วัน · **ทำเรียง, ห้ามข้าม**

| งาน | ผลลัพธ์ | เกณฑ์ผ่าน |
| --- | --- | --- |
| A. `gilrs` กับ Xbox (DualSense เลื่อน) | log แกน/ปุ่ม/deadzone/drift | Xbox: เห็นครบ 8 ปุ่ม + trigger + แกน 4 ทิศ · **DualSense: ก่อน release** |
| B. Tauri overlay โปร่งใส | หน้าต่างทดสอบ 2 จอ 150% | คลิกทะลุ, ไม่แย่ง focus, ไม่กระพริบ |
| C. `SendInput` ภาษาไทย | พิมพ์ "กขคง" ลง Notepad | ตัวอักษรถูกทุกตัว (Unicode path) |

**ทางถอย:** A ไม่ผ่าน → `sdl3` · B ไม่ผ่าน → egui · C ไม่ผ่าน → ตรวจ scancode/IME ก่อน · พังทั้งคู่ → C#
**เจ้าของ:** owner รันบนเครื่องจริง + agent เขียน spike code (คนละส่วนกับ P2–P5)

---

### P1 — Contract freeze + แยก workspace · 2 วัน · **ทำเรียง, ทำคนเดียว**

| งาน | รายละเอียด |
| --- | --- |
| แยก crate | `rnitype-core` (pure) · `rnitype-win` (injector/detection) · `rnitype-app` (runtime/tray) · `ui/hud` · `ui/settings` |
| `Action` enum | เพิ่ม `MouseMove/Click/Wheel`, `WheelCommand`, `SetMode`, `SetLayer`, `KeyDown/Up`, `MovePivot` |
| `hud-state` schema | snapshot ที่ HUD ใช้: mode, layer, shift, group, sector, preview, pill, pivot, lang, badges |
| `config` schema | ตาม [[01-architecture]] §4 + `languages[]` + `overrides` |
| IPC list | Tauri event/command ที่ app ↔ HUD ↔ Settings ใช้ (ชื่อ, payload, ความถี่) |
| Fake | `FakeInjector`, `FakePad`, `StatePlayer` (เล่น `hud-state.sample.json`) |
| เอกสาร | `docs/contracts/*.json` + `ipc.md` + 1 ตัวอย่าง state ต่อโหมด |

**เกณฑ์ผ่าน:** `cargo test --workspace` เขียว · schema validate ตัวอย่างได้ · เปิด `ui/hud` ด้วย state player ได้โดยไม่ต้องมีแอป
**เจ้าของ:** lead (deepseek) — ห้ามแตกให้ subagent เพราะ contract แตกแล้วขนานทั้งแผนพัง

---

### P2 — Core behaviors (Track A) · 5–7 วัน · ขนานได้

> **slice 1 ✅ `ce7d027`** — วงคำสั่ง · วงภาษา · mode arbiter · emergency
> **slice 2 ✅ `2084e26`** — Shift toggle (LT) · ปุ่มหน้า A/B/X/Y แตะ · B ยกเลิก gesture (50 core tests)
> **slice 3 ✅ 2026-11-16** — mouse math/accel · Nav pivot · auto-repeat · layer 2 · double-tap/hold
> เหลือ: quickbar (LB) · caret nav (R3) · sticky drag · pivot clamp/DPI

| งาน | เกณฑ์ผ่าน |
| --- | --- |
| วงคำสั่ง (D-pad ←, เลือกด้วย RS, ปล่อย = ยืนยัน, arm/lock) | ครบทุกเคสใน [[04-state-machine-spec]] §6 |
| วงภาษา (1 ภาษา = สลับ, หลายภาษา = wheel) | ครบ + ไม่มีช่องเมื่อมีภาษาเดียว |
| Mode arbiter (Type/Mouse/Nav) + emergency + exclusion | สลับกลาง gesture = ยกเลิก ไม่มีคีย์ค้าง |
| repeat / layer 2 / Shift toggle / quickbar / caret nav | ตรงค่าเวลาใน spec |
| Mouse math (vmax/gamma/deadzone/recenter) + Nav pivot | unit test ค่าคงที่ + property test hysteresis |
| ทดสอบเชิงรุก | 21 เคสใน spec §6 ผ่าน 100% + property test 45° |

**ไม่แตะ:** Windows API, UI, `Cargo.toml` ราก
**ขนานกับ:** P3, P4, P5

---

### P3 — Windows runtime + injector (Track B) · 5–7 วัน · ขนานได้

> **slice 1 ✅ `a90b909`** — SendInput Unicode/VK, mouse rel/abs/buttons/wheel, modifier guard, partial-send tracking, Drop safety net
> เหลือ: scancode/VK path สำหรับ IME, exclusion/UIPI/admin, app runtime wiring

| งาน | เกณฑ์ผ่าน |
| --- | --- |
| `rnitype-win`: SendInput scancode + Unicode fallback | พิมพ์ EN/TH ลง Notepad ถูก 100% |
| Mouse: relative + absolute (recenter/ข้ามจอ) | ข้ามจอ 1920 px < 1.5 s |
| Modifier RAII guard + จอยหลุด | ไม่มี Alt/Ctrl/Shift ค้างหลัง 1 ชม. |
| `rnitype-app`: loop 250 Hz, hotplug, tray, single instance | CPU idle < 0.5%, จอยหลุด/ต่อกลับได้ |
| Config load/save/hot reload + foreground/exclusion | เปลี่ยน config แล้วมีผลทันที |
| **Walking skeleton**: จอย → พิมพ์ Notepad | พิมพ์ 20 คำถูก, วัด WPM ได้ |

**ขนานกับ:** P2 (core), P4 (HUD ใช้ state ปลอม), P5a (ตรวจภาษา)
**เจ้าของ:** deepseek (injector/unsafe) + MiniMax (Tauri shell/tray)

---

### P4 — HUD overlay (Track C) · 4–5 วัน · ขนานได้

> **slice 1 ✅ `bd98777`** — state-driven renderer ครบทุก mode (type/cmd/lang/mouse/nav/disabled)
> **slice 2 ✅ `2354912`** — Tauri overlay เต็ม virtual desktop, click-through/no-focus, tray Quit, `hud:state` demo
> **slice 3 (next, owner feedback 2026-11-15)** — typography/readability: ตัวหนังสือใหญ่ขึ้น + ฟอนต์ไทยไม่มีหัว (IBM Plex Sans Thai) + text scale
> **visual refresh ✅ 2026-11-16 (ADR-22)** — tokens เดียว · HUD plate `.88` (no blur) · active 0 ms · pill 28px · Settings sidebar 760×640
> เหลือ: pivot per-monitor/mixed-DPI, 60 Hz coalescing + latency, real runtime wiring

| งาน | เกณฑ์ผ่าน |
| --- | --- |
| พอร์ต `hud-prototype` → Tauri overlay | หน้าตา parity กับ v0.11 |
| click-through, always-on-top, ไม่แย่ง focus | ตรงตาม Spike B |
| 2 จอ + DPI 150%/200%, จำ pivot | ย้ายจอแล้วไม่เพี้ยน |
| วงใน/วงนอก/วงคำสั่ง/วงภาษา/pill/idle/ไทยมีขีด | ครบทุก state ใน schema |
| latency + repaint | HUD ขึ้น < 16 ms, วาด < 8 ms |
| **Typography/readability (owner feedback)** | group 14–15px · sub 11px · pill 13px · preview 32px · ฟอนต์ไทย loopless · อ่านชัดที่ 100–150% DPI |

**เริ่มได้ทันทีหลัง P1** ด้วย `StatePlayer` — ไม่ต้องรอ P3
**เสี่ยงต่ำ:** ถ้า P3 ช้า HUD ยังเสร็จและเดโมได้

---

### P5 — Settings + languages/config (Track D) · 5–6 วัน · ขนานได้

| งาน | เกณฑ์ผ่าน |
| --- | --- |
| P5a ตรวจ layout: `GetKeyboardLayoutList` + `ToUnicodeEx` | เห็น EN/TH/RU/EL ตามเครื่องจริง |
| P5a สร้าง 8×8 จาก QWERTY + cache + override | RU/EL พิมพ์ได้, แก้ช่องได้ |
| P5b Settings window (General/Languages/Keymap/HUD) | ตั้งค่าแล้วมีผลทันที |
| **P5b Settings → HUD: text size + font family** | slider 0.8–2.0 + loopless Thai font (IBM Plex Sans Thai / Noto Sans Thai / System) + preview |
| Keymap editor + controller map + Reset all | reset ได้ทั้ง section/ทั้งหมด |
| Profiles ต่อ exe + autostart | reload โดยไม่ต้องรีสตาร์ต |

**P5a ขนานกับ P5b ได้** (คนละภาษา/คนละโฟลเดอร์)
**ขนานกับ:** P2, P3, P4

---

### P6 — วัดผล + ไทย + แพ็กเกจ · 6–8 วัน · เริ่มเมื่อ P3/P4/P5 merge แล้ว

| งาน | เกณฑ์ผ่าน |
| --- | --- |
| ชุดทดสอบ 500 ประโยค EN + 50 ประโยค TH | ตาราง WPM/error/latency มีตัวเลขจริง |
| `tools/smoke/*` ครบ 5 ตัว + `run-all.ps1` | รันก่อน commit ได้ |
| ทดสอบกับคน 3–5 คน | เปิดวงคำสั่ง < 2 s, เปลี่ยนกลุ่ม < 1.2 s |
| Performance budget + soak 1 ชม. | ตาม [[01-architecture]] §6 |
| ไทย: วรรณยุกต์/สระล่าง/สระนำ + IME passthrough | 12 WPM ไทย, error < 1.5% |
| Installer + autostart (Task Scheduler) + admin option | ติดตั้งบน Win11 สะอาดได้ |
| แพ็กเกจ Steam/itch + privacy + logs + onboarding | zip/instaler รันได้, ไม่มีข้อมูลส่วนตัว |

---

## 4. Dependency graph

```
P0 Spike (gate)
      │  ผ่าน A/B/C
      ▼
P1 Contracts + workspace  ←── owner เคาะ Q1/Q2/Q5
      │
      ├──────────┬──────────┬──────────┐
      ▼          ▼          ▼          ▼
   P2 Core    P3 Windows  P4 HUD    P5 Settings
   (Track A)  (Track B)   (Track C)  (Track D)
      │          │          │          │
      └──────────┴────┬─────┴──────────┘
                      ▼
              P6 วัดผล + ไทย + แพ็กเกจ
```

**เวลาโดยประมาณ**

| แบบ | เวลาปฏิทิน |
| --- | --- |
| ทำเรียง (1 agent) | ~38 person-days ≈ 7–8 สัปดาห์ |
| ขนาน 4 track | ~20 working days ≈ **4 สัปดาห์** + buffer 1 สัปดาห์ |

---

## 5. Track และเจ้าของไฟล์ (กัน conflict)

| Track | โฟลเดอร์ที่ถือ | เฟส | โมเดลหลัก |
| --- | --- | --- | --- |
| A — Core | `crates/rnitype-core/` | P2 | deepseek-v4.1-flash |
| B — Windows | `crates/rnitype-win/`, `crates/rnitype-app/` | P3 | deepseek (injector) + MiniMax (shell) |
| C — HUD | `ui/hud/` | P4 | MiniMax M3.1 Flash (session 1) |
| D — Settings | `ui/settings/` | P5b | MiniMax M3.1 Flash (session 2) |
| D — Language | `crates/rnitype-win/lang/` | P5a | deepseek |
| Lead | `Cargo.toml` ราก, `docs/contracts/`, merge, review | P1, P6 | deepseek-v4.1-flash |

**กฎเหล็ก**
- `Cargo.toml` ราก + `docs/contracts/` → **lead เท่านั้น**
- 1 โฟลเดอร์มี owner เดียว ณ เวลาหนึ่ง; ถ้าต้องแก้ข้ามโฟลเดอร์ ให้เปิด PR เล็กแล้วให้ lead merge
- UI ทั้งสองหน้าต่างใช้ design token ร่วม (`ui/shared/tokens.css`) ที่ lead สร้างใน P1
- ห้าม subagent commit ลง `main`; ใช้ branch/worktree ต่อ track

---

## 6. Sub Agent — ใครทำอะไร

> ใช้ 2 โมเดลเท่านั้น: `opencode-go/deepseek-v4.1-flash` · `opencode-go/minimax-m3.1-flash`
> **ไม่ใช้ `opencode/big-pickle`** — เป็น stealth model ฟรีที่ข้อมูลอาจถูกนำไปปรับปรุงโมเดล
> ไม่เหมาะกับโค้ดสินค้าที่จะขายบน Steam/itch

| งาน | Sub agent (model) | เหตุผล | ตรวจโดย |
| --- | --- | --- | --- |
| P0 เขียน spike A/B/C | **deepseek-v4.1-flash** | งาน Win32/Tauri ต้องแม่น API; owner เป็นคนรัน | owner |
| P1 contracts + workspace | **lead (deepseek)** — ไม่ใช้ subagent | contract ต้องมีสมองเดียว | owner |
| P2 core behaviors | **deepseek-v4.1-flash** (session A) | logic/เวลา/hysteresis ต้องแม่น, test เยอะ | deepseek session ใหม่ (spec-only) |
| P2 เขียน test จาก spec (§6 21 เคส) | **MiniMax M3.1 Flash** (session ใหม่) | งาน batch อ่าน spec แล้วเขียน test; ให้มุมมองต่างจากคน implement | deepseek |
| P3 injector + SendInput + mouse | **deepseek-v4.1-flash** (session B) | `unsafe`, Win32, modifier guard — พลาดแล้วคีย์ค้าง | lead + owner รันจริง |
| P3 Tauri shell/tray/single-instance | **MiniMax M3.1 Flash** | Tauri/JS scaffolding เร็ว, งานเชื่อม UI–Rust | deepseek |
| P4 HUD overlay (พอร์ต mockup) | **MiniMax M3.1 Flash** (session C) | HTML/CSS/JS ตรงสกิล, มี mockup เป็น source of truth | deepseek (parity + perf) + owner ดูภาพ |
| P5a ตรวจภาษา + QWERTY map | **deepseek-v4.1-flash** | Win32 `ToUnicodeEx`, cache/override ต้องเป๊ะ | lead |
| P5b Settings window | **MiniMax M3.1 Flash** (session D) | งาน form/table CRUD; แยก session จาก HUD เพื่อไม่ชน CSS | deepseek (schema) + MiniMax session C (visual) |
| P6 smoke scripts + รายงาน | **MiniMax M3.1 Flash** | PowerShell/Node/รายงานตาราง — งานเร็ว ไม่ต้องคิดลึก | deepseek ตรวจตัวเลข |
| P6 independent QA (edge case) | **deepseek-v4.1-flash** (session ใหม่) | อ่าน spec + diff แล้วไล่เคสตกหล่น; ห้ามเห็น rationale ของคนเขียน | lead |
| P6 installer/signing/release | **deepseek-v4.1-flash** | งาน risk สูง, ห้ามพลาดเรื่องสิทธิ์/คีย์ | owner |
| เอกสาร/onboarding | **MiniMax M3.1 Flash** | เขียนไทย/อังกฤษเร็ว | lead ตรวจข้อเท็จจริง |

### นโยบายโมเดล (v0.2)

- **2 โมเดลที่ใช้:** deepseek-v4.1-flash (core/Win32/review) + MiniMax M3.1 Flash (UI/scripts/docs)
- **ขนานหลาย session ของโมเดลเดียวกันได้** แต่ต้องแยก worktree + context และห้าม session reviewer เห็นเหตุผลการตัดสินใจของคนเขียน
- ** reviewer ต้องเป็นคนละ session เสมอ**; ถ้าเป็นโมเดลเดียวกัน ให้ใช้ "spec-only prompt" (ให้เฉพาะ spec + diff + test output)
- **ห้ามใส่ secret ใน prompt/repo:** signing key, token ร้านค้า, ข้อมูลผู้ใช้จริง — ใช้ environment variable ตอน release เท่านั้น

### รูปแบบการเรียก subagent (แนะนำ)

| ช่วง | เรียกพร้อมกัน | หมายเหตุ |
| --- | --- | --- |
| หลัง P1 ปิด | 4 sessions: core (deepseek) / win (deepseek) / hud (MiniMax) / settings (MiniMax) | 1 worktree ต่อ session |
| ระหว่าง P2 | + MiniMax เขียน test ใน worktree แยก (spec-only) | test ต้อง fail ก่อน implement (TDD) |
| ก่อน merge P3 | + deepseek session ใหม่ QA อ่าน diff | ไม่แก้โค้ด แค่รายงาน |
| P6 | 2 sessions: harness (MiniMax) / QA (deepseek ใหม่) + lead วิเคราะห์ตัวเลข | ห้ามขนานแก้ไฟล์เดียวกัน |

---

## 7. Review protocol (กันงานขนานพัง)

1. **implementer ≠ reviewer** ทุกงาน — ต้องคนละ session; ถ้าเป็นโมเดลเดียวกันให้ใช้ spec-only prompt
2. reviewer ได้: brief + diff + spec section + คำสั่งที่รันและผลลัพธ์
3. ห้าม approve ด้วยคำว่า "ดูดี" — ต้องมี output ของ `cargo test`, `hud-smoke`, smoke script
4. ทุก track merge เข้า `main` อย่างน้อยวันละครั้ง (lead เป็นคน merge) เพื่อลด integration debt
5. ตัวเลข WPM/latency/error ต้องอัปเดตใน [[02-roadmap-and-smoke-tests]] §4 ทุก loop

---

## 8. ความเสี่ยงของงานคู่ขนาน + ทางรับมือ

| ความเสี่ยง | ผลกระทบ | ทางรับมือ |
| --- | --- | --- |
| Contract drift (HUD กับ core ไม่ตรง) | สูง | P1 freeze + schema test + ตัวอย่าง state ต่อโหมด |
| Merge conflict | กลาง | 1 crate = 1 owner, PR เล็ก, rebase ทุกวัน |
| UI สองหน้าต่างไม่เหมือนกัน | กลาง | `tokens.css` ร่วม + mockup เป็น source of truth |
| รอ owner ทดสอบจอย | สูง | ใช้ fake pad/state player; รวม hardware test วันละ 2 รอบ |
| ใช้โมเดลเดิมหลาย session → จุดบอดซ้ำกัน | กลาง | สลับ reviewer เป็นอีกโมเดลเมื่อทำได้ + spec-only prompt + test ที่เขียนก่อน implement |
| Integration hell ตอนท้าย | สูง | walking skeleton จบ P3 + merge รายวัน |
| Agent แก้เกินขอบเขต | กลาง | ระบุไฟล์ที่ถือใน brief + ห้ามแตะ `Cargo.toml` ราก |
| path มี `'` (Erin's Works) → Tauri RC2135 | สูง | dev ใช้ `spikes/overlay/run.ps1` (icon ไป `%TEMP%` + `TAURI_CONFIG`); release build ใน CI path สะอาด |

---

## 9. 5 อย่างที่ต้องทำก่อนเปิด subagent

1. ✅ **Commit baseline** — commit `00a6668` (`.nznote/`, `.opencode/`, `target/` ถูก ignore)
2. ✅ **P0 ผ่าน 3/3** — A (XInput 16 ปุ่ม) · B (click-through) · C (SendInput ไทย); DualSense เลื่อนก่อน release
3. ✅ **P1 contracts** — `project menager/Design/03-engineering/contracts/` + `tools/contracts/validate.mjs`
4. ✅ **แยก workspace** — core / win / app / probe + `ui/hud` / `ui/shared` + `NullInjector`
5. ⏳ **แตก brief 4 track** พร้อมไฟล์ที่ถือ + เกณฑ์ผ่าน + คำสั่งตรวจ แล้วค่อย spawn

**คำสั่งตรวจ P1 (รันซ้ำได้)**

```powershell
cargo test --workspace
node tools/contracts/validate.mjs
node tools/state-player/serve.mjs   # เปิด http://127.0.0.1:4173/ui/hud/
```

---

## 10. ตารางแปลง M ↔ P

> ใช้คู่กับ §0.5: **M = ปลายทาง/เกณฑ์ผ่าน · P = ลำดับ/การขนาน**

| M (milestone) | ขอบเขต | P ที่ไปถึง | หมายเหตุ |
| --- | --- | --- | --- |
| **M0 Spike** | อ่านจอย + ตรวจ overlay + พิมพ์ไทยได้ | **P0** + **P1** | P1 เพิ่ม contracts/workspace ที่ M0 เดิมไม่มี |
| **M1 Typing core** | normalizer + state machine + keymap + injector | **P2** + **P3** | P2 = logic, P3 = ส่งคีย์จริง |
| **M2 Mouse** | pointer + accel + click + recenter | **P2** + **P3** + **P4** | math อยู่ P2 · injection P3 · cursor P4 |
| **M2.5 HUD polish** | variant B, วงคำสั่ง, เปลี่ยนกลุ่ม | **P4** + **P6** | P6 = ทดสอบคนจริง |
| **M3 HUD** | overlay 2 วง + pill + latency | **P4** | เริ่มหลัง P1 ไม่ต้องรอ P3 |
| **M4 Config UI** | tray + settings + profile + autostart | **P5** | P5a ภาษา + P5b หน้าตั้งค่า |
| **M5 Thai + polish** | keymap ไทย + layer 2 + วัดผล | **P6** + **P5a** | ไทย/หลายภาษามีใน P4/P5a ด้วย |
| **Release** (ใหม่) | installer + Steam/itch + onboarding | **P6** | ไม่มีใน roadmap เดิม |

**วิธีอ่านให้ไม่สับสน**

- ถามว่า **"เสร็จหรือยัง"** → ดู **M** และเกณฑ์ใน [[02-roadmap-and-smoke-tests]]
- ถามว่า **"เริ่มอะไรก่อน / ใครทำ / ขนานกับอะไร"** → ดู **P** ในเอกสารนี้
- รายงานความคืบหน้าเป็น **% ของ M** (เช่น M1 35%) ไม่ใช่ % ของ P
- P1 ไม่มี M รองรับ (เป็นงาน infrastrucure ที่เพิ่มเข้ามา) — ให้ถือเป็น "ประตูของ M0"
