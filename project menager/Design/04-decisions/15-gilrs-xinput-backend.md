---
title: ADR-15 — ใช้ gilrs XInput backend แทน WGI
status: accepted — 2026-11-15
updated: 2026-11-15
tags: [decision, input, gilrs, xinput]
---

# ADR-15 — เปลี่ยน gilrs backend เป็น XInput

## บริบท

P0 Spike A ต้องพิสูจน์ว่าอ่านจอย Xbox ได้ครบก่อนเริ่ม M1
ผลบนเครื่อง owner:

| การทดสอบ | ผล |
| --- | --- |
| Windows `joy.cpl` | เห็นจอย + ขยับ LS/RS/กดปุ่มครบ ✅ |
| `gilrs` backend WGI (ค่าเริ่มต้น) | เห็นชื่อ `Xbox 360 Controller for Windows` แต่ event = 0 ทั้ง 2 รอบ ❌ |
| `gilrs` backend XInput | แกน ±1.0 ครบ, 8 sectors, ปุ่ม 14/16 ✅ |

WGI enumerate จอยได้แต่ไม่ส่ง event → เป็นปัญหาที่ backend ไม่ใช่ฮาร์ดแวร์

## คำตอบ

**ใช้ `gilrs` กับ feature `xinput` เป็นค่าเริ่มต้นบน Windows**

```toml
gilrs = { version = "0.11", default-features = false, features = ["xinput"] }
```

- เก็บ WGI ไว้เป็น feature flag เพื่อทดสอบภายหลัง
- ไม่เปลี่ยนไป `sdl3` ตอนนี้ เพราะ XInput แก้ปัญหา Xbox ได้โดยไม่ต้องเพิ่ม DLL/link setup
- `sdl3` ยังเป็นทางเลือกเมื่อต้องรองรับ DualSense เต็มรูปแบบ (gyro/touchpad/adaptive trigger)

## ผลกระทบ

| เรื่อง | ผล |
| --- | --- |
| Trigger | XInput ให้ LT/RT เป็น **digital** — `LZ/RZ = 0`; เพียงพอสำหรับ Shift toggle/คลิกขวา |
| DualSense | ยังไม่ทดสอบ (เลื่อนก่อน release); XInput อาจเห็นเป็นจอย Xbox พื้นฐาน |
| Gyro / touchpad | ไม่มีใน v1 — ต้องใช้ SDL3/HIDAPI ถ้าจะทำภายหลัง |
| จำนวนจอย | XInput รองรับสูงสุด 4 ตัว (เพียงพอ) |
| Hotplug | ต้องทดสอบเพิ่มใน P3 (ต่อ/ถอดระหว่างรัน) |
| Deadzone | เครื่องนี้ idle peak 0.194 → ต้องมี per-device calibration ใน P3 (อย่าขยับ `ls_activate`) |

## งานค้าง

- [x] กด LS/RS click (L3/R3) แล้วยืนยันว่า probe เห็นครบ 16 ปุ่ม
- [ ] ทดสอบ hotplug (ถอด/ต่อจอยระหว่างรัน)
- [ ] DualSense ก่อน release — ถ้าต้องใช้ฟีเจอร์เฉพาะ ให้เพิ่ม SDL3 backend
- [ ] Per-device deadzone calibration (P3)
