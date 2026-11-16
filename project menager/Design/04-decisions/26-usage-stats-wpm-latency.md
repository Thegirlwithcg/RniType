---
title: ADR-26 — Usage stats: WPM + action latency (P6 slice 1)
status: accepted
updated: 2026-11-16
tags: [decision, measurement, stats, settings]
---

# ADR-26 — P6 slice 1: ตัวเลข WPM / action latency / uptime

## บริบท

Roadmap กำหนดว่า "ทุก loop ต้องมีตัวเลข: WPM, error rate, latency, CPU — ไม่มีตัวเลข = ยังไม่เสร็จ"
แต่ก่อนหน้านี้ runtime มีแค่ `sent` (จำนวน action) และ log `[type]` — ไม่มี WPM หรือ latency
ให้ owner ใช้ตัดสินว่าเร็วพอหรือยัง (เป้า M5: 12 WPM ไทย · 20 WPM อังกฤษ)

## การตัดสินใจ

### 1. WPM = 5 ตัวอักษรต่อคำ (หน้าต่าง 60 วินาที)

- `Session` เก็บ `commits: VecDeque<Instant>` — push ทุกครั้งที่ส่ง `Action::Key` ที่เป็น `Key::Char`
  (ปุ่ม named/ลูกศร/เมาส์/คีย์ลัดไม่นับ)
- ตัดรายการที่เก่ากว่า 60 วิทุกครั้งที่ push → WPM = `commits.len() / 5`
- `chars` = ยอดตลอด session (ใช้เทียบกับ log)
- สูตรอยู่ใน `crates/rnitype-app/src/stats.rs` (`wpm()`) — test ได้โดยไม่ต้องมีนาฬิกา

**ทำไม 60 วิ**: ค่า WPM ต้องตอบ "ตอนนี้เร็วแค่ไหน" ไม่ใช่เฉลี่ยทั้งวัน (คนพักกลางทางทำค่าเฉลี่ยตกลวง)

### 2. Action latency = เวลาของ `session.step`

- วัดรอบ `Instant::now()` ครอบ `session.step` (state machine + `SendInput`) ทุกเฟรม
- เก็บ avg/max ต่อวินาที แล้วรีเซ็ตหน้าต่าง
- ตัวเลข = **controller sample → ส่งคีย์ออก** (ส่วน poll จอยอยู่ใน `sample` ก่อนหน้า ~4 ms)
- `SendInput` คืนเร็ว (0.01 ms avg) แต่มี spike (max ~1.8 ms) จาก scheduler — รายงานทั้งคู่

### 3. ที่แสดงผล

| ที่ | ข้อมูล |
| --- | --- |
| log ทุก 10 วิ | `[stats] chars=15 wpm=3.0 actions=15 step avg=0.005 ms max=0.748 ms uptime=10s` |
| `AppState.stats` | อัปเดตทุก 1 วิ — Settings อ่านผ่าน `get_stats` |
| Settings → General → Usage | Characters · WPM · 60 s · Action latency (avg / max) · Uptime |

`get_stats` เป็น command เพิ่ม (additive) — ไม่แตะ event `app:status`

### 4. ข้อจำกัดที่บันทึกไว้

- WPM จาก `--fake-pad` (~17 WPM steady) **ไม่ใช่ตัวเลขผู้ใช้จริง** — ต้องรอ owner ทดสอบกับจอย
- ยังไม่วัด error rate (ต้องมี reference text/diff) และ CPU — P6 slice ถัดไป
- latency ยังไม่รวมเวลาที่ OS/game ใช้ประมวลผลคีย์ (วัดได้แค่ฝั่งเรา)

## ผลที่ตามมา

- `cargo test --workspace` = **205 tests** (core 98 · win 56 · app 51)
- log จริง: `chars=15 wpm=3.0 actions=15 step avg=0.005 ms max=0.748 ms uptime=10s` (และ steady ~15–17 WPM จาก fake-pad)
- Settings General มีการ์ด Usage 4 ช่อง · settings-smoke = **20/20**
