---
title: State Machine Specification
status: draft v0.1
updated: 2026-11-15
tags: [spec, state-machine]
---

# 02.4 — State machine และค่าเวลา

## 1. สถานะ (states)

| State | ความหมาย | HUD |
| --- | --- | --- |
| `DISABLED` | ปิดรับ input (ฉุกเฉิน / app ใน exclusion list / จอยหลุด) | เทา + เหตุผล |
| `IDLE` | ไม่มี gesture ค้างอยู่ | ซ่อน (หรือแถบสถานะจาง) |
| `GROUP_ARMED` | LS ผ่าน threshold แล้ว ยังไม่เลือกตัวอักษร | วงในไฮไลต์กลุ่ม |
| `CHAR_SELECTED` | RS อยู่ในภาคใดภาคหนึ่ง | วงนอกไฮไลต์ + preview |
| `COMMITTING` | RS กลับเข้า rest แล้ว กำลังรอ dwell 60 ms | preview กระพริบ |
| `REPEATING` | ยังค้าง LS + RS ทิศเดิมหลัง commit | แสดง ×n |
| `QUICKBAR` | กด LB ค้าง | 8 ช่องทางลัด |
| `CARET_NAV` | กด R3 เปิดลูกศร, RS = ลูกศร (กด R3 อีกครั้ง = ปิด) | ไม่มีวง (แถบสถานะ) |
| `CMD_WHEEL` | กด D-pad ← → วงคำสั่งในวงใน 8 ช่อง (EN, Language, Mouse, Place UI, Copy/Paste/Undo/Redo) | วงในไฮไลต์ช่องที่ RS ชี้, วงนอกจาง |
| `LANG_WHEEL` | เลือกช่อง Language ใน `CMD_WHEEL` → วงภาษา (level 2) | รหัสภาษาที่เปิดใช้ · RS เลือก |
| `MOUSE` | Mouse mode | เคอร์เซอร์ + ค่าความเร็ว |
| `NAV` | Nav mode = โหมดจัดตำแหน่ง UI (pivot) | crosshair ที่ pivot + pill |

## 2. การเปลี่ยนสถานะหลัก

```
IDLE --LS ผ่าน 0.35--> GROUP_ARMED
GROUP_ARMED --LS กลับ < 0.25--> IDLE          (ไม่พิมพ์)
GROUP_ARMED --LS เปลี่ยน sector--> GROUP_ARMED (เปลี่ยนกลุ่ม, sector = -1)
GROUP_ARMED --RS ผ่าน 0.40--> CHAR_SELECTED
CHAR_SELECTED --RS เปลี่ยน sector--> CHAR_SELECTED (เปลี่ยนตัวที่เลือก)
CHAR_SELECTED --LS เปลี่ยน sector--> GROUP_ARMED (ยกเลิกตัวที่เลือก)
CHAR_SELECTED --RS กลับ < 0.25 ค้าง 60ms--> COMMITTING
COMMITTING --LS เปลี่ยน sector--> GROUP_ARMED (ยกเลิก commit)
COMMITTING --ครบ 60ms--> IDLE + ส่งคีย์
CHAR_SELECTED --ค้างในภาค > 600ms--> IDLE     (ยกเลิก ไม่พิมพ์)
CHAR_SELECTED --LS กลับ < 0.25--> IDLE        (ยกเลิก)
หลัง commit ถ้า LS ยังค้าง + RS ยังค้างที่เดิม 350ms --> REPEATING
REPEATING --RS ปล่อย/เปลี่ยน sector--> IDLE หรือ CHAR_SELECTED
```

## 3. ลำดับความสำคัญของ event (แก้เมื่อเกิดพร้อมกัน)

1. ปุ่มปิดฉุกเฉิน (LB+RB ค้าง 1.5 s)
2. จอยหลุด / ต่อใหม่
3. เปิด/ปิดวงคำสั่ง (D-pad ←) — ตัด gesture พิมพ์ที่ค้างอยู่
4. Modifier (LT / LB)
5. ปุ่มหน้า (A/B/X/Y)
6. Gesture บนแกน (LS/RS) — **LS = วงใน (กลุ่มอักษร), RS = วงนอก (ตัวอักษร) · วงคำสั่งใช้ RS**

> กฎ: event ที่สำคัญกว่าจะ **ตัด** gesture ที่ค้างอยู่ ไม่ต่อยอด

## 3.1 วงคำสั่ง (v0.7)

```
IDLE --D-pad ←--> CMD_WHEEL            (ถ้า RS ยังดันอยู่: armed = false)
CMD_WHEEL --RS ผ่าน 0.40--> CMD_WHEEL + ไฮไลต์ช่อง (armed แล้วเท่านั้น)
CMD_WHEEL --ปล่อย RS < 0.25 ค้าง 60ms--> ทำคำสั่ง + IDLE
CMD_WHEEL --A--> ทำคำสั่งทันที (ถ้ามีช่องที่เลือก) + IDLE
CMD_WHEEL --B / D-pad ←--> IDLE        (ยกเลิก)
CMD_WHEEL --armed = false--> รอจน RS กลับศูนย์ก่อน แล้วจึงรับการเลือก
```

- **LS ไม่มีผลใน `CMD_WHEEL`** (วงในระหว่างพิมพ์ยังใช้ LS ตามปกติ)
- ถ้าทำคำสั่งด้วย A ขณะ LS ยังค้าง → ตั้ง `lock` จนกว่า LS จะกลับศูนย์ (กันเลือกกลุ่มอักษรทันที)

## 3.2 วงภาษา (v0.11)

```
CMD_WHEEL --เลือกช่อง "ภาษาอื่น"--> ถ้ามี 1 ภาษา: สลับทันที
                                  ถ้ามากกว่า 1: LANG_WHEEL
ถ้าไม่มีภาษาที่สอง --> ไม่มีช่องนี้ใน CMD_WHEEL
LANG_WHEEL --RS ผ่าน 0.40--> ไฮไลต์ภาษา (แบ่งช่องเท่ากันตามจำนวนภาษา)
LANG_WHEEL --ปล่อย RS--> เปลี่ยนเป็น type-<lang> + IDLE
LANG_WHEEL --A--> เปลี่ยนภาษาทันที · --B / D-pad ←--> ยกเลิก
LANG_WHEEL --armed = false--> รอ RS กลับศูนย์ (ถ้าเพิ่งใช้ RS เลือกช่องภาษาอื่น)
```

- ภาษาที่ปิดใน Settings จะไม่อยู่ในรายการ (สูงสุด 8) · EN ไม่อยู่ในวงภาษา (ใช้ช่อง ↑)
- ภาษา `ime` ใช้ตาราง QWERTY (physical keys) — ดู [[14-universal-languages]]

## 4. ค่าพารามิเตอร์ทั้งหมด (ค่าเริ่มต้น + เหตุผล)

| พารามิเตอร์ | ค่า | ช่วงปรับ | เหตุผล |
| --- | --- | --- | --- |
| `ls_activate` | 0.35 | 0.25–0.50 | สูงพอที่จะไม่พิมพ์จากการวางนิ้ว |
| `ls_release` | 0.25 | 0.15–0.35 | ต่ำกว่า activate → hysteresis |
| `rs_activate` | 0.40 | 0.30–0.55 | ต้องแม่นกว่า LS |
| `rs_release` | 0.25 | 0.15–0.35 | ต้องกลับชัดเจนเพื่อกันพิมพ์ค้าง |
| `sector_hysteresis` | 9° | 5–14° | กันสั่นตรงเส้นแบ่ง 45° |
| `commit_dwell` | 60 ms | 40–120 | กันการปล่อยสะดุด; มากไป = หน่วง |
| `cancel_dwell` | 600 ms | 400–900 | ค้างเพื่อยกเลิก |
| `repeat_delay` | 350 ms | 250–600 | auto-repeat ตัวแรก |
| `repeat_rate` | 90 ms | 50–150 | auto-repeat ต่อจากนั้น |
| `mode_hold` | 250 ms | 150–400 | สำรองไว้ใช้กับ mode แบบชั่วคราว (v0.3 ใช้ RS เลือก ไม่ต้องกดค้าง) |
| `double_tap` | 280 ms | 200–400 | A (Enter) / Y (Esc) / RB (sticky drag) |
| `trigger_down` / `up` | 0.40 / 0.20 | — | hysteresis ของ RT |
| `poll_hz` | 250 | 125–1000 | latency vs CPU |
| `hud_timeout` | 800 ms | 0–3000 | ซ่อน HUD |
| `deadman` | 3 s | 1–10 s | หยุดรับหลังกดปิดฉุกเฉิน |

## 5. Pseudocode

```csharp
void Tick(PadState p, double dt)
{
    if (state == State.Disabled) return;

    if (EmergencyHeld(p, 1.5)) { Disable(3.0); return; }

    HandleCmdWheel(p);            // D-pad ← = วงคำสั่งวงใน (RS เลือก, ปล่อย = ยืนยัน)
    HandleFaceButtons(p);         // A/B/X/Y priority over axes
    if (state is Mode.Mouse) { HandlePointer(p, dt); return; }
    if (state is Mode.Nav) { HandlePivot(p, dt); return; }   // v0.10: Nav = จัดตำแหน่ง UI

    // --- typing ---
    double lm = Magnitude(p.LS);
    if (state == State.Idle && lm >= P.ls_activate) { EnterGroup(Sector(p.LS)); }
    if (state == State.GroupArmed && lm < P.ls_release) { state = State.Idle; }

    if (state >= State.GroupArmed) {
        // เลื่อน LS เปลี่ยนกลุ่มได้ (ไม่ต้องปล่อย) — ดู ADR-09
        if (lm >= P.ls_activate) {
            var g2 = Sector(p.LS);            // 8 ทิศ + hysteresis 9°
            if (g2 != group) { group = g2; currentSector = -1; state = State.GroupArmed; }
        }

        double rm = Magnitude(p.RS);
        if (rm >= P.rs_activate) {
            var s = Sector(p.RS);              // 8 ทิศ + hysteresis 9°
            if (s != currentSector) { currentSector = s; dwell = 0; }
            state = State.CharSelected;
            dwell += dt;
            if (dwell > P.cancel_dwell) { state = State.Idle; return; }  // ยกเลิก
        }
        else if (rm < P.rs_release && state == State.CharSelected) {
            state = State.Committing; commitTimer = 0;
        }
    }

    if (state == State.Committing) {
        if (Magnitude(p.RS) > P.rs_activate) { state = State.CharSelected; } // เปลี่ยนใจ
        else if ((commitTimer += dt) >= P.commit_dwell) {
            Emit(Map[group][currentSector], Modifiers(p));
            state = State.Idle; StartRepeatWatch(p);
        }
    }
}

// v0.7 — วงคำสั่งวงใน (D-pad ←) · v0.10 เปลี่ยนมาใช้ RS
void HandleCmdWheel(PadState p)
{
    if (DpadLeftTapped(p)) ToggleCmdWheel();          // armed = RS < 0.25 ตอนเปิด

    if (!cmd.open) return;
    if (!cmd.armed) { if (Magnitude(p.RS) < P.rs_release) cmd.armed = true; return; }

    if (Magnitude(p.RS) >= P.rs_activate)
        cmd.idx = Sector(p.RS);                        // LS ไม่ถูกอ่านในวงนี้
    else if (cmd.idx >= 0 && Magnitude(p.RS) < P.rs_release && ++cmd.dwell >= P.commit_dwell)
        RunCommand(cmd.idx);                           // ปล่อย RS = ยืนยัน
}
```

## 6. สิ่งที่ต้องทดสอบกับ state machine นี้

| เคส | ผลที่ต้องได้ |
| --- | --- |
| ดัน LS แล้วปล่อย โดยไม่แตะ RS | ไม่มีคีย์ออก |
| ดัน LS ค้างแล้วเลื่อนไป sector อื่น | เปลี่ยนกลุ่มทันที ไม่มีคีย์ออก |
| เลื่อน LS ระหว่างเลือกตัวอักษร | ยกเลิกตัวที่เลือก กลับไป `GROUP_ARMED` |
| ดัน LS + RS แล้วปล่อย RS ก่อน | พิมพ์ 1 ตัว (ไม่พิมพ์ซ้ำ) |
| ปล่อย LS ก่อน RS | ยกเลิก |
| ค้าง RS 800 ms | ยกเลิก ไม่พิมพ์ |
| ดัน LS ค้าง + RS ทิศเดิมค้างต่อ | repeat ตาม `repeat_delay` / `repeat_rate` |
| สลับ mode กลาง gesture | ยกเลิก + เปลี่ยน mode |
| เปิดวงคำสั่ง (D-pad ←) แล้วดัน LS | ไม่มีช่องถูกเลือก (วงคำสั่งใช้ RS) |
| เปิดวงคำสั่งขณะ RS ค้าง แล้วไม่ปล่อย | ไม่มีคำสั่งทำงาน |
| วงคำสั่ง: เลือกด้วย RS แล้วปล่อย | ทำคำสั่งครั้งเดียว (Copy/Paste/Undo/Redo หรือสลับโหมด) |
| วงคำสั่ง: กด A ขณะ LS ค้าง | ทำคำสั่ง แล้วไม่เลือกกลุ่มอักษรทันที (`lock`) |
| วงภาษา: เลือก RU ด้วย RS | โหมดเป็น `type-ru`, ตัวอักษรมาจาก QWERTY ของ RU |
| มีภาษาที่สองภาษาเดียว | ช่องภาษาอื่นแสดงรหัสภาษานั้น (เช่น TH) และสลับทันที |
| มีภาษาเดียว (EN) | ไม่มีช่องภาษาอื่นในวงคำสั่ง |
| มีภาษาที่สองหลายภาษา | ช่องเป็น `Lang` และเปิดวงภาษา |
| ภาษาที่ใช้ IME | HUD แสดง QWERTY + badge IME, ส่งคีย์ให้ OS IME |
| Nav mode: ดัน RS/LS | pivot ขยับ, pointer/ข้อความไม่เปลี่ยน |
| กด LT (Shift) ครั้งเดียว | ล็อกตัวพิมพ์ใหญ่ + UI แสดง `ตัวใหญ่`; กดอีกครั้ง = ตัวเล็ก |
| RS สั่นตรงเส้นแบ่ง 45° | ไม่สลับตัวอักษรไปมา (hysteresis ทำงาน) |
| จอยหลุดกลาง gesture | ไม่มีคีย์ค้าง (modifier ถูกปล่อย) |
