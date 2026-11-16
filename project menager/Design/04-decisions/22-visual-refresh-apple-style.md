---
title: ADR-22 — Visual refresh: Apple/macOS-inspired tokens, HUD plate, Settings sidebar
status: accepted (2026-11-16)
updated: 2026-11-16
tags: [decision, ux, ui, hud, settings]
---

# ADR-22 — Visual refresh (Apple/macOS-inspired)

## บริบท

Owner ขอให้ปรึกษา Claude Fable 5.2 (medium) ผ่าน Claude Code CLI เรื่อง UX/UI ที่ดูทันสมัยแบบ
Apple/macOS แล้วนำมาปรับกับ RniType — คำตอบดิบอยู่ที่ [[05a-claude-ux-consult]]
ข้อสรุปที่เลือกใช้อยู่ที่ [[05-visual-language]]

ข้อจำกัดที่ต้องรักษา:
- HUD ต้องอ่านได้บนพื้นหลังใดก็ได้ (เกม/เว็บ/เอกสาร)
- overlay ต้องเบา, click-through, latency < 16 ms
- ฟอนต์ไทยไม่มีหัว (IBM Plex Sans Thai), 100–150% DPI
- ไม่ใช้ macOS-only API / ไม่ลอก traffic lights หรือ SF font

## การตัดสินใจ

| เรื่อง | ทำอะไร | เหตุผล |
| --- | --- | --- |
| Tokens | `ui/shared/tokens.css` เป็นแหล่งเดียว (Settings + HUD + mode + motion) | ความสม่ำเสมอคือสิ่งที่ทำให้ "ดูแพง" |
| Accent | เปลี่ยนจาก teal `#7cc5ff` → Apple blue `#0A84FF` | accent เดียว ใช้เฉพาะสิ่งที่กำลังเลือก |
| Mode | Type = blue · Mouse = orange `#FF9F0A` · Nav = purple `#BF5AF2` · Caret = cyan `#64D2FF` | หนึ่งโหมด = หนึ่งสี หนึ่งรูปทรง หนึ่งป้าย |
| HUD plate | แผ่นทึบ `rgba(18,18,20,.88)` + ขอบสองชั้น (ขาวใน/ดำนอก) | อ่านได้ทุกพื้นหลัง; ถูกที่สุดต่อเฟรม |
| Blur | **เอา `backdrop-filter` ออกทั้งหมดใน HUD** | WebView เบลอได้แค่เนื้อหาในตัวเอง + เปลือง GPU |
| Active wedge | fill accent + stroke 2px + dim glyph อื่น .45 · **0 ms** | หา target เร็ว; ไม่มีปัญหา wedge ทับ |
| Selected char | ขาว 600 + 31.5px (ที่ text scale 1.4) ไม่มี glow/filter | ลด GPU และคง contrast |
| Pill | สูง 28px · radius 999 · dot สีโหมด 8px · badge Shift/Layer · idle 3 วิ → opacity .6 | เป็นส่วนที่ผู้ใช้เห็นนานที่สุด |
| Correlation | crosshair ขาว 1.5px + outline ดำ + จุดกลางสีโหมด | อ่านได้ทั้งพื้นมืด/สว่าง |
| Motion | HUD 80 ms · mode/pill 140–220 ms · commit flash 120 ms · highlight 0 ms | ไม่หน่วงการเลือก |
| Reduced motion | ปิด transition/animation ทั้งหมด | accessibility |
| Settings | sidebar 200px + inset group + row 44px + window 760×640 | ดูเป็นผลิตภัณฑ์ ไม่ใช่เครื่องมือ dev |
| Settings reset | ยังใช้ปุ่มเดิม (Reset HUD to defaults) | Reset แบบ Undo ต้องมี backup ฝั่ง app — ไป P5 |

## สิ่งที่ไม่รับจากคำปรึกษา

| ข้อเสนอ | เหตุผล |
| --- | --- |
| Traffic lights / SF font | ผิดแพลตฟอร์ม/ผิดไลเซนส์ |
| Vibrancy/blur บน HUD | ใช้ไม่ได้จริงใน overlay + เปลือง GPU |
| หน้าต่าง HUD 420×420 | overlay เต็ม virtual desktop + click-through เป็น architecture ที่ล็อกแล้ว |
| Mica/Acrylic | ยังไม่ทำ — ต้องทดสอบ Tauri `windowEffects` บน Win10/11 |
| Keymap วง + 8 ช่องเต็มรูปแบบ | ต้องมี backend keymap editor (P5) — รอบนี้ทำเฉพาะ sidebar + HUD tab |
| Reset แบบ backup + Undo | ต้องเขียน `settings.backup.json` ฝั่ง app — ไป P5 |

## ผลที่ตามมา

- `ui/shared/tokens.css`: เพิ่ม token ชุด Settings/HUD/mode/radius/spacing/elevation/motion + alias เดิม
- `ui/hud/hud.css`: plate, active state, pill, crosshair, mode colors, reduced motion; ลบ blur/glow
- `ui/hud/hud.js`: idle 3 วิ (heartbeat 2 วิ ไม่รีเซ็ต timer — dedupe ด้วย signature)
- `ui/settings/{index.html,settings.css}`: sidebar + inset group + preview ใหม่
- `crates/rnitype-app/src/main.rs`: Settings window 760×640 (min 680×480)

## หลักฐาน

```
node tools/tests/hud-smoke.mjs       → 54/54
node tools/tests/hud-typography.mjs  → 30/30
  group 14.28px · char 31.5/23.8px · pill 14px · contrast 18.12:1
  screenshots: Design/05-mockup/hud-shots/{hud-typing,hud-command,settings-hud}-100dpi.png
cargo test --workspace               → 145 passed
```

## ความเสี่ยง

| ความเสี่ยง | ทางรับมือ |
| --- | --- |
| แผ่นทึบ .88 บังเนื้อหาใต้ HUD มาก | HUD แสดงเฉพาะตอนพิมพ์; idle เหลือ pill; owner ปรับ opacity ใน Settings ได้ |
| Accent เปลี่ยนจาก teal เป็น blue | เป็นการเปลี่ยน brand ที่ owner ขอ; ย้อนได้ด้วย token เดียว |
| Settings 760×640 อาจเล็กสำหรับ 4 แท็บในอนาคต | ขั้นต่ำ 680×480 + content scroll; ขยายได้เมื่อเพิ่มแท็บ |
| Contrast ของ `--text-3` ต่ำ | ใช้เฉพาะ disabled/placeholder ตามที่กำหนด |
