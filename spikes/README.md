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

## B — Tauri overlay

```powershell
powershell -File spikes/overlay/run.ps1
```

ผ่านเมื่อ:
1. panel โปร่งใส เห็น desktop รอบ ๆ
2. กด “ทดสอบคลิกทะลุ 5 วิ” → คลิก/พิมพ์ Notepad ด้านหลังได้
3. Notepad ยังเป็น foreground (ไม่แย่ง focus)
4. ย้ายไปจอ 2 ที่ 150%/200% แล้วตำแหน่งถูก

**ข้อควรรู้:** path ของ vault มี `'` (Erin's Works) → `tauri-build`/RC.EXE อ่าน path icon ไม่ได้
(`RC2135`) จึงต้องรันผ่าน `run.ps1` ที่ copy icon ไป `%TEMP%` และตั้ง `TAURI_CONFIG`
ถ้าย้ายโปรเจกต์ไป path ที่ไม่มี apostrophe จะรัน `cargo run` ตรง ๆ ได้

## C — SendInput ไทย

```powershell
cargo run --release --manifest-path spikes/sendinput/Cargo.toml
```

เปิด Notepad, วาง cursor, รอ 3 วินาที
ผ่านเมื่อ: `กขคง Hello ไทย 123` ตรงทุกตัว

## บันทึกผล

| Gate | วันที่ | ผล | หมายเหตุ |
| --- | --- | --- | --- |
| A — จอย | — | ยังไม่รัน | |
| B — overlay | 2026-11-15 | 🟡 รันได้: panel โปร่งใส + เห็น 2 จอ (0: 1920×1080, 1: −1920,0) | ยังต้องทดสอบ click-through / ไม่แย่ง focus / DPI |
| C — SendInput | — | ยังไม่รัน | compile ผ่าน |
