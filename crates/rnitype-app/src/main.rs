//! rnitype-app — RniType runtime (Tauri v2 overlay shell)
//!
//! P4 Track C · slice 2 — real Windows overlay
//!
//! - หน้าต่าง Tauri v2: โปร่งใส · ไม่มีขอบ · always-on-top · ไม่ขึ้น taskbar · ไม่แย่ง focus
//! - คลิกทะลุระดับ OS (Spike B parity):
//!   `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`
//!   + `set_ignore_cursor_events(true)` + `SetWindowPos(SWP_FRAMECHANGED)`
//! - ครอบ virtual desktop ทั้งหมด (ทุกจอ) ด้วยพิกเซลจริง
//!   (`GetSystemMetrics(SM_*VIRTUALSCREEN)`)
//! - ส่ง `hud:state` (contract v1) เข้า HUD ที่ `ui/hud`
//!
//! วิธีรัน
//! ```
//! cargo run -p rnitype-app -- --demo --seconds 8
//! ```
//! หรือผ่าน `crates/rnitype-app/run.ps1 -- --demo --seconds 8`
//! (run.ps1 แก้ปัญหา path ที่มี apostrophe ให้ RC.EXE อ่าน icon ได้)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use serde_json::{json, Value};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

/// ตัวอย่าง state จาก contract (คัดลอกไว้ใน crate เพื่อให้ embed ได้)
const DEMO_STATES_JSON: &str = include_str!("../assets/hud-demo-states.json");
/// ระยะห่างระหว่างการ emit `hud:state` ตอน `--demo` (มิลลิวินาที)
const DEMO_INTERVAL_MS: u64 = 3_000;

/// ตัวเลือกจากบรรทัดคำสั่ง
#[derive(Debug, Clone, Copy)]
struct Options {
    /// ส่ง `hud:state` วนจากไฟล์ตัวอย่างทุก ๆ `DEMO_INTERVAL_MS`
    demo: bool,
    /// ปิดแอปเองหลัง N วินาที (สำหรับรันแบบ headless / ตรวจงาน)
    seconds: Option<u64>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            demo: false,
            seconds: None,
        }
    }
}

/// สถานะกลางของแอป (เก็บใน `App` เพื่อให้ command อ่านได้)
#[derive(Default)]
struct AppState {
    /// HUD ส่ง `hud_ready` มาแล้วหรือยัง
    hud_ready: std::sync::atomic::AtomicBool,
    /// แหล่ง state ของโหมด demo (ไม่มี = demo ปิด)
    demo: std::sync::Mutex<Option<DemoSource>>,
}

/// แหล่ง state ของโหมด demo — ใช้ทั้ง thread ที่ส่งวนและ command `hud_ready` ที่ส่งซ้ำ
struct DemoSource {
    handle: tauri::AppHandle,
    states: Vec<Value>,
    /// index ของ state ที่ส่งล่าสุด
    current: usize,
}

fn main() {
    let options = parse_options();
    println!("[app] rnitype-app — RniType overlay (P4 slice 2)");
    println!(
        "[app] options: demo={} seconds={:?}",
        options.demo, options.seconds
    );

    let demo_states = load_demo_states();
    println!(
        "[app] demo states loaded: {} (contract v1)",
        demo_states.len()
    );

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![hud_ready])
        .setup(move |app| setup_overlay(app, &options, &demo_states))
        .run(tauri::generate_context!())
        .expect("error while running rnitype-app");
}

/* ============================================================
   ตัวเลือกบรรทัดคำสั่ง
   ============================================================ */

fn parse_options() -> Options {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => options.demo = true,
            "--seconds" => {
                if let Some(value) = args.next() {
                    match value.parse::<u64>() {
                        Ok(secs) => options.seconds = Some(secs),
                        Err(err) => eprintln!("[app] --seconds อ่านไม่ได้: {value:?} ({err})"),
                    }
                } else {
                    eprintln!("[app] --seconds ต้องมีตัวเลขตามมา");
                }
            }
            other if other.starts_with("--seconds=") => {
                let raw = &other["--seconds=".len()..];
                match raw.parse::<u64>() {
                    Ok(secs) => options.seconds = Some(secs),
                    Err(err) => eprintln!("[app] --seconds อ่านไม่ได้: {raw:?} ({err})"),
                }
            }
            "--help" | "-h" => {
                println!("[app] usage: rnitype-app [--demo] [--seconds N]");
            }
            other => eprintln!("[app] ไม่รู้จักตัวเลือก: {other:?}"),
        }
    }

    options
}

fn load_demo_states() -> Vec<Value> {
    match serde_json::from_str::<Vec<Value>>(DEMO_STATES_JSON) {
        Ok(states) => states,
        Err(err) => {
            eprintln!("[app] อ่าน demo states ไม่ได้: {err}");
            Vec::new()
        }
    }
}

/* ============================================================
   หน้าต่าง overlay
   ============================================================ */

fn setup_overlay(
    app: &tauri::App,
    options: &Options,
    demo_states: &[Value],
) -> Result<(), Box<dyn std::error::Error>> {
    // หน้าต่างสร้างตอน runtime (ไม่ประกาศใน tauri.conf.json) เพื่อคุม geometry เอง
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "hud",
        tauri::WebviewUrl::App("hud/index.html".into()),
    )
    .title("RniType HUD")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .resizable(false)
    .shadow(false)
    .visible(false)
    .build()?;

    // ---- virtual desktop (ทุกจอ) ด้วยพิกเซลจริง ----
    let (vx, vy, vw, vh) = virtual_desktop();
    println!("[overlay] virtual desktop: x={vx} y={vy} w={vw} h={vh}");

    window.set_position(PhysicalPosition::new(vx, vy))?;
    window.set_size(PhysicalSize::new(vw, vh))?;

    // ---- exstyle: คลิกทะลุ + ไม่แย่ง focus (Spike B parity) ----
    // เรียก set_ignore_cursor_events ก่อน เพราะบน Windows มันไปแตะ WS_EX_TRANSPARENT เอง
    window.set_ignore_cursor_events(true)?;
    let exstyle = apply_overlay_exstyle(&window);
    println!(
        "[overlay] exstyle=0x{exstyle:X} (LAYERED={} TRANSPARENT={} NOACTIVATE={} TOOLWINDOW={})",
        has_flag(exstyle, WS_EX_LAYERED),
        has_flag(exstyle, WS_EX_TRANSPARENT),
        has_flag(exstyle, WS_EX_NOACTIVATE),
        has_flag(exstyle, WS_EX_TOOLWINDOW),
    );

    let foreground_before = foreground_window();
    window.show()?;
    let foreground_after = foreground_window();

    let pos = window.outer_position()?;
    let size = window.outer_size()?;
    println!(
        "[overlay] window: pos=({},{}) size={}x{} focused={}",
        pos.x,
        pos.y,
        size.width,
        size.height,
        window.is_focused().unwrap_or(false)
    );
    println!(
        "[overlay] foreground: before={foreground_before:?} after={foreground_after:?} (เท่ากัน = ไม่แย่ง focus)"
    );

    // ---- tray (ทางออกของโปรแกรม เพราะ HUD คลิกทะลุ) ----
    match setup_tray(app) {
        Ok(()) => println!("[tray] tray icon พร้อม (เมนู Quit)"),
        Err(err) => eprintln!("[tray] ตั้ง tray ไม่สำเร็จ: {err} (ใช้ --seconds หรือ Ctrl+C แทน)"),
    }

    // ---- demo: ส่ง hud:state วนทุก 3 วินาที ----
    if options.demo {
        if demo_states.is_empty() {
            eprintln!("[demo] ไม่มี state ตัวอย่าง — ปิด demo");
        } else {
            // เก็บแหล่ง state ไว้ใน AppState เพื่อให้ command `hud_ready` ส่ง state ต่อได้
            // (ipc.md §4: app เริ่มส่ง state หลัง HUD handshake)
            {
                let app_state = app.state::<AppState>();
                let mut guard = app_state.demo.lock().unwrap();
                *guard = Some(DemoSource {
                    handle: app.handle().clone(),
                    states: demo_states.to_vec(),
                    current: 0,
                });
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    {
                        let app_state = handle.state::<AppState>();
                        let mut guard = app_state.demo.lock().unwrap();
                        let Some(source) = guard.as_mut() else {
                            break;
                        };
                        if source.states.is_empty() {
                            break;
                        }
                        let index = source.current % source.states.len();
                        let state = source.states[index].clone();
                        let mode = state.get("mode").and_then(Value::as_str).unwrap_or("?");
                        let st = state.get("state").and_then(Value::as_str).unwrap_or("?");
                        match source.handle.emit_to("hud", "hud:state", &state) {
                            Ok(()) => println!(
                                "[demo] hud:state #{index} → hud (mode={mode} state={st})"
                            ),
                            Err(err) => eprintln!("[demo] ส่ง hud:state ไม่สำเร็จ: {err}"),
                        }
                        source.current = (index + 1) % source.states.len();
                    }
                    std::thread::sleep(Duration::from_millis(DEMO_INTERVAL_MS));
                }
            });
            println!("[demo] เปิด demo: ส่ง hud:state ทุก {DEMO_INTERVAL_MS} ms (เริ่มทันที)");
        }
    } else {
        println!("[demo] demo ปิด — HUD จะไม่ได้รับ state (รอ runtime จริงใน P4 slice 3)");
    }

    // ---- ปิดแอปอัตโนมัติ ----
    if let Some(secs) = options.seconds {
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(secs));
            println!("[app] --seconds {secs} ครบ → ปิดแอป");
            handle.exit(0);
        });
        println!("[app] จะปิดอัตโนมัติใน {secs} วินาที");
    }

    Ok(())
}

/* ============================================================
   Tray
   ============================================================ */

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let quit_item = MenuItem::with_id(app, "quit", "Quit RniType", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let icon = match Image::from_bytes(include_bytes!("../icons/icon.ico")) {
        Ok(image) => image,
        Err(err) => {
            eprintln!("[tray] อ่าน icon.ico ไม่ได้: {err} — ใช้ไอคอนสำรอง 16x16");
            Image::new_owned(fallback_icon_rgba(), 16, 16)
        }
    };

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("RniType")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                println!("[tray] เลือก Quit → ปิดแอป");
                app.exit(0);
            }
        })
        .build(app)?;

    Ok(())
}

/* ============================================================
   Commands (ipc.md §3)
   ============================================================ */

/// HUD แจ้งว่าพร้อมรับ state (ipc.md §3) — log + ส่ง state ให้ HUD ทันทีหลัง handshake
/// (HUD เป็น read-only จึงไม่มี command อื่นที่เปลี่ยน state ของแอป)
#[tauri::command]
fn hud_ready(state: tauri::State<'_, AppState>) -> Value {
    let first = !state
        .hud_ready
        .swap(true, std::sync::atomic::Ordering::SeqCst);
    println!(
        "[hud] hud_ready{} — HUD โหลดแล้วและมี window.__TAURI__ (handshake สำเร็จ)",
        if first { " (ครั้งแรก)" } else { "" }
    );

    // ipc.md §4 — หลัง handshake แอปส่ง state ให้ HUD ทันที (state ถัดไปในคิว)
    {
        let mut guard = state.demo.lock().unwrap();
        if let Some(source) = guard.as_mut() {
            if !source.states.is_empty() {
                let index = source.current % source.states.len();
                let payload = source.states[index].clone();
                let mode = payload.get("mode").and_then(Value::as_str).unwrap_or("?");
                let st = payload.get("state").and_then(Value::as_str).unwrap_or("?");
                match source.handle.emit_to("hud", "hud:state", &payload) {
                    Ok(()) => println!(
                        "[hud] ส่ง hud:state ให้ HUD หลัง handshake (mode={mode} state={st})"
                    ),
                    Err(err) => eprintln!("[hud] ส่ง hud:state หลัง handshake ไม่สำเร็จ: {err}"),
                }
            }
        }
    }

    json!({ "ok": true })
}

/* ============================================================
   Win32
   ============================================================ */

/// ขอบเขต virtual desktop ของ Windows (ทุกจอ) เป็นพิกเซลจริง
#[cfg(windows)]
fn virtual_desktop() -> (i32, i32, u32, u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as u32;
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as u32;
        (x, y, w, h)
    }
}

#[cfg(not(windows))]
fn virtual_desktop() -> (i32, i32, u32, u32) {
    (0, 0, 1920, 1080)
}

/// ตั้ง exstyle ของหน้าต่าง: คลิกทะลุ + ไม่แย่ง focus + ไม่ขึ้น taskbar
/// (โค้ดเดียวกับ Spike B — `spikes/overlay/src-tauri/src/main.rs`)
#[cfg(windows)]
fn apply_overlay_exstyle(window: &tauri::WebviewWindow) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GWL_EXSTYLE, SetWindowLongPtrW, SetWindowPos, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };

    let Ok(hwnd) = window.hwnd() else {
        eprintln!("[overlay] hwnd() ไม่สำเร็จ — ข้ามการตั้ง exstyle");
        return 0;
    };
    let hwnd = hwnd.0 as _;

    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next = ex
            | (WS_EX_LAYERED as isize)
            | (WS_EX_TRANSPARENT as isize)
            | (WS_EX_NOACTIVATE as isize)
            | (WS_EX_TOOLWINDOW as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
    }
}

#[cfg(not(windows))]
fn apply_overlay_exstyle(_window: &tauri::WebviewWindow) -> isize {
    0
}

/// HWND ของหน้าต่างที่อยู่เหนือสุด (ใช้พิสูจน์ว่าไม่แย่ง focus)
#[cfg(windows)]
fn foreground_window() -> Option<isize> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd as isize)
        }
    }
}

#[cfg(not(windows))]
fn foreground_window() -> Option<isize> {
    None
}

/* ---------- ค่าคงที่ WS_EX_* (ใช้ได้ทั้ง Windows / non-Windows) ---------- */

const WS_EX_LAYERED: u32 = 0x0008_0000;
const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;

fn has_flag(exstyle: isize, flag: u32) -> bool {
    (exstyle as u32) & flag != 0
}

/// ไอคอนสำรอง 16×16 (RGBA) — วงกลมสีฟ้า ถ้าอ่าน icon.ico ไม่ได้
fn fallback_icon_rgba() -> Vec<u8> {
    let size = 16usize;
    let mut rgba = Vec::with_capacity(size * size * 4);
    let center = 7.5f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let inside = (dx * dx + dy * dy) <= 49.0;
            if inside {
                rgba.extend_from_slice(&[0x7c, 0xc5, 0xff, 0xff]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    rgba
}
