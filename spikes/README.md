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

ผ่านเมื่อ: เห็น LS/RS/trigger/ปุ่มครบทั้ง **Xbox** และ **DualSense**, deadzone/drift นิ่ง
ผลที่ต้องจด: ชื่อจอย, ปุ่มที่ขาด, ค่า drift สูงสุด

**ถ้าเจอ `pads: 0`** — probe จะรอจอย 15 วินาที ลองเปิดจอย/กดปุ่ม guide ระหว่างนั้น
ถ้ายังไม่เจอ: ต่อ USB ก่อน → ปิด Steam Input/DS4Windows → ดู Device Manager → ค่อยลอง Bluetooth

probe จะมีช่วง **“ยืนยัน input: ขยับ LS หรือกด A ภายใน 10 วินาที”** — ต้องเห็น `OK: ได้รับ input แล้ว` ก่อน
ถ้าไม่ขึ้น: ปิด Steam Input/DS4Windows → ต่อ USB → ดู Device Manager

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

## บันทึกผล

| Gate | วันที่ | ผล | หลักฐาน / หมายเหตุ |
| --- | --- | --- | --- |
| **A — จอย** | 2026-11-15 | 🟡 **ยังไม่ผ่าน** | เห็น `Xbox 360 Controller for Windows` แต่ axes/buttons = 0 ทั้งหมด · ยังไม่ได้ทดสอบ DualSense |
| **B — overlay** | 2026-11-15 | ❌ **ไม่ผ่าน** | ใส ✅ · ย้ายจอ ✅ · click-through ❌ · focus ❌ |
| **C — SendInput** | 2026-11-15 | ✅ **ผ่าน** | พิมพ์ `กขคง Hello ไทย 123` ลง Notepad ถูกทุกตัว |

**สรุป P0: 1/3 ผ่าน (C)** → ยังไม่เปิด P2–P5 ตามแผน

### หลักฐาน C — พิมพ์ไทยลง Notepad

![[spikes/attachments/Pasted image 20261115-101154.png]]

### ผลดิบ A (รอบแรก)

```text
--- สรุป (20.001535 s) ---
[0] Xbox 360 Controller for Windows
  LSx  range +0.00 .. +0.00
  LSy  range +0.00 .. +0.00
  RSx  range +0.00 .. +0.00
  RSy  range +0.00 .. +0.00
  LZ   range +0.00 .. +0.00
  RZ   range +0.00 .. +0.00
  idle_peak=0.000 (deadzone 0.15 OK)  max_magnitude=0.00
  buttons: -
  sectors: -
  expected buttons: A/South, B/East, X/West, Y/North, LB, RB, LT, RT, LS, RS, D-Up, D-Down, D-Left, D-Right, View, Menu
```

**ตีความ A:** gilrs เห็นจอย (ชั้น WGI ทำงาน) แต่ไม่ได้รับ input เลย
ยัง **ไม่ใช่** ข้อสรุปว่า gilrs ใช้ไม่ได้ — ต้องรันใหม่แล้วขยับ LS/RS + กดปุ่มจริง
ถ้ายังเป็น 0: ปิด Steam Input/DS4Windows → ต่อ USB → ลอง DualSense

### ผลดิบ B

1. พื้นหลังใส ✅
2. click-through ❌
3. ไม่เป็น foreground ❌
4. ย้ายจอได้ ✅

**ตีความ B:** transparency + multi-monitor ผ่าน; ที่เหลือคือ click-through/focus ซึ่งเป็นเกณฑ์ตัดสิน Tauri
→ รัน `run.ps1` แล้วกด “ทดสอบคลิกทะลุ 10 วิ” + คลิกปุ่มใน **target window** แล้วดูว่า PASS หรือไม่
ถ้ายัง fail → ตามแผนถอยไป **egui** หรือทำ click-through ด้วย Win32 เอง แล้วเขียน ADR

## ขั้นถัดไป

1. **A — รันใหม่** แล้วขยับ LS/RS 8 ทิศ + กดปุ่มให้ครบ; ต้องเห็น `OK: ได้รับ input แล้ว`
   ```powershell
   cargo run -p rnitype-probe -- --seconds 20 --raw
   ```
2. **B — รัน target harness** แล้วเก็บ log
   ```powershell
   powershell -File spikes/overlay/run.ps1
   ```
   ต้องเห็น `PASS — ได้รับคลิก` และ `target click: ... match=true`
3. **C — ปิดได้ ✅**

## เกณฑ์เปิด P2–P5

- [ ] A ผ่าน: Xbox + DualSense, input ครบ 16 ปุ่ม + แกน 8 ทิศ
- [ ] B ผ่าน: click-through ได้ + target เป็น foreground
- [x] C ผ่าน: SendInput ไทยลง Notepad ถูกต้อง
