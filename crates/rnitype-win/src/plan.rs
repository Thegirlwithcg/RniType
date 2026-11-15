//! แปลง `Action` เป็นรายการ [`Input`] (plan) แบบบริสุทธิ์
//!
//! ชั้นนี้ **ไม่เรียก Win32 เลย** → unit test ได้ทุกเครื่อง (ไม่ต้องมี Windows)
//! `SendInputInjector` เป็นผู้เอา plan ไปส่งผ่าน `SendInput`
//!
//! หมายเหตุ: `Input::VirtualKey` เก็บตัวเลข VK ดิบ (ดู [`vk`]) แทน `NamedKey`
//! เพื่อให้ plan เป็นข้อมูลล้วน ไม่ผูกกับ enum ของ core

use rnitype_core::{Action, Key, KeyPress, MouseButton, NamedKey};

/// Virtual-key code ที่ใช้ใน plan — ค่าตรงกับ `VK_*` ของ Win32
///
/// ประกาศเองแทนการดึงจาก `windows-sys` เพื่อให้ layer นี้ test ได้โดยไม่ต้องมี Windows
pub mod vk {
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const ESCAPE: u16 = 0x1B;
    pub const PRIOR: u16 = 0x21; // PageUp
    pub const NEXT: u16 = 0x22; // PageDown
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const SNAPSHOT: u16 = 0x2C;
    pub const SPACE: u16 = 0x20;
    pub const F1: u16 = 0x70;
    pub const F24: u16 = 0x87;
}

/// 1 event ที่จะส่งออก — ยังไม่ผูกกับ `INPUT` ของ Win32
///
/// `up = true`/`down = false` = ปล่อย, ค่าอื่น = กด
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// อักขระ 1 UTF-16 code unit ผ่าน `KEYEVENTF_UNICODE` (`wVk = 0`, `wScan = unit`)
    Unicode {
        unit: u16,
        up: bool,
    },
    /// ปุ่มมีชื่อ/modifier ผ่าน virtual-key
    VirtualKey {
        vk: u16,
        up: bool,
    },
    /// เมาส์ relative (`MOUSEEVENTF_MOVE`)
    MouseMove {
        dx: i32,
        dy: i32,
    },
    /// เมาส์ absolute ที่ normalize เป็น 0..=65535 บน virtual desktop แล้ว
    MouseMoveAbs {
        x: i32,
        y: i32,
    },
    MouseButton {
        button: MouseButton,
        down: bool,
    },
    /// `delta` เป็นหน่วยเดียวกับ `WHEEL_DELTA` (120 = 1 notch, บวก = ขึ้น)
    Wheel {
        delta: i32,
    },
    /// ล้อแนวนอน (บวก = ขวา)
    HWheel {
        delta: i32,
    },
}

impl Input {
    /// input ที่ปล่อยสิ่งที่ input นี้กดอยู่ (คืน `None` กับสิ่งที่ไม่มีสถานะค้าง)
    pub fn released(&self) -> Option<Input> {
        match *self {
            Input::Unicode { unit, .. } => Some(Input::Unicode { unit, up: true }),
            Input::VirtualKey { vk, .. } => Some(Input::VirtualKey { vk, up: true }),
            Input::MouseButton { button, .. } => Some(Input::MouseButton {
                button,
                down: false,
            }),
            Input::MouseMove { .. }
            | Input::MouseMoveAbs { .. }
            | Input::Wheel { .. }
            | Input::HWheel { .. } => None,
        }
    }
}

/// กรอบ virtual desktop (physical px ของ Windows, หลายจอ = กรอบรวม)
///
/// ใช้ normalize เมาส์ absolute; `ScreenRect::virtual_desktop()` (Windows) เป็นตัวสร้างจริง
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRect {
    /// normalize จุด (x, y) เป็น 0..=65535 ของกรอบนี้ (ค่าที่เกินกรอบถูก clamp)
    pub fn normalize(&self, x: i32, y: i32) -> (i32, i32) {
        (
            normalize_axis(x, self.x, self.width),
            normalize_axis(y, self.y, self.height),
        )
    }
}

/// แปลง 1 แกนเป็น 0..=65535 (`size <= 1` = กรอบเล็กเกินไป → 0)
fn normalize_axis(value: i32, origin: i32, size: i32) -> i32 {
    if size <= 1 {
        return 0;
    }
    let last = (size - 1) as i64;
    let offset = i64::from(value.saturating_sub(origin)).clamp(0, last);
    (offset * 65_535 / last) as i32
}

/// แปลง `Action` เป็น plan (0 event = ไม่ต้องส่งอะไรถึง OS)
///
/// - `Key` = กดแล้วปล่อย; ถ้า `ctrl`/`shift` เปิด จะครอบด้วย modifier down..up
/// - `KeyDown`/`KeyUp` = ค้าง/ปล่อยจริง (modifier ที่เปิดจะค้างตามไปด้วย)
/// - `Char` ส่ง Unicode path เสมอ: **Shift ไม่ทำให้ Unicode เป็นตัวใหญ่**
///   (runtime ส่งตัวใหญ่มาอยู่แล้วด้วย `shift_lock`; shift ที่ครอบจึงมีผลกับ hotkey
///   ของแอปปลายทางเท่านั้น)
/// - `Command`/`SetMode`/`SetLang`/`SetLayer`/`MovePivot`/`Noop` = แอปจัดการเอง → plan ว่าง
/// - `NamedKey::F(n)` ที่ n ไม่อยู่ใน 1..=24 = plan ว่าง (ดู [`vk_for`])
pub fn plan(action: Action, screen: ScreenRect) -> Vec<Input> {
    let mut out = Vec::with_capacity(4);
    match action {
        Action::Key(press) => {
            push_modifiers(&mut out, press, true);
            push_tap_key(&mut out, press);
            push_modifiers(&mut out, press, false);
        }
        Action::KeyDown(press) => {
            push_modifiers(&mut out, press, true);
            push_held_key(&mut out, press, true);
        }
        Action::KeyUp(press) => {
            push_held_key(&mut out, press, false);
            push_modifiers(&mut out, press, false);
        }
        Action::MouseMove { dx, dy } => out.push(Input::MouseMove { dx, dy }),
        Action::MouseMoveAbs { x, y } => {
            let (x, y) = screen.normalize(x, y);
            out.push(Input::MouseMoveAbs { x, y });
        }
        Action::MouseButton { button, down } => out.push(Input::MouseButton { button, down }),
        Action::MouseWheel { delta } => out.push(Input::Wheel { delta }),
        Action::MouseHWheel { delta } => out.push(Input::HWheel { delta }),
        Action::Command(_)
        | Action::SetMode(_)
        | Action::SetLang { .. }
        | Action::SetLayer(_)
        | Action::MovePivot { .. }
        | Action::RecenterPointer
        | Action::ResetPivot
        | Action::Noop => {}
    }
    out
}

/// modifier ที่เปิดใน `KeyPress` — กดตามลำดับ ctrl→shift, ปล่อยย้อนลำดับ
fn push_modifiers(out: &mut Vec<Input>, press: KeyPress, down: bool) {
    let modifiers = [(press.ctrl, vk::CONTROL), (press.shift, vk::SHIFT)];
    if down {
        for (enabled, code) in modifiers {
            if enabled {
                out.push(Input::VirtualKey {
                    vk: code,
                    up: false,
                });
            }
        }
    } else {
        for (enabled, code) in modifiers.into_iter().rev() {
            if enabled {
                out.push(Input::VirtualKey { vk: code, up: true });
            }
        }
    }
}

/// ปุ่มเดียวแบบกดแล้วปล่อย (Char = ทีละ UTF-16 unit, down+up ติดกันเหมือน Spike C)
///
/// **Ctrl + ตัวอักษร ASCII = ส่ง VirtualKey ไม่ใช่ Unicode** — แอปปลายทางอ่าน
/// `Ctrl+VK_C` เป็นคีย์ลัด (Copy/Paste/Undo/Redo) แต่ไม่ตีความ `Ctrl+Unicode`
fn push_tap_key(out: &mut Vec<Input>, press: KeyPress) {
    if press.ctrl
        && let Key::Char(ch) = press.key
        && let Some(vk) = vk_for_ctrl_char(ch)
    {
        out.push(Input::VirtualKey { vk, up: false });
        out.push(Input::VirtualKey { vk, up: true });
        return;
    }
    push_tap_key_unicode(out, press.key);
}

fn push_tap_key_unicode(out: &mut Vec<Input>, key: Key) {
    match key {
        Key::Char(ch) => {
            let (units, len) = utf16_units(ch);
            for &unit in &units[..len] {
                out.push(Input::Unicode { unit, up: false });
                out.push(Input::Unicode { unit, up: true });
            }
        }
        Key::Named(named) => {
            if let Some(vk) = vk_for(named) {
                out.push(Input::VirtualKey { vk, up: false });
                out.push(Input::VirtualKey { vk, up: true });
            }
        }
    }
}

/// VK ของตัวอักษร ASCII ที่ใช้คู่กับ Ctrl (A–Z, 0–9) — `None` = ใช้ Unicode path
pub fn vk_for_ctrl_char(ch: char) -> Option<u16> {
    match ch {
        'a'..='z' => Some(u16::from(b'A' + (ch as u8 - b'a'))),
        'A'..='Z' => Some(u16::from(ch as u8)),
        '0'..='9' => Some(u16::from(b'0' + (ch as u8 - b'0'))),
        _ => None,
    }
}

/// ปุ่มเดียวแบบค้าง/ปล่อย (ใช้กับ `Action::KeyDown`/`KeyUp`)
fn push_held_key(out: &mut Vec<Input>, press: KeyPress, down: bool) {
    if press.ctrl
        && let Key::Char(ch) = press.key
        && let Some(vk) = vk_for_ctrl_char(ch)
    {
        out.push(Input::VirtualKey { vk, up: !down });
        return;
    }

    match press.key {
        Key::Char(ch) => {
            let (units, len) = utf16_units(ch);
            for &unit in &units[..len] {
                out.push(Input::Unicode { unit, up: !down });
            }
        }
        Key::Named(named) => {
            if let Some(vk) = vk_for(named) {
                out.push(Input::VirtualKey { vk, up: !down });
            }
        }
    }
}

/// UTF-16 code units ของ `char` (1–2 unit) — คำนวณเองแทน `char::encode_utf16`
/// เพื่อให้ plan test ได้เหมือนกันทุก toolchain
fn utf16_units(ch: char) -> ([u16; 2], usize) {
    let code = ch as u32;
    if code <= 0xFFFF {
        ([code as u16, 0], 1)
    } else {
        let rest = code - 0x1_0000;
        (
            [0xD800 | (rest >> 10) as u16, 0xDC00 | (rest & 0x3FF) as u16],
            2,
        )
    }
}

/// VK ของปุ่มมีชื่อ — `None` = ค่า F ที่อยู่นอกช่วง 1..=24
pub fn vk_for(named: NamedKey) -> Option<u16> {
    let code = match named {
        NamedKey::Space => vk::SPACE,
        NamedKey::Enter => vk::RETURN,
        NamedKey::Backspace => vk::BACK,
        NamedKey::Tab => vk::TAB,
        NamedKey::Escape => vk::ESCAPE,
        NamedKey::Delete => vk::DELETE,
        NamedKey::Insert => vk::INSERT,
        NamedKey::Home => vk::HOME,
        NamedKey::End => vk::END,
        NamedKey::PageUp => vk::PRIOR,
        NamedKey::PageDown => vk::NEXT,
        NamedKey::Up => vk::UP,
        NamedKey::Down => vk::DOWN,
        NamedKey::Left => vk::LEFT,
        NamedKey::Right => vk::RIGHT,
        NamedKey::PrintScreen => vk::SNAPSHOT,
        NamedKey::F(n) if (1..=24).contains(&n) => vk::F1 + u16::from(n - 1),
        NamedKey::F(_) => return None,
    };
    Some(code)
}

/// สิ่งที่ injector กดค้างอยู่ — ให้ `release_all()` ปล่อยครบตอนจอยหลุด/ปิดแอป
///
/// เก็บเฉพาะ event ที่ "กด" (ไม่เก็บ wheel/การขยับ) และไม่เก็บซ้ำ
/// (auto-repeat ของ OS อยู่นอกขอบเขต; เราส่ง down ครั้งเดียวต่อปุ่ม)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldState {
    keys: Vec<Input>,
    buttons: Vec<MouseButton>,
}

impl HeldState {
    /// อัปเดตสถานะจาก plan ที่ส่งสำเร็จแล้ว
    pub(crate) fn track(&mut self, plan: &[Input]) {
        for input in plan {
            match *input {
                Input::Unicode { up, .. } | Input::VirtualKey { up, .. } => {
                    if up {
                        if let Some(pos) = self.keys.iter().rposition(|held| same_key(held, input))
                        {
                            self.keys.remove(pos);
                        }
                    } else if !self.keys.iter().any(|held| same_key(held, input)) {
                        self.keys.push(*input);
                    }
                }
                Input::MouseButton { button, down } => {
                    if down {
                        if !self.buttons.contains(&button) {
                            self.buttons.push(button);
                        }
                    } else {
                        self.buttons.retain(|held| *held != button);
                    }
                }
                Input::MouseMove { .. }
                | Input::MouseMoveAbs { .. }
                | Input::Wheel { .. }
                | Input::HWheel { .. } => {}
            }
        }
    }

    /// plan สำหรับปล่อยทุกอย่างที่ค้าง (ย้อนลำดับการกด)
    pub fn release_plan(&self) -> Vec<Input> {
        let mut out: Vec<Input> = self.keys.iter().rev().filter_map(Input::released).collect();
        out.extend(self.buttons.iter().rev().map(|button| Input::MouseButton {
            button: *button,
            down: false,
        }));
        out
    }

    /// ไม่มีอะไรค้างอยู่
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.buttons.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.keys.clear();
        self.buttons.clear();
    }
}

/// เทียบว่าเป็นปุ่มเดียวกันหรือไม่ (ไม่สน `up`)
fn same_key(a: &Input, b: &Input) -> bool {
    match (a, b) {
        (Input::Unicode { unit: a, .. }, Input::Unicode { unit: b, .. }) => a == b,
        (Input::VirtualKey { vk: a, .. }, Input::VirtualKey { vk: b, .. }) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnitype_core::{Command, Mode};

    fn full_hd() -> ScreenRect {
        ScreenRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    fn tap(key: Key, shift: bool, ctrl: bool) -> Action {
        Action::Key(KeyPress { key, shift, ctrl })
    }

    #[test]
    fn char_uses_unicode_down_then_up() {
        let plan = plan(tap(Key::Char('a'), false, false), full_hd());
        assert_eq!(
            plan,
            vec![
                Input::Unicode {
                    unit: 0x0061,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0061,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn thai_char_goes_through_unicode_not_vk() {
        let plan = plan(tap(Key::Char('ก'), false, false), full_hd());
        assert_eq!(
            plan,
            vec![
                Input::Unicode {
                    unit: 0x0E01,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0E01,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn non_bmp_char_emits_one_pair_per_utf16_unit() {
        // Spike C พิสูจน์เฉพาะ BMP — พฤติกรรมนี้คือ "เหมือน Spike C ทีละ unit" ยังไม่ยืนยันกับ OS
        let plan = plan(tap(Key::Char('😀'), false, false), full_hd());
        let (units, len) = utf16_units('😀');
        assert_eq!(len, 2);
        assert_eq!(
            plan,
            vec![
                Input::Unicode {
                    unit: units[0],
                    up: false
                },
                Input::Unicode {
                    unit: units[0],
                    up: true
                },
                Input::Unicode {
                    unit: units[1],
                    up: false
                },
                Input::Unicode {
                    unit: units[1],
                    up: true
                },
            ]
        );
    }

    #[test]
    fn named_keys_map_to_documented_vk_codes() {
        let cases = [
            (NamedKey::Space, 0x20),
            (NamedKey::Enter, 0x0D),
            (NamedKey::Backspace, 0x08),
            (NamedKey::Tab, 0x09),
            (NamedKey::Escape, 0x1B),
            (NamedKey::Delete, 0x2E),
            (NamedKey::Insert, 0x2D),
            (NamedKey::Home, 0x24),
            (NamedKey::End, 0x23),
            (NamedKey::PageUp, 0x21),
            (NamedKey::PageDown, 0x22),
            (NamedKey::Up, 0x26),
            (NamedKey::Down, 0x28),
            (NamedKey::Left, 0x25),
            (NamedKey::Right, 0x27),
            (NamedKey::PrintScreen, 0x2C),
            (NamedKey::F(1), 0x70),
            (NamedKey::F(12), 0x7B),
            (NamedKey::F(24), 0x87),
        ];
        for (named, expected) in cases {
            assert_eq!(vk_for(named), Some(expected), "{named:?}");
            assert_eq!(
                plan(tap(Key::Named(named), false, false), full_hd()),
                vec![
                    Input::VirtualKey {
                        vk: expected,
                        up: false
                    },
                    Input::VirtualKey {
                        vk: expected,
                        up: true
                    },
                ],
                "{named:?}"
            );
        }
    }

    #[test]
    fn f_keys_outside_1_to_24_produce_no_input() {
        assert_eq!(vk_for(NamedKey::F(0)), None);
        assert_eq!(vk_for(NamedKey::F(25)), None);
        assert!(plan(tap(Key::Named(NamedKey::F(0)), false, false), full_hd()).is_empty());
        assert!(plan(tap(Key::Named(NamedKey::F(25)), false, false), full_hd()).is_empty());
    }

    #[test]
    fn ctrl_wraps_tap() {
        let plan = plan(tap(Key::Char('c'), false, true), full_hd());
        assert_eq!(
            plan,
            vec![
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: false
                },
                // Ctrl + ตัวอักษร ASCII = VK (แอปปลายทางเห็นเป็น Ctrl+C)
                Input::VirtualKey {
                    vk: 0x43,
                    up: false
                },
                Input::VirtualKey { vk: 0x43, up: true },
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn ctrl_with_non_ascii_stays_on_the_unicode_path() {
        // ไทยไม่มี VK → ส่ง Unicode เหมือนเดิม (Ctrl ไม่ทำให้เพี้ยน)
        let plan = plan(tap(Key::Char('ก'), false, true), full_hd());
        assert_eq!(
            plan,
            vec![
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0E01,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0E01,
                    up: true
                },
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn vk_for_ctrl_char_covers_letters_and_digits() {
        assert_eq!(vk_for_ctrl_char('v'), Some(0x56));
        assert_eq!(vk_for_ctrl_char('Z'), Some(0x5A));
        assert_eq!(vk_for_ctrl_char('7'), Some(0x37));
        assert_eq!(vk_for_ctrl_char('/'), None);
    }

    #[test]
    fn shift_wraps_named_key_but_not_unicode_letter_case() {
        let named_plan = plan(tap(Key::Named(NamedKey::Tab), true, false), full_hd());
        assert_eq!(
            named_plan,
            vec![
                Input::VirtualKey {
                    vk: vk::SHIFT,
                    up: false
                },
                Input::VirtualKey {
                    vk: vk::TAB,
                    up: false
                },
                Input::VirtualKey {
                    vk: vk::TAB,
                    up: true
                },
                Input::VirtualKey {
                    vk: vk::SHIFT,
                    up: true
                },
            ]
        );

        // Char ยังเป็นตัวเดิม — shift_lock ของ runtime ส่ง 'A' มาเองแล้ว
        let char_plan = plan(tap(Key::Char('A'), true, false), full_hd());
        assert_eq!(
            char_plan,
            vec![
                Input::VirtualKey {
                    vk: vk::SHIFT,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0041,
                    up: false
                },
                Input::Unicode {
                    unit: 0x0041,
                    up: true
                },
                Input::VirtualKey {
                    vk: vk::SHIFT,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn ctrl_and_shift_press_in_order_and_release_in_reverse() {
        let plan = plan(tap(Key::Named(NamedKey::Enter), true, true), full_hd());
        let codes: Vec<u16> = plan
            .iter()
            .map(|input| match input {
                Input::VirtualKey { vk, .. } => *vk,
                other => panic!("expected VirtualKey, got {other:?}"),
            })
            .collect();
        assert_eq!(
            codes,
            vec![
                vk::CONTROL,
                vk::SHIFT,
                vk::RETURN,
                vk::RETURN,
                vk::SHIFT,
                vk::CONTROL
            ]
        );
        assert_eq!(plan[0].released(), Some(plan[5]));
        assert_eq!(plan[1].released(), Some(plan[4]));
    }

    #[test]
    fn key_down_holds_modifiers_and_key() {
        let plan = plan(
            Action::KeyDown(KeyPress {
                key: Key::Char('v'),
                shift: false,
                ctrl: true,
            }),
            full_hd(),
        );
        assert_eq!(
            plan,
            vec![
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: false
                },
                Input::VirtualKey {
                    vk: 0x56,
                    up: false
                },
            ]
        );
    }

    #[test]
    fn key_up_releases_key_then_modifiers() {
        let plan = plan(
            Action::KeyUp(KeyPress {
                key: Key::Char('v'),
                shift: false,
                ctrl: true,
            }),
            full_hd(),
        );
        assert_eq!(
            plan,
            vec![
                Input::VirtualKey { vk: 0x56, up: true },
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: true
                },
            ]
        );
    }

    #[test]
    fn key_down_then_key_up_for_named_key() {
        let press = KeyPress {
            key: Key::Named(NamedKey::Space),
            shift: false,
            ctrl: false,
        };
        assert_eq!(
            plan(Action::KeyDown(press), full_hd()),
            vec![Input::VirtualKey {
                vk: vk::SPACE,
                up: false
            }]
        );
        assert_eq!(
            plan(Action::KeyUp(press), full_hd()),
            vec![Input::VirtualKey {
                vk: vk::SPACE,
                up: true
            }]
        );
    }

    #[test]
    fn mouse_move_is_relative_and_keeps_sign() {
        assert_eq!(
            plan(Action::MouseMove { dx: 3, dy: -7 }, full_hd()),
            vec![Input::MouseMove { dx: 3, dy: -7 }]
        );
    }

    #[test]
    fn mouse_move_abs_normalizes_over_virtual_desktop() {
        let screen = ScreenRect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        assert_eq!(
            plan(Action::MouseMoveAbs { x: 0, y: 0 }, screen),
            vec![Input::MouseMoveAbs { x: 0, y: 0 }]
        );
        assert_eq!(
            plan(Action::MouseMoveAbs { x: 99, y: 99 }, screen),
            vec![Input::MouseMoveAbs {
                x: 65_535,
                y: 65_535
            }]
        );
        assert_eq!(
            plan(Action::MouseMoveAbs { x: 50, y: 50 }, screen),
            vec![Input::MouseMoveAbs {
                x: 33_098,
                y: 33_098
            }]
        );
    }

    #[test]
    fn mouse_move_abs_clamps_points_outside_virtual_desktop() {
        let screen = ScreenRect {
            x: -1920,
            y: 0,
            width: 3840,
            height: 1080,
        };
        assert_eq!(screen.normalize(-5000, -50), (0, 0));
        assert_eq!(screen.normalize(5000, 5000), (65_535, 65_535));
        assert_eq!(screen.normalize(-1920, 0), (0, 0));
    }

    #[test]
    fn mouse_move_abs_handles_degenerate_rect() {
        let screen = ScreenRect {
            x: 10,
            y: 20,
            width: 1,
            height: 0,
        };
        assert_eq!(screen.normalize(10, 20), (0, 0));
    }

    #[test]
    fn mouse_buttons_are_passed_through() {
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            assert_eq!(
                plan(Action::MouseButton { button, down: true }, full_hd()),
                vec![Input::MouseButton { button, down: true }]
            );
            assert_eq!(
                plan(
                    Action::MouseButton {
                        button,
                        down: false
                    },
                    full_hd()
                ),
                vec![Input::MouseButton {
                    button,
                    down: false
                }]
            );
        }
    }

    #[test]
    fn mouse_wheel_is_passed_through() {
        assert_eq!(
            plan(Action::MouseWheel { delta: -120 }, full_hd()),
            vec![Input::Wheel { delta: -120 }]
        );
        assert_eq!(
            plan(Action::MouseHWheel { delta: 120 }, full_hd()),
            vec![Input::HWheel { delta: 120 }]
        );
    }

    #[test]
    fn app_actions_never_reach_the_os() {
        let actions = [
            Action::Command(Command::Copy),
            Action::Command(Command::Paste),
            Action::Command(Command::Undo),
            Action::Command(Command::Redo),
            Action::Command(Command::TypeEn),
            Action::Command(Command::TypeTh),
            Action::Command(Command::Language),
            Action::Command(Command::Mouse),
            Action::Command(Command::Nav),
            Action::Command(Command::PlaceUi),
            Action::SetMode(Mode::Type),
            Action::SetLang { index: 1 },
            Action::SetLayer(2),
            Action::MovePivot { dx: 12.5, dy: -4.0 },
            Action::RecenterPointer,
            Action::ResetPivot,
            Action::Noop,
        ];
        for action in actions {
            assert!(plan(action, full_hd()).is_empty(), "{action:?}");
        }
    }

    #[test]
    fn tap_leaves_no_held_state() {
        let mut held = HeldState::default();
        let plan = plan(tap(Key::Char('a'), true, true), full_hd());
        held.track(&plan);
        assert!(held.is_empty());
        assert!(held.release_plan().is_empty());
    }

    #[test]
    fn partially_sent_plan_tracks_only_the_inserted_prefix() {
        // SendInput รับไปแค่ event แรก (keydown) — injector ต้องจำว่าคีย์ยังค้าง
        let plan = plan(tap(Key::Char('a'), false, false), full_hd());
        assert_eq!(plan.len(), 2);

        let mut held = HeldState::default();
        held.track(&plan[..1]);
        assert!(!held.is_empty());
        assert_eq!(
            held.release_plan(),
            vec![Input::Unicode {
                unit: 0x0061,
                up: true
            }]
        );
    }

    #[test]
    fn key_down_is_tracked_and_released() {
        let key = Key::Char('v');
        let mut held = HeldState::default();
        held.track(&plan(
            Action::KeyDown(KeyPress {
                key,
                shift: false,
                ctrl: true,
            }),
            full_hd(),
        ));
        assert!(!held.is_empty());
        assert_eq!(
            held.release_plan(),
            vec![
                Input::VirtualKey { vk: 0x56, up: true },
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: true
                }
            ]
        );
    }

    #[test]
    fn key_up_clears_tracked_state() {
        let press = KeyPress {
            key: Key::Named(NamedKey::Space),
            shift: true,
            ctrl: false,
        };
        let mut held = HeldState::default();
        held.track(&plan(Action::KeyDown(press), full_hd()));
        held.track(&plan(Action::KeyUp(press), full_hd()));
        assert!(held.is_empty());
    }

    #[test]
    fn duplicate_down_for_same_key_is_tracked_once() {
        let press = Action::KeyDown(KeyPress {
            key: Key::Char('a'),
            shift: false,
            ctrl: false,
        });
        let mut held = HeldState::default();
        held.track(&plan(press, full_hd()));
        held.track(&plan(press, full_hd()));
        assert_eq!(held.release_plan().len(), 1);
    }

    #[test]
    fn release_plan_goes_backwards_through_keys_then_buttons() {
        let mut held = HeldState::default();
        held.track(&[
            Input::VirtualKey {
                vk: vk::CONTROL,
                up: false,
            },
            Input::MouseButton {
                button: MouseButton::Left,
                down: true,
            },
            Input::Unicode {
                unit: 0x0061,
                up: false,
            },
        ]);
        held.track(&[
            Input::Wheel { delta: 120 },
            Input::MouseMove { dx: 5, dy: 5 },
        ]);
        assert_eq!(
            held.release_plan(),
            vec![
                Input::Unicode {
                    unit: 0x0061,
                    up: true
                },
                Input::VirtualKey {
                    vk: vk::CONTROL,
                    up: true
                },
                Input::MouseButton {
                    button: MouseButton::Left,
                    down: false
                },
            ]
        );
    }

    #[test]
    fn releasing_unknown_key_or_button_is_harmless() {
        let mut held = HeldState::default();
        held.track(&[Input::MouseButton {
            button: MouseButton::Right,
            down: false,
        }]);
        assert!(held.is_empty());
    }

    #[test]
    fn clear_forgets_everything() {
        let mut held = HeldState::default();
        held.track(&[Input::VirtualKey {
            vk: vk::SHIFT,
            up: false,
        }]);
        held.clear();
        assert!(held.is_empty());
    }
}
