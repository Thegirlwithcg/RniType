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

/// โหมดหลักของแอป — HUD และ injector ใช้ร่วมกัน
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Disabled,
    Type,
    Mouse,
    Nav,
}

/// ช่องในวงคำสั่ง (8 ช่อง) — ตัวเลขจริงอยู่ใน state machine (P2)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    TypeEn,
    TypeTh,
    Language,
    Mouse,
    Nav,
    PlaceUi,
    Copy,
    Paste,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// สิ่งที่ engine ตัดสินใจแล้ว แต่ยังไม่รู้วิธีส่งออก
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// กดแล้วปล่อยทันที (ตัวอักษร/ปุ่มลัด)
    Key(KeyPress),
    /// กดค้าง (ใช้กับ modifier / auto-repeat)
    KeyDown(KeyPress),
    /// ปล่อยปุ่มที่ค้าง
    KeyUp(KeyPress),
    /// เมาส์แบบ relative (โหมด Mouse)
    MouseMove {
        dx: i32,
        dy: i32,
    },
    /// เมาส์แบบ absolute บน virtual desktop (recenter / ข้ามจอ)
    MouseMoveAbs {
        x: i32,
        y: i32,
    },
    MouseButton {
        button: MouseButton,
        down: bool,
    },
    MouseWheel {
        delta: i32,
    },
    /// คำสั่งจากวงคำสั่ง (Copy/Paste/Undo/Redo/สลับโหมด)
    Command(Command),
    SetMode(Mode),
    /// เปลี่ยนภาษาตาม index ใน `languages[]` ของ config
    SetLang {
        index: u8,
    },
    SetLayer(u8),
    /// Nav mode: ขยับ pivot (CSS px)
    MovePivot {
        dx: f32,
        dy: f32,
    },
    /// ไม่ทำอะไร (ใช้เป็นผลลัพธ์ของ tick ที่ไม่มี action)
    Noop,
}
