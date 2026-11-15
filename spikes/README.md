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
| **A — จอย** | 🟡 **รอทดสอบ XInput** | `joy.cpl` ผ่านทุกปุ่ม ✅ · gilrs/WGI event = 0 ❌ → สลับเป็น XInput แล้ว · **DualSense เลื่อน** |
| **B — Tauri overlay** | ✅ **ผ่าน** | โปร่งใส ✅ · ย้ายจอ ✅ · click-through ✅ · target เป็น foreground ✅ (`match=true` 4/4) |
| **C — SendInput ไทย** | ✅ **ผ่าน** | พิมพ์ `กขคง Hello ไทย 123` ลง Notepad ถูกทุกตัว |

**สรุป P0: 2/3 ผ่าน (B, C)** — เหลือ A อย่างเดียว
B ผ่านแล้ว = Tauri overlay ใช้เป็น HUD ได้ ไม่ต้องถอย egui

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

### ผลดิบ A — รอบ 2 (เหมือนรอบแรก)

```text
--- สรุป (20.003643 s) ---
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
```

**ตีความ A:** `joy.cpl` แสดงค่าครบทุกปุ่ม → **ตัดสาเหตุฮาร์ดแวร์ออก**
เหลือสาเหตุเดียวคือ gilrs/WGI ไม่ส่ง event บนเครื่องนี้ → เปลี่ยน backend เป็น **XInput** แล้ว

**ลำดับตรวจต่อ:**
1. รัน probe (default = XInput) → ต้องเห็น `OK: ได้รับ input แล้ว` + ค่าแกน/ปุ่มครบ
2. ถ้า XInput ยัง 0 → รันแบบ WGI เพื่อยืนยันว่าเป็นที่ backend
3. ถ้าทั้งคู่ 0 → ทำ spike `sdl3` ตามทางถอย

## ขั้นถัดไป

1. **A — ทดสอบ XInput** (default) แล้วดูว่ามี event ไหม; ถ้ายัง 0 → ทำ spike `sdl3`
2. **B — ปิดได้ ✅**
3. **C — ปิดได้ ✅**

## เกณฑ์เปิด P2–P5

- [ ] A ผ่าน: Xbox ครบ 16 ปุ่ม + แกน 8 ทิศ (DualSense เลื่อนไปก่อน release)
- [x] B ผ่าน: click-through ได้ + target เป็น foreground
- [x] C ผ่าน: SendInput ไทยลง Notepad ถูกต้อง
