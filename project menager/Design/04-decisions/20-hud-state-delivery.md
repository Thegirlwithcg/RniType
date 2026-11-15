---
title: ADR-20 — การส่ง HudState ให้ถึงจอเสมอ (handshake resend + heartbeat + single instance)
status: accepted (2026-11-15)
updated: 2026-11-15
tags: [decision, hud, ipc, runtime]
---

# ADR-20 — "UI ค้างจอ" เพราะ state ไม่ถึง HUD

## อาการที่ owner เจอ

- วงล้อค้างอยู่บนจอตลอดเวลา ทั้งที่ไม่ได้แตะจอย (pad-log: `state=Idle` ต่อเนื่อง)
- เห็น chip `Type TH` สองอัน / วงซ้อนกันในบาง screenshot

## สาเหตุ (ยืนยันด้วย CDP)

1. **แอปส่ง state ครั้งแรกก่อน HUD พร้อม** แล้ว dedupe ด้วย `text != last_json`
   → payload ไม่เปลี่ยนอีกเลย = **ไม่ส่งซ้ำตลอดไป** HUD จึงค้างที่ค่าเริ่มต้นของ HTML
   (`data-visible="false"`, `mode="disabled"`) หรือค้างที่ state สุดท้ายที่เคยได้
2. `hud_ready` (handshake) ส่ง state ซ้ำ **เฉพาะโหมด `--demo`** (`snapshot_for_hud` ใช้ demo source)
   path runtime จึงไม่มีการ resync ตอน HUD บูตช้า/รีโหลด
3. **เปิดแอปซ้ำได้** → overlay 2 หน้าต่างซ้อนกัน (chip 2 อัน) + injector 2 ตัวแย่งกันพิมพ์

## การตัดสินใจ

| กลไก | ทำอะไร |
| --- | --- |
| `hud_ready` → `hud_need_resend = true` | รอบ emit ถัดไปล้าง cache → ส่ง state ปัจจุบันทันที (ไม่ว่า payload จะเท่าเดิม) |
| **heartbeat 2 วิ** (`HUD_RESEND_MS`) | ส่งซ้ำแม้ไม่เปลี่ยน → ถ้า event หลุด/ถูก throttle HUD กลับมาตรงภายใน ≤ 2 วิ |
| dedupe 60 Hz ยังอยู่ | ยังคงส่งเฉพาะเมื่อเปลี่ยนในระดับเฟรม ประหยัด IPC |
| **single instance** (`Local\RniType.SingleInstance` mutex) | instance ที่สองพิมพ์ "รันอยู่แล้ว" แล้วออกทันที |

## ผลที่ตามมา

- `crates/rnitype-app/src/main.rs`: `AppState.hud_need_resend`, `already_running()`, heartbeat ใน `run_loop`
- `windows-sys` เพิ่ม feature `Win32_Security` + `Win32_System_Threading` (CreateMutexW)
- ตรวจด้วย CDP กับแอปจริง (จอยต่อ, ไม่แตะจอย):
  `{"state":"idle","rings":"off","mode":"type-th","visible":"true","ringOpacity":"0","pill":"Type TH"}`
  → เห็นแค่ pill ไม่มีวง ✓
- เปิด instance ที่สอง → `[app] RniType รันอยู่แล้ว — ปิดตัวเก่าก่อน` + exit code 1 ✓

## ความเสี่ยง

| ความเสี่ยง | ทางรับมือ |
| --- | --- |
| heartbeat 2 วิ กิน IPC | ~1 KB/2 วิ (เล็กน้อยเทียบ 60 Hz ปกติ) |
| mutex ค้างหลังโปรเซสตาย | named mutex ถูกปล่อยเมื่อโปรเซสจบเอง; ถ้าค้างผิดปกติให้ reboot หรือแก้ชื่อ `Local\RniType.SingleInstance` |
| HUD ยังค้างเพราะ compositor (ไม่ใช่ event) | ถ้าเจออีก: เพิ่ม `--disable-features=CalculateNativeWinOcclusion` ให้ WebView2 (ยังไม่ทำ เพราะยังพิสูจน์สาเหตุไม่ได้) |
