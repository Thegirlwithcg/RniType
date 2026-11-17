//! ตำแหน่ง caret ของหน้าต่างที่ focus อยู่ (พิกัดหน้าจอ)
//!
//! ใช้กับ Fast mode (ticket 02 · Feature 2): HUD ไปเกาะตำแหน่งที่ผู้ใช้กำลังจะพิมพ์
//! โดยไม่ต้องมีจุด/mาร์กเกอร์บนจอ
//!
//! - [`caret_point`] = FFI ของ Windows (`GetGUIThreadInfo` + `ClientToScreen`)
//! - คืน `None` เมื่อไม่มี caret (แอปที่ไม่ประกาศ caret, หน้าต่างไม่ใช่ text, ฯลฯ)
//!   → ผู้เรียกต้องคง pivot เดิมไว้ ไม่ใช่ย้ายไปมุมจอ

/// ตำแหน่ง caret (มุมซ้ายล่างของ caret) ในพิกัดหน้าจอ
///
/// `None` = ไม่มี caret ให้เกาะ — ห้ามใช้ค่าเดาแทน
#[cfg(windows)]
pub fn caret_point() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GUITHREADINFO, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    };

    // SAFETY: ทุกพอยน์เตอร์ชี้ไปยัง struct/ตัวแปรบนสแต็กที่ยังมีชีวิตตลอดการเรียก
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        if thread == 0 {
            return None;
        }
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(thread, &mut info) == 0 || info.hwndCaret.is_null() {
            return None;
        }
        // rcCaret เป็นพิกัด client ของ hwndCaret → แปลงเป็นพิกัดหน้าจอ
        let mut point = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.bottom,
        };
        if ClientToScreen(info.hwndCaret, &mut point) == 0 {
            return None;
        }
        Some((point.x as f32, point.y as f32))
    }
}

/// ที่อื่นไม่มี caret ของระบบ → `None`
#[cfg(not(windows))]
pub fn caret_point() -> Option<(f32, f32)> {
    None
}
