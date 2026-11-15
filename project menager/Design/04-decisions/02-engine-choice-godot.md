---
title: ADR-02 — ใช้ Godot เป็นแกนแอปหรือไม่ (ขายบน Steam / itch)
status: accepted (ข้อเสนอ)
updated: 2026-11-15
tags: [decision, engine, distribution]
---

# ADR-02 — Godot เหมาะกับ RniType ไหม

> **อัปเดต 2026-11-15**: ภาษาหลักเปลี่ยนจาก C# เป็น **Rust + Tauri** แล้ว ([[03-stack-rust]])
> แต่ข้อสรุป "ไม่ใช้ Godot เป็นแกน" และข้อมูลช่องทางขายยังใช้ได้ทั้งหมด

## บริบท

RniType เป็น **utility พื้นหลังบน Windows** ที่รับค่าจอย → แปลงเป็นคีย์บอร์ด/เมาส์
มีหน้าตั้งค่าแบบ Logi Options+ และวางแผนขายบน **Steam** และ **itch.io**
คำถาม: ควรสร้างด้วย Godot หรือไม่

## ข้อเท็จจริงเรื่องช่องทางขาย (ไม่เกี่ยวกับเอนจิน)

| เรื่อง | Steam | itch.io |
| --- | --- | --- |
| ค่าธรรมเนียม | Steam Direct ~$100 ต่อแอป (ได้คืนเมื่อยอดขายถึงเกณฑ์) | ฟรี |
| ประเภทสินค้า | ขาย software/utility ได้ (ไม่ใช่แค่เกม) | ขายอะไรก็ได้ |
| ต้องใช้เอนจินไหม | **ไม่** — อัปโหลด build .exe/.zip ได้เลย | **ไม่** |
| ระบบที่ต้องทำเอง | Steamworks SDK (achievements ไม่จำเป็น, cloud save ไม่จำเป็น), หน้า store, อัปเดตผ่าน Steam | หน้า product, อัปเดตเอง (butler CLI) |
| ภาษี/ payout | ต้องกรอกข้อมูลภาษี + ธนาคาร | กรอกในหน้า dashboard |

> สรุป: **การขายบน Steam/itch ไม่ใช่เหตุผลให้เลือก Godot** — ทั้งคู่รับ executable ทั่วไป

## เทียบกับงานที่แอปนี้ต้องทำจริง

| งาน | Godot 4 | Rust / .NET 8 (ที่เลือกไว้) |
| --- | --- | --- |
| ส่งคีย์/เมาส์ (`SendInput`) | ต้อง P/Invoke เอง (ทำได้ แต่ไม่ได้อะไรเพิ่ม) | เรียก API ตรง |
| อ่านจอยหลายรุ่น | ผ่าน SDL3 หรือ Godot Input (จำกัดการแมป) | SDL3 ตรง คุม deadzone/kind ได้ |
| Tray icon + เมนู | **ไม่มีในตัว** ต้องทำ native plugin | มีในตัว (WinForms/WinUI) |
| Autostart / Task Scheduler / สิทธิ์ admin | ต้องเขียนเพิ่ม | มี Inno/MSIX + API |
| Single instance + ส่งคำสั่งไป instance เดิม | ต้องเขียนเอง | มาตรฐาน .NET |
| Overlay คลิกทะลุ ไม่แย่ง focus | ทำได้ (transparent window) แต่ต้องตั้ง Win32 เพิ่ม | WPF + `WS_EX_TRANSPARENT` ตรงไปตรงมา |
| IME ไทย / ช่องพิมพ์ข้อความ | **ไม่มี native text input** ต้องทำเอง | มี TextBox + IME ที่ใช้ได้ทันที |
| Accessibility / screen reader | ทำเอง | ได้จาก native controls |
| ขนาดติดตั้ง | หลายสิบ MB ถึง ~100 MB (C# build ใหญ่กว่า) | self-contained ~40–70 MB (trim/NativeAOT เล็กลง) |
| เวลาเปิดโปรแกรม | ~1–3 s (โหลดเอนจิน) | < 1 s |
| CPU ตอน idle (poll 250 Hz) | ต้องดูแล render loop ไม่ให้กิน CPU | thread ธรรมดา คุมง่าย |
| ต้นทุนเรียนรู้ | ต้องเขียน GDExtension/C# interop | ภาษาเดียวจบ |

## Godot เก่งเรื่องอะไร (และจะใช้เมื่อไหร่)

- UI แบบคอนโซล/เกม, animation, 3D, onboarding แบบ interactive → **ทำได้สวยกว่า WPF มาก**
- ถ้าอนาคตอยากมี "โหมดสอนพิมพ์" ที่เป็นมินิเกม 3D หรือ tutorial สนุกๆ → Godot เหมาะ
- แต่ถ้าเอามาเป็น **แกนของ background service** จะได้ overhead โดยไม่ได้ประโยชน์
  (ต้องเขียน Win32 interop เองอยู่ดี ทั้ง tray, SendInput, overlay flags)

## ตัวเลือกที่พิจารณา

| ทางเลือก | ผล |
| --- | --- |
| **A. .NET 8 + WPF/WinUI + SDL3** | **เลือก** — ตรงกับงาน, ของครบ, ขนาดและ latency ดี |
| B. Godot เป็นแกนทั้งหมด | ไม่เลือก — ขาด tray/IME/autostart, ต้อง interop หนัก, เริ่มช้า, ไฟล์ใหญ่ |
| C. Godot เป็นแค่หน้าต่างตั้งค่า (แยกโปรเซส) | เป็นไปได้ในอนาคต ถ้าอยาก UI สวยแบบเกม — คุยกันทีหลัง |
| D. Godot สำหรับ onboarding/มินิเกมแยก | เปิดทางไว้ (M6+) — ไม่กระทบสถาปัตยกรรมหลัก |
| E. Avalonia (.NET) | ทางเลือกถ้าจะรองรับ macOS/Linux ในอนาคต |

## ผลที่ตามมา

- ยืนยันสแตกเดิมใน [[01-architecture]]: Rust + Tauri v2, SendInput, overlay โปร่งใส
- ใช้ SFX/animation เบาๆ ใน UI (WebView2) ได้ (ไม่ต้องมีเอนจิน)
- ถ้าจะทำ onboarding แบบเกม ให้เพิ่มเป็น **โปรเซสแยก** (Godot export) เปิดจากเมนู — ตัดสินใจตอน M6

## ความเสี่ยงเฉพาะของการขาย software แบบนี้ (ต้องเผื่อเวลา)

1. **SmartScreen / Antivirus** — แอปที่ inject คีย์จะถูกตั้งคำถาม → ต้อง code signing (EV หรือ OV + reputation)
2. **Steam Input ทับจอย** — Steam อาจ intercept จอยก่อนแอป → ต้องมีคู่มือ "ปิด Steam Input ต่อแอปนี้"
3. **นโยบายคืนเงิน** — ต้องมี trial/รุ่นฟรี เพราะผู้ใช้ตัดสินใจยาก
4. **อัปเดต** — Steam จัดการให้; itch ต้องใช้ butler + ปุ่มอัปเดตในแอป
5. **สิทธิ์ admin** — ถ้าแอปต้อง elevated จะเตือน UAC ทุกครั้ง → อธิบายให้ชัด หรือใช้ Task Scheduler

## คำตอบสั้น

**ไม่แนะนำให้ใช้ Godot เป็นแกนของ RniType** — Godot เก่งเรื่องเกม แต่แอปนี้เป็น
system utility ที่พึ่ง Win32/SendInput/tray/IME ซึ่ง .NET ทำได้ตรงกว่าและเบากว่า
การขายบน Steam/itch ไม่ได้บังคับให้ใช้เอนจินใดๆ
