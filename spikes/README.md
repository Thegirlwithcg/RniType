# Spikes — P0 gates

> รันบนเครื่อง owner เท่านั้น (ต้องมีจอย + 2 จอ)
> ผ่านครบ A/B/C → เปิด P2–P5 · ไม่ผ่าน → ดูทางถอยใน [[03-stack-rust]]
>
> **รันจาก repo root** (`D:\Erin's Works\Programming\RniType`) หรือใช้ absolute path
> ถ้ารันจาก `crates/rnitype-probe` path ของ spikes จะหาไม่เจอ

## A — จอย (gilrs)

```powershell
cargo run -p rnitype-probe -- --seconds 20 --raw
```

ผ่านเมื่อ (รอบนี้): **Xbox** เห็น LS/RS/trigger/ปุ่มครบ + deadzone/drift นิ่ง
ผลที่ต้องจด: ชื่อจอย, ปุ่มที่ขาด, ค่า drift สูงสุด

> **DualSense: เลื่อนไปก่อน** (owner ยังไม่มีฮาร์ดแวร์ให้ลอง)
> ต้องทดสอบก่อน release — บันทึกไว้ใน [[02-roadmap-and-smoke-tests]] M0

**ถ้าเจอ `pads: 0`** — probe จะรอจอย 15 วินาที ลองเปิดจอย/กดปุ่ม guide ระหว่างนั้น
ถ้ายังไม่เจอ: ต่อ USB ก่อน → ปิด Steam Input/DS4Windows → ดู Device Manager → ค่อยลอง Bluetooth

probe จะมีช่วง **“ยืนยัน input: ขยับ LS หรือกด A ภายใน 10 วินาที”** — ต้องเห็น `OK: ได้รับ input แล้ว` ก่อน
ถ้าไม่ขึ้น: ปิด Steam Input/DS4Windows → ต่อ USB → ดู Device Manager

> **2026-11-15:** `joy.cpl` เห็นค่าครบทุกปุ่ม → ฮาร์ดแวร์/Windows ปกติ
> แต่ probe (gilrs/WGI) ได้ 0 → เปลี่ยน backend เป็น **XInput** แล้ว (`default = ["xinput"]`)
> ทดสอบ WGI แบบเดิม: `cargo run -p rnitype-probe --no-default-features --features wgi -- --seconds 20 --raw`

**ถ้าเห็นชื่อจอยแต่ range เป็น 0 ทั้งหมด** — ระหว่าง 20 วินาทีต้อง **ขยับ LS/RS ให้สุด 8 ทิศ + กด A/B/X/Y/LB/RB/trigger/D-pad**
ถ้าขยับแล้วยัง 0: ปิด Steam Input/DS4Windows แล้วรันใหม่ (Steam อาจ intercept จอย)

## B — Tauri overlay

```powershell
powershell -File spikes/overlay/run.ps1
```

ผ่านเมื่อ:

1. panel โปร่งใส เห็น desktop รอบ ๆ
2. กด “ทดสอบคลิกทะลุ 10 วิ” → panel จะจางหาย เห็น **target window** ด้านหลัง
3. คลิกปุ่มใหญ่ใน target window → ต้องขึ้น `PASS — ได้รับคลิก` และ overlay กลับมารับคลิก
4. ระหว่างนั้น target window เป็น foreground (overlay ไม่แย่ง focus)
5. ย้ายไปจอ 2 ที่ 150%/200% แล้วตำแหน่งถูก

> target window ถูกวางใต้ overlay อัตโนมัติ — ไม่ต้องใช้ Notepad แล้ว
> terminal จะพิมพ์ `click_through=... exstyle=...` และ `target click: ... match=...` ให้เก็บ log

**ข้อควรรู้:** path ของ vault มี `'` (Erin's Works) → `tauri-build`/RC.EXE อ่าน path icon ไม่ได้
(`RC2135`) จึงต้องรันผ่าน `run.ps1` ที่ copy icon ไป `%TEMP%` และตั้ง `TAURI_CONFIG`
ถ้าย้ายโปรเจกต์ไป path ที่ไม่มี apostrophe จะรัน `cargo run` ตรง ๆ ได้

## C — SendInput ไทย

```powershell
cargo run --release --manifest-path spikes/sendinput/Cargo.toml
```

เปิด Notepad → คลิกวาง cursor → **กด F8** (ไม่ต้องกลับมาที่ terminal)
ผ่านเมื่อ: `กขคง Hello ไทย 123` ตรงทุกตัวใน Notepad

> ถ้าข้อความไปลง terminal แทน = F8 ถูกกดตอน terminal ยัง focus อยู่ → คลิก Notepad ก่อนแล้วกด F8 ใหม่

## บันทึกผล (2026-11-15)

| Gate | ผล | หลักฐาน / หมายเหตุ |
| --- | --- | --- |
| **A — จอย** | ✅ **ผ่าน (XInput)** | แกน ±1.0 ครบ · 8 sectors · ปุ่ม **16/16** (LS/RS จากรอบ 3) · DualSense เลื่อน |
| **B — Tauri overlay** | ✅ **ผ่าน** | โปร่งใส ✅ · ย้ายจอ ✅ · click-through ✅ · target เป็น foreground ✅ (`match=true` 4/4) |
| **C — SendInput ไทย** | ✅ **ผ่าน** | พิมพ์ `กขคง Hello ไทย 123` ลง Notepad ถูกทุกตัว |

**สรุป P0: 3/3 ผ่าน** (DualSense เลื่อนไปก่อน release) → เปิด P2–P5 ได้

### A — หลักฐานรวม 2 รอบ

รอบ 2 — แกน/8 sectors/ปุ่ม 14 ตัว:

```text
--- สรุป (20.00214 s) ---
[0] Xbox Controller
  LSx  range -1.00 .. +0.99
  LSy  range -0.99 .. +1.00
  RSx  range -0.99 .. +0.95
  RSy  range -0.98 .. +0.94
  LZ   range +0.00 .. +0.00
  RZ   range +0.00 .. +0.00
  idle_peak=0.194  max_magnitude=1.00
  buttons: A/South, B/East, D-Down, D-Left, D-Right, D-Up, LB, LT, Menu, RB, RT, View, X/West, Y/North
  sectors: ← ↑ → ↓ ↖ ↗ ↘ ↙
```

รอบ 3 — กด LS/RS click:

```text
--- สรุป (20.002575 s) ---
[0] Xbox Controller
  LSx  range -0.22 .. +0.61
  LSy  range -0.79 .. +0.00
  RSx  range -0.25 .. +0.00
  RSy  range -0.97 .. +0.00
  LZ   range +0.00 .. +0.00
  RZ   range +0.00 .. +0.00
  idle_peak=0.040 (deadzone 0.15 OK)  max_magnitude=1.00
  buttons: LS, RS
  sectors: ↓ ↘
```

**สรุป A:** รวมสองรอบได้ **16/16 ปุ่ม + แกนเต็ม + 8 sectors**
- `LZ/RZ = 0` เป็นปกติของ XInput (trigger เป็น digital ที่ LT/RT) — เพียงพอสำหรับ Shift/คลิกขวา
- `idle_peak` ต่างกัน 0.040–0.194 → P3 ต้องทำ **per-device deadzone calibration** (ห้ามขยับ `ls_activate`)
- ดู [[15-gilrs-xinput-backend]] สำหรับเหตุผลที่เปลี่ยนจาก WGI

### หลักฐาน C — พิมพ์ไทยลง Notepad

![[spikes/attachments/Pasted image 20261115-101154.png]]

### ผลดิบ B — click-through ทำงาน

```text
Spike B: overlay + target started
click_through=true  exstyle=0xC0138
target click: foreground=0x861024 target=Some(0x861024) match=true
click_through=false exstyle=0x40118
click_through=true  exstyle=0xC0138
target click: foreground=0x861024 target=Some(0x861024) match=true
click_through=false exstyle=0x40118
click_through=true  exstyle=0xC0138
target click: foreground=0x861024 target=Some(0x861024) match=true
click_through=false exstyle=0x40118
click_through=true  exstyle=0xC0138
target click: foreground=0x861024 target=Some(0x861024) match=true
click_through=false exstyle=0x40118
```

**ตีความ B:** ทั้ง 4 รอบ target ได้รับคลิกและเป็น foreground → ผ่านเกณฑ์
`exstyle=0xC0138` = `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | ...` ตามที่ต้องการ

## ขั้นถัดไป

**P0 ปิดแล้ว** → เปิด P2–P5 ตาม [[03-parallel-execution-plan]]

1. **P2 — Core behaviors** (Track A): วงคำสั่ง, วงภาษา, mode arbiter, repeat, layer 2, mouse math, Nav
2. **P3 — Windows runtime** (Track B): injector จริง, loop 250 Hz, tray, config + deadzone calibration
3. **P4 — HUD overlay** (Track C): พอร์ต mockup ลง Tauri ตาม contract
4. **P5 — Settings + languages** (Track D)

## เกณฑ์เปิด P2–P5

- [x] A ผ่าน: Xbox 16 ปุ่ม + แกน 8 ทิศ (DualSense เลื่อนไปก่อน release)
- [x] B ผ่าน: click-through ได้ + target เป็น foreground
- [x] C ผ่าน: SendInput ไทยลง Notepad ถูกต้อง
