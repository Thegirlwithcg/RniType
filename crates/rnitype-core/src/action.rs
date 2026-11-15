//! Action = สิ่งที่ engine ตัดสินใจแล้ว แต่ยังไม่รู้วิธีส่งออก

use std::fmt;

/// ปุ่มที่มีชื่อ (ชั้น 2 / แป้นควบคุม) — ใช้ต่อใน M1
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
    Space,
    Enter,
    Backspace,
    Tab,
    Escape,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

/// คีย์เดียวที่ต้องส่ง (ตัวอักษร หรือปุ่มมีชื่อ)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Named(NamedKey),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{c}"),
            Key::Named(n) => write!(f, "{n:?}"),
        }
    }
}

/// คำสั่งส่งออก 1 ครั้ง (injector เป็นผู้แปลงเป็น SendInput)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub shift: bool,
    pub ctrl: bool,
}

/// ยังมีแค่ปุ่ม — mouse/HUD จะเพิ่มใน M2–M3
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Key(KeyPress),
}
