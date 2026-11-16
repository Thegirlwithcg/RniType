---
title: ADR-29 — P6 slice 2: CPU/RAM + corrections proxy
status: accepted
updated: 2026-11-16
tags: [decision, measurement, stats]
---

# ADR-29 — P6 slice 2: CPU/RAM ของโปรเซส + corrections

## บริบท

P6 slice 1 วัด WPM และ action latency ได้แล้ว แต่ roadmap ยังต้องการ CPU และ "error rate"
ก่อนตัดสินว่าแอปเบาพอสำหรับ background utility และก่อนทดสอบคนจริง

## การตัดสินใจ

### 1. CPU ของโปรเซส

- `GetProcessTimes` → kernel+user time (100 ns) · เก็บ sample ก่อนหน้าใน loop
- `cpu_percent(delta_cpu_100ns, delta_wall_ms)` = CPU time / wall time × 100 (ปัด 0.1)
- วัดเฉพาะโปรเซส RniType (ไม่รวม WebView2 child processes — บันทึกเป็นข้อจำกัด)
- แสดง/лог ทุก 1/10 วินาที

### 2. Memory

- `K32GetProcessMemoryInfo` → `WorkingSetSize` (bytes) → MB
- ใช้ Working Set (ไม่ใช่ private bytes) เพราะเป็นตัวเลขที่ผู้ใช้เห็นใน Task Manager

### 3. Corrections (proxy ของ error rate)

- นับ `Action::Key(Key::Named(Backspace))` ที่ส่งออก (`Session::backspaces`)
- `corrections` = จำนวนครั้งที่ผู้ใช้ลบ — **ไม่ใช่ error rate จริง** เพราะไม่รู้ข้อความอ้างอิง
- ใช้เป็นสัญญาณคุณภาพ: ถ้า corrections/chars สูงผิดปกติ = mapping หรือ gesture มีปัญหา

### 4. ที่แสดงผล

| ที่ | ข้อมูล |
| --- | --- |
| log ทุก 10 วิ | `[stats] chars=15 wpm=3.0 actions=15 corrections=0 step avg=0.004 ms max=0.729 ms cpu=3.1% rss=32 MB uptime=10s` |
| `get_stats` | เพิ่ม `cpuPercent`, `rssBytes`, `corrections` (additive) |
| Settings → General → Footprint | CPU · Memory · Corrections + คำอธิบายว่า corrections เป็น proxy |

## ผลที่ตามมา

- `cargo test --workspace` = **219 tests** (core 101 · win 58 · app 60) รวม `cpu_percent`/`format_mb`
- E2E จริง: `[stats] … cpu=3.1% rss=32 MB corrections=0 uptime=10s`
- ยังไม่ทำ: error rate จากข้อความอ้างอิง (diff), CPU ของ WebView2 child, RAM peak/private,
  installer/Steam/itch
