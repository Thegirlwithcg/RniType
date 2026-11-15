//! Spike B — Tauri v2 overlay
//!
//! รัน: cargo run --manifest-path spikes/overlay/src-tauri/Cargo.toml
//! ทดสอบ: โปร่งใส · always-on-top · click-through 5 วิ · ไม่แย่ง focus · ย้าย 2 จอ
//!
//! ผ่านเมื่อ:
//! 1) เห็น panel ลอยเหนือหน้าต่างอื่น พื้นหลังรอบ panel โปร่งใส
//! 2) กด "ทดสอบคลิกทะลุ" แล้วคลิก/พิมพ์ใน Notepad ด้านหลังได้
//! 3) ระหว่างคลิกทะลุ Notepad ยังเป็น foreground (title bar ไม่เปลี่ยน)
//! 4) ย้ายไปจอ 2 แล้ว panel ยังอยู่ถูกตำแหน่ง (DPI 150%/200%)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, PhysicalPosition, PhysicalSize};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            list_monitors,
            move_to_monitor,
            quit
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");
            #[cfg(windows)]
            unsafe {
                make_no_activate(&window);
            }
            println!("Spike B: overlay started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running spike overlay");
}

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, on: bool) -> Result<(), String> {
    window.set_ignore_cursor_events(on).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_monitors(window: tauri::WebviewWindow) -> Result<Vec<String>, String> {
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                "{}: {}×{} @ {},{} · scale {:.2}",
                i,
                m.size().width,
                m.size().height,
                m.position().x,
                m.position().y,
                m.scale_factor()
            )
        })
        .collect())
}

#[tauri::command]
fn move_to_monitor(window: tauri::WebviewWindow, index: usize) -> Result<String, String> {
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let monitor = monitors.get(index).ok_or("ไม่พบจอนี้")?;
    let pos = monitor.position();
    window
        .set_position(PhysicalPosition::new(pos.x + 80, pos.y + 80))
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(720, 420))
        .map_err(|e| e.to_string())?;
    Ok(format!("ย้ายไปจอ {index} · scale {:.2}", monitor.scale_factor()))
}

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg(windows)]
unsafe fn make_no_activate(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    let Ok(hwnd) = window.hwnd() else {
        eprintln!("Spike B: hwnd() ไม่สำเร็จ — ข้าม WS_EX_NOACTIVATE");
        return;
    };
    let hwnd = hwnd.0 as _;
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let next = ex | (WS_EX_NOACTIVATE as isize) | (WS_EX_TOOLWINDOW as isize);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
}
