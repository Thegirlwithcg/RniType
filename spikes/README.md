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

| Gate | วันที่ | ผล | หมายเหตุ |
| --- | --- | --- | --- |
| A — จอย | 2026-11-15 | 🟡 เห็น `Xbox 360 Controller for Windows` แต่ axes/buttons = 0 ทั้งหมด | ต้องขยับ LS/RS + กดปุ่มระหว่าง 20 วิ; ยังไม่ได้ทดสอบ DualSense |
| B — overlay | 2026-11-15 | ❌ ใส ✅ · ย้ายจอ ✅ · click-through ❌ · focus ❌ | แก้ harness เป็น target window + log แล้ว รอรอบ 2 |
| C — SendInput | 2026-11-15 | 🟡 ส่งออกได้ 18 units แต่ไปลง PowerShell (ไม่ได้โฟกัส Notepad) | เปลี่ยนเป็นกด F8 หลังคลิก Notepad แล้ว รอรอบ 2 |



ผลของ A

\--- สรุป (20.001585 s) ---

\[0\] Xbox 360 Controller for Windows

  LSx  range +0.00 .. +0.00

  LSy  range +0.00 .. +0.00

  RSx  range +0.00 .. +0.00

  RSy  range +0.00 .. +0.00

  LZ   range +0.00 .. +0.00

  RZ   range +0.00 .. +0.00

  idle\_peak=0.000 (deadzone 0.15 OK)  max\_magnitude=0.00

  buttons: -

  sectors: -

  expected buttons: A/South, B/East, X/West, Y/North, LB, RB, LT, RT, LS, RS, D-Up, D-Down, D-Left, D-Right, View, Menu

OK: probe รันจบ — ส่ง log นี้ให้ agent เพื่อปิด spike A



ผลของ B  
1.พื้นหลังใส  
2.คลิ๊กทะลุ ไม่ได้ พิมพ์ Note pad ไม่ได้  
3\. Notepad ไม่เป็น Foreground   
4.ย้ายจอได้



C.ข้อความ: "กขคง Hello ไทย 123" (18 UTF-16 units)

สลับไป Notepad แล้วคลิกวาง cursor — เริ่มใน 3 วินาที...

  3...

  2...

  1...

ส่งแล้ว 18 units — ตรวจ Notepad:

  ผ่าน = ข้อความตรงทุกตัว

  ไม่ผ่าน = จดสิ่งที่ได้ + เวอร์ชัน Windows/Notepad ไว้ใน HANDOFF

PS D:\\Erin's Works\\Programming\\RniType> กขคง Hello ไทย 123
