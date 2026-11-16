---
title: ADR-28 — Universal QWERTY mapping (P5 slice 4)
status: accepted
updated: 2026-11-16
tags: [decision, i18n, keymap, win]
---

# ADR-28 — Universal QWERTY mapping สำหรับ layout ที่ติดตั้งใน Windows

## บริบท

P5 slice 3 เปิดแท็บ Keymap แล้ว แต่แก้ได้เฉพาะ EN/TH เพราะ core ยัง lookup ตาราง static
(`keymap::lookup_layer`) · ภาษา RU/EL ขึ้น `Needs mapping` และ toggle ปิด
ADR-14 ต้องการให้ภาษาอื่น map จาก **ตำแหน่ง QWERTY** ของ layout นั้นโดยอัตโนมัติ

## การตัดสินใจ

### 1. Core: `Keymap` แบบ dynamic + profile ต่อ slot

```rust
pub struct Keymap {
    pub cells:  [[Key; 8]; 8],          // layer 1
    pub cells2: [[Option<Key>; 8]; 8],  // layer 2
}

pub struct LangSlot {
    pub index: u8,
    pub lang: Lang,        // EN/TH = ชื่อกลุ่มเดิม · layout อื่นใช้กลุ่มของ EN
    pub code: String,      // "ru", "el" — key ของ override/HUD
    pub label: String,     // "Russian (Russia)"
    pub keymap: Option<Keymap>, // None = built-in ของ lang
}

pub struct TypingPipeline {
    lang, code, label,
    table: Option<Keymap>,
    overrides: BTreeMap<(String, u8, u8, u8), Option<Key>>, // key ด้วย code
}
```

- `key_at()`: override → `table` (ถ้ามี) → built-in
- `set_profile(lang, code, label, table)` ใช้ตอนสลับภาษา
- override เปลี่ยนจาก key ด้วย `Lang` → **`code`** เพื่อไม่ชนกับ EN

### 2. สร้างตารางจาก HKL (`rnitype-win/src/language.rs`)

```
target    = LoadKeyboardLayoutW(klid, KLF_NOTELLSHELL)
reference = LoadKeyboardLayoutW("00000409")        // EN
for each cell ในตาราง EN:
  VkKeyScanExW(char, reference)  → VK + shift state
  MapVirtualKeyExW(VK, reference) → scan
  ToUnicodeEx(VK, scan, shift?, target) → ตัวอักษรของ layout เป้าหมาย
  ถ้าไม่ได้ (dead key/ต้อง Ctrl/AltGr/ผลลัพธ์แปลก) → คงตัวอักษร EN ไว้
```

| เรื่อง | ค่า |
| --- | --- |
| layer 2 | แปลงเฉพาะช่อง `Char` · ปุ่ม named (F1/Tab/…) คงเดิม |
| dead key | เรียก `ToUnicodeEx` ซ้ำ 1 ครั้ง · ถ้ายังไม่ได้ผล 1 หน่วย → คงค่า EN |
| surrogate pair | รวม high/low เป็น char เดียว |
| KLID ปลอม | ตรวจ lang id 4 หลักท้ายของ KLID กับ HKL ที่โหลดได้ — ไม่ตรง = `None` |
| IME | ยังไม่เปิด (JA/ZH/KO) — ต้อง passthrough scancode แยก slice |

### 3. App

- `language_slots(config)` → `[EN] + enabled` โดย layout อื่นสร้างตารางจาก `klid`
  (config entry หรือ layout ที่ตรวจพบ) · สร้างไม่ได้ = log แล้วข้าม
- `defaults.typing.mode = "ru"` เปิด session ด้วยภาษานั้นทันที (activate หลัง set slots)
- Language toggle: เปิด layout ที่ติดตั้งได้ (เก็บ `klid`/`label`/`kind` ลง config) · IME ตอบ `LANG_NOT_FOUND`
- `keymap_payload` ใช้ตารางจริง (built-in หรือสร้างจาก KLID) → แท็บ Keymap แก้ RU/EL ได้
- live reload เดิม: เปลี่ยนภาษา/override มีผลภายใน 250 ms

## ผลที่ตามมา

- `cargo test --workspace` = **219 tests** (core 101 · win 58 · app 60)
  - win: EN identity table, RU `a→ф`/`A→Ф`
  - core: custom table ชนะ built-in, override ยังชนะ custom
- E2E จริง (config mode=ru + fake pad): `[type] ф к и Э 5`
- settings-smoke **29/29** · contracts 12/12
- ยังไม่ทำ: IME passthrough, layout ที่ไม่ได้ติดตั้งแต่มี KLID (config เขียนมือ),
  shift-state ที่ต้องใช้ AltGr, ตาราง cache ลง config
