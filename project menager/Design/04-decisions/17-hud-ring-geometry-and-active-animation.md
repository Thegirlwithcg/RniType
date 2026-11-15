---
title: ADR-17 — วงเล็กลง ตัวอักษรใหญ่ขึ้น + animation ตอนเลือก + ขีดกำกับสระล่าง
status: accepted (P4 slice 4) — §5 (animation) ถูกยกเลิกใน ADR-19
updated: 2026-11-15
tags: [decision, hud, typography, thai, animation, settings]
---

# ADR-17 — Ring ที่เล็กลงแต่ชัดขึ้น, active animation, และเลิกใช้คำว่า "ขีดล่าง"

> **อัปเดต:** §5 (ขยาย sector ด้วย transform) ถูกยกเลิกใน [[19-revert-sector-scaling-and-shift-labels]]
> เพราะทำให้ wedge ทับกันจนวงเพี้ยน · ส่วน geometry/ขนาด/ป้าย 2 บรรทัด/ขีดสระล่าง ยังใช้อยู่

## บริบท (feedback รอบ 2 จาก owner, 2026-11-15)

1. **"ขีดล่าง"** — คำนี้โผล่ใต้สระ ุ ู อยากได้ **สัญลักษณ์** มากกว่า ถ้าไม่ได้ก็เอาคำออก
2. **Active ควรมี animation** — ช่องที่เลือก (วงล้อ/วงตัวอักษร) ควรขยายให้เห็น
3. **ไม่รู้จะเปิด Settings ตรงไหน** เพื่อเลื่อน text size
4. **ขนาดวงล้อใหญ่ไปนิด** — ย่อลงได้ แต่ยังต้อง **ชัดเจน** เรื่องตัวอักษร

พร้อมกันนั้น การตรวจของ session ก่อนหน้าเจอ **บั๊กในตัววัด**: `getBBox()` ของ `<text>` ไม่ serialize ผ่าน CDP
→ ผล "ไม่ชนกัน" เป็น false positive; วัดใหม่ด้วย ink box แล้วพบว่า **วรรณยุกต์/กลาง ชนกัน 6%** ที่ text scale 1.2
(ตรงกับที่ owner เห็นว่าอ่านยาก)

## การตัดสินใจ

> **owner ยืนยันค่าเริ่มต้น (2026-11-15): Text size 1.4 · Ring size 1.0 (= 340px)** — ล็อกแล้ว ไม่ต้องถามซ้ำ

### 1. ขนาด: วงเล็กลง + ตัวอักษรใหญ่ขึ้น (ไม่ใช่แลกกัน)

| | ก่อน (ADR-16) | หลัง (ADR-17) |
| --- | --- | --- |
| `hudScale` (วง) | 1.2 → 408px | **1.0 → 340px** (ขนาดที่ออกแบบ) |
| `hudTextScale` | 1.2 | **1.4** |
| group label (จริง) | 13.8px | **15.96px** |
| ป้ายยาว (len-m) | 12.6px | **14.28px** |
| char | 20.4–30px | **25.2px** (เลือก = ขยายด้วย transform 1.3) |
| pill | 13.2px | **14.0px** |
| วงเล็กลง | — | **−17%** |

ตัวคูณ 2 ตัวยังแยกกัน (ADR-16) — วงย่อได้โดยตัวอักษรไม่ย่อ

### 2. วงในกว้างขึ้น เพื่อให้ป้ายไทยมีที่

| | ก่อน | หลัง |
| --- | --- | --- |
| วงใน (กลุ่ม) | 31–68 · text r=49.5 | **33–90 · text r=63** |
| chord ที่ text | 37.9 u | **46.7 u (+23%)** |
| วงนอก (char) | 74–158 · text r=116 | **96–158 · text r=127** |
| วงคำสั่ง/ภาษา | 34–96 · text r=66 | **36–112 · text r=76** |

### 3. ป้ายกลุ่มยาว = 2 บรรทัด (ไม่ย่อจนอ่านไม่ออก)

- ป้าย ≥ **7 grapheme cluster** → แบ่ง 2 บรรทัดครึ่งกลาง (`Intl.Segmenter('th')` — ไม่ตัดสระ/วรรณยุกต์ออกจากพยัญชนะ)
  เช่น `วรรณยุกต์` → `วรรณ` / `ยุกต์`
- `len-m` / `len-l` นับตาม **grapheme** ไม่ใช่ code point (พยัญชนะ = 6 clusters ไม่ใช่ 7 → ไม่ถูกย่อเกินจำเป็น)
- ป้าย 5–6 clusters ยังบรรทัดเดียวที่ 14.28px

### 4. "ขีดล่าง" → ขีดจริงเหนือสระล่าง (ทำตาม ADR-12 ที่ค้างอยู่)

- `hud.js` วาด `<line class="char-guide">` เป็น **เส้นประสั้น** เหนือ ุ ู ฺ (สว่างขึ้นเมื่อถูกเลือก)
- ลบ `"badge": "ขีดล่าง"` ออกจาก `hud-state.samples.json` + `assets/hud-demo-states.json`
- HUD ตัดสินเองด้วยชุดอักขระ 3 ตัว (เหมือนม็อกอัป v0.11 `isBelowMark`) — ไม่เพิ่ม field ใน contract

### 5. Active = ขยาย (animation)

| องค์ประกอบ | effect | origin |
| --- | --- | --- |
| sector (group/char/cmd/lang) | `scale(1.05–1.07)` | **ศูนย์กลางวง** (`transform-box: view-box`) → ปูดออกจากกลาง |
| label ที่เลือก | `scale(1.1–1.3)` | **กลางของตัวเอง** (`transform-box: fill-box`) |
| เปลี่ยนจาก `font-size` → `transform` | เพื่อให้ motion เนียนและไม่ดัน layout | `transition: transform 0.2s var(--ease-out)` |

`prefers-reduced-motion` ยังใช้ได้ (transition ถูกตัดให้เกือบทันที)

### 6. เปิด Settings ได้จาก 2 ทาง

- **คลิกซ้ายที่ไอคอน tray** (ใหม่) หรือคลิกขวา → Settings… (ไอคอนอาจอยู่ในถาดซ่อน `^`)
- `run.ps1 --demo --settings` เปิดหน้าต่างมาพร้อมกัน
- เพิ่ม **Ring size slider** (0.8–1.6) ใน Settings → HUD เพื่อปรับวงได้เองโดยไม่ต้องรอ build
- หน้าต่าง Settings ยกตัวเองขึ้นมา (`focused` + `set_focus`) หลังสร้าง

### 7. `ui.hudScale` ถูกประทับลง `HudState.scale` โดยแอป

- source of truth เดียว: config → app stamp ทุกเฟรม → HUD อ่าน `state.scale`
- `save_config` ส่ง state ปัจจุบันซ้ำทันที เพื่อให้ slider เห็นผลทันที (ไม่ต้องรอ tick ถัดไป)
- ลำดับ lock ที่ใช้ทั้งไฟล์: **config → demo → origin** (กัน deadlock กับ thread ของ demo)

## ผลที่ตามมา

- `ui/hud/hud.js` — geometry ใหม่, `graphemes()`/`renderLines()`, `char-guide`, class `is-armed` บน label
- `ui/hud/hud.css` — `--fs-*` ใหม่, transform/transition, `.char-guide`
- `ui/settings/*` — Ring size slider, ค่าเริ่มต้น 1.4/1.0, preview ใช้ base เดียวกับ HUD
- `crates/rnitype-app` — `hud_scale()` + stamp, re-emit หลัง save, tray left click, window focus, ขนาดหน้าต่าง 560×880
- `tools/tests/hud-typography.mjs` — **ink box** แทน `getBBox()` (แก้ false pass), เพิ่ม 5 checks (guide, 2 บรรทัด, animation ×3, sub ของวงคำสั่ง)
- ตัวเลขใน ADR-16 (ขนาด/ค่าเริ่มต้น) ถูกแทนที่ด้วย ADR-17 — โครงสร้างการตัดสินใจ (2 ตัวคูณ, `hud:style`, ฟอนต์) ยังใช้อยู่

## หลักฐาน (2026-11-15)

| หลักฐาน | ผล |
| --- | --- |
| `cargo test --workspace` | **91 passed** (app 7 + core 50 + win 34) |
| `node tools/contracts/validate.mjs` | 10/10 |
| `node tools/tests/hud-smoke.mjs` | 54/54 |
| `node tools/tests/hud-typography.mjs` | **29/29** (ไม่มี label ชนกันทั้งวงกลุ่ม/ตัวอักษร/วงคำสั่ง) |
| E2E กับแอปจริง | save 1.5/1.3 → HUD `--hud-text-scale 1.50`, stage 442px, config.json ตรง; reset → 340px |
| screenshot | `05-mockup/hud-shots/` — typing/command 100–150% DPI, Settings window จริง, overlay จริง |

## ความเสี่ยง

| ความเสี่ยง | ระดับ | ทางรับมือ |
| --- | --- | --- |
| ป้าย 2 บรรทัดดูสูงกว่าช่องอื่น | ต่ำ | ใช้เฉพาะป้าย ≥ 7 clusters; ขนาด 14.28px เท่า len-m |
| แบ่งคำไทยกลางคำ (วรรณ/ยุกต์) อ่านสะดุด | ต่ำ | ตัดที่ grapheme boundary; ถ้ารับไม่ได้ให้แก้ label ที่ต้นทาง (keymap) ให้สั้นลง |
| animation ทำให้อ่านไม่ทันตอนกดเร็ว | ต่ำ | 0.2s และขยายไม่เกิน 1.3; ปิดได้ด้วย reduced-motion |
| ค่า default ใหม่ทับ config เดิมของผู้ใช้ | ต่ำ | ค่าเดิมใน config ยังชนะ (default ใช้เมื่อไม่มีค่า) |
