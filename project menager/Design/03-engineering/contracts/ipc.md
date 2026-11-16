# IPC contract — app ↔ HUD ↔ Settings

> P1 freeze · version 1 · 2026-11-15
> v1.1 (2026-11-15) — เพิ่ม `hud:style` (event) + `hud_style` (command) สำหรับ typography · additive ไม่แตะ payload เดิม (ADR-16)
> v1.2 (2026-11-16) — ระบุรูปแถวของ `rescan_languages` / `set_language_enabled` และ `get_status` ที่ P5 slice 2 ใช้ (additive, ADR-25)
> v1.3 (2026-11-16) — เพิ่ม `get_stats` (chars/WPM/action latency/uptime) สำหรับ Usage cards (additive, ADR-26)
> v1.4 (2026-11-16) — เพิ่ม `get_keymap` / `set_keymap_cell` / `reset_keymap` สำหรับ Keymap editor (additive, ADR-27)
> v1.5 (2026-11-16) — `get_stats` เพิ่ม CPU/RAM/corrections (ADR-29) · `get_keymap` ใช้ได้กับ layout ที่สร้างจาก KLID (ADR-28)
> v1.6 (2026-11-16) — เพิ่ม `record_bench` สำหรับแท็บ Practice (ADR-30)
> ไฟล์นี้คือสัญญาที่ P3 (runtime), P4 (HUD), P5 (Settings) ต้องยึด
> เปลี่ยนชื่อ/รูป payload = ต้อง bump `version` และแจ้ง lead

## 1. หลัก

| ข้อ | กฎ |
| --- | --- |
| Transport | Tauri v2 events (`emit`) สำหรับ push · `invoke` commands สำหรับ request/response |
| Naming | event = `domain:event` · command = `snake_case` |
| Rate | `hud:state` สูงสุด **60 Hz** และส่งเฉพาะเมื่อค่าเปลี่ยน (coalesce) |
| Errors | ทุก command คืน `{ ok: true, data }` หรือ `{ ok: false, error: { code, message } }` |
| Version | payload มี `version: 1` เฉพาะ `hud:state` / config; ที่เหลือดูจาก field |
| หน่วย | `hud:state.pivot`/`pointer` = พิกัดในหน้าต่าง overlay · `hud:state.scale` = `ui.hudScale` (app ประทับทุกเฟรม) |
| ห้าม | HUD เรียก command ที่เปลี่ยน state ของแอป (HUD เป็น read-only + click-through) |

## 2. Events (app → UI)

| Event | Payload | ผู้รับ | ความถี่ |
| --- | --- | --- | --- |
| `hud:state` | [`hud-state.schema.json`](hud-state.schema.json) | HUD | ≤ 60 Hz, on change |
| `hud:style` | `{ textScale: number (0.8–2), font: "ibm-plex-sans-thai"\|"noto-sans-thai"\|"system" }` | HUD | on change (typography · ADR-16) |
| `hud:visibility` | `{ visible: bool, reason: "idle"\|"hotkey"\|"pause"\|"exclusion" }` | HUD | on change |
| `settings:config` | [`config.schema.json`](config.schema.json) | Settings | on open + on external change |
| `app:status` | `{ device: string\|null, connected: bool, paused: bool, admin: bool, version: string }` | Settings, tray | on change |
| `app:toast` | `{ text: string, kind: "info"\|"warn"\|"error" }` | Settings | on demand |

## 3. Commands (UI → app)

### Settings

| Command | Args | Returns |
| --- | --- | --- |
| `get_config` | — | `Config` |
| `save_config` | `{ config: Config }` | `{ ok: true, data: Config }` (validate ก่อนเขียน + backup) |
| `reset_config` | `{ section: "all"\|"general"\|"languages"\|"keymap"\|"hud" }` | `Config` ใหม่ |
| `rescan_languages` | — | `LanguageRow[]` (ดู §5) |
| `set_language_enabled` | `{ code: string, enabled: bool }` | `Config` |
| `set_keymap_cell` | `{ lang: string, layer: u8, group: u8, sector: u8, text: string }` | `Config` |
| `export_profile` | — | `{ path, json }` |
| `import_profile` | `{ json }` | `Config` |
| `set_paused` | `{ paused: bool }` | `{ paused: bool }` |
| `get_status` | — | `app:status` payload |
| `get_stats` | — | `{ chars, wpm, actions, stepAvgUs, stepMaxUs, uptimeSecs, cpuPercent, rssBytes, corrections }` |
| `get_keymap` | `{ code }` | `KeymapPayload` (ดู §5) |
| `set_keymap_cell` | `{ code, layer, group, sector, text }` | `KeymapPayload` |
| `reset_keymap` | `{ code, group? }` | `KeymapPayload` |
| `record_bench` | `{ lang, target, typed, errors, errorPercent, wpm, seconds }` | `{ ok, data: record }` (append `bench.jsonl`) |
| `open_settings` | `{ tab?: "general"\|"languages"\|"keymap"\|"hud" }` | `{ ok: true }` |

### HUD (read-only + lifecycle)

| Command | Args | Returns |
| --- | --- | --- |
| `hud_ready` | — | `{ ok: true }` — HUD แจ้งว่าพร้อมรับ state; app จึงเริ่มส่ง |
| `hud_style` | — | `{ textScale, font }` — typography ปัจจุบัน (HUD ดึงตอน boot กันหลุด event) |
| `hud_set_scale` | `{ scale: number }` | `{ ok: true }` |
| `hud_set_pivot` | `{ x: number, y: number }` | `{ ok: true }` — ใช้ตอน Nav ลากด้วยเมาส์ (ถ้ามี) |

## 4. Flow

```
app (250 Hz) ──coalesce 60 Hz──▶ hud:state ──▶ HUD overlay
     ▲                                              │
     │ hud_ready                                    │ (read-only)
     └──────────────────────────────────────────────┘

Settings ──invoke save_config──▶ app ──validate──▶ config.json
     ▲                              │
     └──── settings:config ─────────┘ (hot reload)
                                    │
                                    └──── hud:style ──▶ HUD (typography, ADR-16)
```

## 5. ตัวอย่าง payload

### `hud:state`
ดู [`hud-state.samples.json`](hud-state.samples.json) — 11 สถานะ: idle, group, char, shift, Thai repeat, cmd, lang, mouse, nav, disabled, quickbar, caret

### `app:status`
```json
{
  "device": "DualSense Wireless Controller",
  "connected": true,
  "paused": false,
  "admin": false,
  "version": "0.1.0"
}
```

### error
```json
{ "ok": false, "error": { "code": "CONFIG_INVALID", "message": "keymaps.en.layers[0].cells ต้องมี 64 ช่อง" } }
```

### `rescan_languages` → `LanguageRow`

```json
{
  "code": "th",
  "klid": "0000041E",
  "label": "Thai (Thailand)",
  "kind": "layout",
  "detected": true,
  "builtin": false,
  "supported": true,
  "enabled": true,
  "locked": false
}
```

| field | ความหมาย |
| --- | --- |
| `code` | รหัสภาษาหลัก (`en`, `th`, `ja`, …) |
| `klid` | layout id ของ Windows (`00000409`); ว่างเมื่อเป็น built-in ที่ไม่ได้ติดตั้ง |
| `kind` | `layout` \| `ime` |
| `detected` | มี layout นี้ใน Windows |
| `builtin` | EN/TH มีตาราง 8×8 ในแอป — แสดงเสมอ |
| `supported` | พิมพ์ได้จริงตอนนี้ (EN/TH) |
| `enabled` | เปิดใน Language wheel |
| `locked` | EN = base ปิดไม่ได้ |

### `app:status` — เพิ่ม `get_status` / `set_paused`

`get_status` คืน payload เดียวกับ event `app:status` · `set_paused { paused }` คืน payload นั้น
หลังเปลี่ยน (Settings ใช้แทนการคลิก tray)

### `get_stats` — Usage cards (ADR-26)

```json
{
  "chars": 75,
  "wpm": 15.0,
  "actions": 75,
  "stepAvgUs": 4,
  "stepMaxUs": 846,
  "uptimeSecs": 51,
  "cpuPercent": 3.1,
  "rssBytes": 33554432,
  "corrections": 0
}
```

`wpm` = 5 ตัวอักษร/คำ จาก commits ใน 60 วิ · `step*Us` = เวลา `session.step` (µs) ต่อวินาทีล่าสุด
`cpuPercent` = เฉพาะโปรเซส RniType (ไม่รวม WebView2 child) · `corrections` = จำนวน Backspace (proxy ของ error rate)

### `KeymapPayload` — Keymap editor (ADR-27)

```json
{
  "code": "th",
  "label": "Thai",
  "groups": ["พยัญชนะ", "สระ", "..."],
  "groups2": ["ฬ ฃ ฅ", "..."],
  "layers": [
    { "name": "base", "cells": ["ก", "น", "..."], "defaults": ["ก", "น", "..."] },
    { "name": "layer 2", "cells": ["ฬ", "..."], "defaults": ["ฬ", "..."] }
  ],
  "overrides": { "1-0-0": "z" }
}
```

`cells` = ค่าที่ใช้จริง (override ชนะ) · `defaults` = ตารางใน core · `""` = ช่องว่าง (layer 2)
override เก็บใน `languages[].overrides` key `<layer>-<group>-<sector>` (รูปเก่า `<g>-<s>` = layer 1)
`get_keymap` ใช้ได้กับทุกภาษาที่เปิด: EN/TH = built-in · layout อื่นสร้างจาก KLID (`build_keymap`)

### `record_bench` — Practice (ADR-30)

```json
{
  "lang": "th",
  "target": "การพิมพ์ด้วยจอย...",
  "typed": "การพิมพ์ด้วยจอย...",
  "chars": 34,
  "errors": 0,
  "errorPercent": 0.0,
  "wpm": 9.3,
  "seconds": 44.0,
  "at": 1763...
}
```

เขียนต่อท้าย `%APPDATA%\RniType\bench.jsonl` (JSONL) และ log `[bench]`

## 6. Error codes

| Code | ความหมาย |
| --- | --- |
| `CONFIG_INVALID` | schema ไม่ผ่าน |
| `CONFIG_IO` | อ่าน/เขียนไฟล์ไม่ได้ |
| `LANG_NOT_FOUND` | ไม่มี layout/IME นี้ในเครื่อง |
| `DEVICE_GONE` | จอยหลุดระหว่างทำรายการ |
| `PERMISSION` | ต้องรันเป็น admin |
| `INTERNAL` | อื่น ๆ (log ไว้) |
