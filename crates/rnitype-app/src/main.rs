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
//! หรือผ่าน `crates/rnitype-app/run.ps1 --demo --seconds 8`
//! (run.ps1 แก้ปัญหา path ที่มี apostrophe ให้ RC.EXE อ่าน icon ได้)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod input;
mod session;
mod stats;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

use input::{GilrsSource, PadSource, ScriptSource};
use rnitype_core::pad::PadState;
use rnitype_core::runtime::RuntimeConfig;
use rnitype_core::keymap::{group_names_for, label_of, lookup_layer};
use rnitype_core::normalizer::Dir8;
use rnitype_core::{Lang, RuntimeEngine};
use rnitype_win::Injector;
use session::{HudStyle, Session};

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
    /// เปิดหน้าต่าง Settings ตั้งแต่เริ่ม (ใช้ตรวจ UI / ถ่าย screenshot)
    settings: bool,
    /// ใช้จอยปลอม (สคริปต์) แทนจอยจริง — ตรวจ runtime โดยไม่ต้องมีจอย
    fake_pad: bool,
    /// log ค่าจอยทุก 250 ms (ใช้หาสาเหตุว่าจอยส่งค่าอะไรเข้ามา)
    pad_log: bool,
    /// บังคับ Unicode path (ปิด scancode auto) — ใช้เมื่อเกม/IME มีปัญหา
    no_scancode: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            demo: false,
            seconds: None,
            settings: false,
            fake_pad: false,
            pad_log: false,
            no_scancode: false,
        }
    }
}

/// สถานะกลางของแอป (เก็บใน `App` เพื่อให้ command อ่านได้)
struct AppState {
    /// HUD ส่ง `hud_ready` มาแล้วหรือยัง
    hud_ready: std::sync::atomic::AtomicBool,
    /// แหล่ง state ของโหมด demo (ไม่มี = demo ปิด)
    demo: std::sync::Mutex<Option<DemoSource>>,
    /// config ทั้งไฟล์ (ตอนนี้แอปใช้ `ui.*` — typography ของ HUD)
    config: std::sync::Mutex<Value>,
    /// origin ของหน้าต่าง overlay บน virtual desktop (ไว้แปลงพิกัดหน้าจอ → พิกัดหน้าต่าง)
    origin: std::sync::Mutex<(i32, i32)>,
    /// ผู้ใช้สั่งพักการพิมพ์จาก tray (จอยยังอ่านอยู่ แต่ไม่ส่งคีย์)
    paused: AtomicBool,
    /// HUD เพิ่ง handshake/รีโหลด → รอบถัดไปต้องส่ง state ซ้ำ แม้ payload ไม่เปลี่ยน
    hud_need_resend: AtomicBool,
    /// เพิ่มทุกครั้งที่ config เปลี่ยน — runtime loop ใช้ reload ค่าที่ต้องมีผลสด
    config_rev: std::sync::atomic::AtomicU64,
    /// ชื่อจอยที่ต่ออยู่ (ว่าง = ไม่มี) — สำหรับ General tab
    device: std::sync::Mutex<String>,
    /// มีจอยต่ออยู่หรือไม่ (ค่าล่าสุดที่ runtime เห็น)
    connected: AtomicBool,
    /// ตัวเลขล่าสุดของ session (chars/WPM/action latency) — Settings อ่านผ่าน `get_stats`
    stats: std::sync::Mutex<Value>,
}

impl AppState {
    fn new(config: Value) -> Self {
        Self {
            hud_ready: std::sync::atomic::AtomicBool::new(false),
            demo: std::sync::Mutex::new(None),
            config: std::sync::Mutex::new(config),
            origin: std::sync::Mutex::new((0, 0)),
            paused: AtomicBool::new(false),
            hud_need_resend: AtomicBool::new(false),
            config_rev: std::sync::atomic::AtomicU64::new(0),
            device: std::sync::Mutex::new(String::new()),
            connected: AtomicBool::new(false),
            stats: std::sync::Mutex::new(Value::Null),
        }
    }

    /// bump revision → loop รอบถัดไปจะ reload languages/pivot จาก config
    fn touch_config(&self) {
        self.config_rev.fetch_add(1, Ordering::Relaxed);
    }
}

/// แหล่ง state ของโหมด demo — ใช้ทั้ง thread ที่ส่งวนและ command `hud_ready` ที่ส่งซ้ำ
struct DemoSource {
    handle: tauri::AppHandle,
    states: Vec<Value>,
    /// index ของ state ที่ส่งล่าสุด
    current: usize,
}

fn main() {
    if already_running() {
        eprintln!("[app] RniType รันอยู่แล้ว — ปิดตัวเก่าก่อน (ไอคอนในถาดระบบ → Quit)");
        std::process::exit(1);
    }

    let options = parse_options();
    println!("[app] rnitype-app — RniType overlay (P4 slice 2)");
    println!(
        "[app] options: demo={} seconds={:?} settings={} fake_pad={} pad_log={}",
        options.demo, options.seconds, options.settings, options.fake_pad, options.pad_log
    );

    let demo_states = load_demo_states();
    println!(
        "[app] demo states loaded: {} (contract v1)",
        demo_states.len()
    );

    let config = config::load();
    println!(
        "[app] ui style: {}",
        serde_json::to_string(&config::ui_style(&config)).unwrap_or_default()
    );

    tauri::Builder::default()
        .manage(AppState::new(config))
        .invoke_handler(tauri::generate_handler![
            hud_ready,
            hud_style,
            get_config,
            save_config,
            get_status,
            get_stats,
            set_paused,
            rescan_languages,
            set_language_enabled,
            reset_config,
            get_keymap,
            set_keymap_cell,
            reset_keymap
        ])
        .setup(move |app| setup_overlay(app, &options, &demo_states))
        .run(tauri::generate_context!())
        .expect("error while running rnitype-app");
}

/* ============================================================
   ตัวเลือกบรรทัดคำสั่ง
   ============================================================ */

/// มี RniType รันอยู่แล้วหรือไม่ (named mutex ต่อ session)
///
/// กันสอง instance: overlay จะซ้อนกัน + injector สองตัวพิมพ์ชนกัน
#[cfg(windows)]
fn already_running() -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = "Local\\RniType.SingleInstance\0".encode_utf16().collect();
    // SAFETY: ชื่อเป็น wide string ที่ null-terminate; handle ถูกปล่อยตอนโปรเซสจบ
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if handle.is_null() {
            return false;
        }
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

#[cfg(not(windows))]
fn already_running() -> bool {
    false
}

fn parse_options() -> Options {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => options.demo = true,
            "--settings" => options.settings = true,
            "--fake-pad" => options.fake_pad = true,
            "--pad-log" => options.pad_log = true,
            "--no-scancode" => options.no_scancode = true,
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
                println!(
                    "[app] usage: rnitype-app [--demo] [--fake-pad] [--pad-log] [--settings] [--no-scancode] [--seconds N]"
                );
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

/// แปลงพิกัดหน้าจอ → พิกัดหน้าต่าง overlay (HUD วาดในหน้าต่างของตัวเอง)
/// contract: `pivot`/`pointer` ที่ HUD ได้รับเป็นพิกัดในหน้าต่าง (0,0 = virtual desktop)
fn to_window_coords(state: &mut Value, origin: (i32, i32)) {
    for field in ["pivot", "pointer"] {
        let Some(point) = state.get_mut(field).and_then(Value::as_object_mut) else {
            continue;
        };
        for (key, offset) in [("x", origin.0), ("y", origin.1)] {
            if let Some(value) = point.get(key).and_then(Value::as_f64) {
                point.insert(key.to_string(), json!(value - f64::from(offset)));
            }
        }
    }
}

/// ขนาดวงมาจาก config (`ui.hudScale`) — แอปประทับลง state ทุกเฟรม
fn apply_scale(state: &mut Value, scale: f64) {
    if let Some(object) = state.as_object_mut() {
        object.insert("scale".to_string(), json!(scale));
    }
}

/// state ปัจจุบันของโหมด demo ในรูปแบบที่ HUD ต้องได้ (scale + พิกัดหน้าต่าง)
/// ⚠ ลำดับ lock ที่ใช้ทั้งไฟล์: config → demo → origin (กัน deadlock)
fn snapshot_for_hud(app: &tauri::AppHandle, scale: f64) -> Option<Value> {
    let state = app.state::<AppState>();
    let mut guard = state.demo.lock().unwrap();
    let source = guard.as_mut()?;
    if source.states.is_empty() {
        return None;
    }
    let index = source.current % source.states.len();
    let mut payload = source.states[index].clone();
    apply_scale(&mut payload, scale);

    let origin = *state.origin.lock().unwrap();
    to_window_coords(&mut payload, origin);
    Some(payload)
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
    {
        let state = app.state::<AppState>();
        *state.origin.lock().unwrap() = (vx, vy);
    }

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

    // ---- Settings (ตรวจ UI / ให้ owner เห็นหน้าต่างจริง) ----
    if options.settings {
        match open_settings(app.handle()) {
            Ok(()) => println!("[settings] เปิดหน้าต่าง Settings แล้ว"),
            Err(err) => eprintln!("[settings] เปิดไม่สำเร็จ: {err}"),
        }
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
                        let scale = {
                            let config = app_state.config.lock().unwrap();
                            config::hud_scale(&config)
                        };
                        let mut guard = app_state.demo.lock().unwrap();
                        let Some(source) = guard.as_mut() else {
                            break;
                        };
                        if source.states.is_empty() {
                            break;
                        }
                        let index = source.current % source.states.len();
                        let mut state = source.states[index].clone();
                        apply_scale(&mut state, scale);

                        let origin = *app_state.origin.lock().unwrap();
                        to_window_coords(&mut state, origin);

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
        // ---- runtime จริง: จอย → core → injector → hud:state ----
        spawn_runtime(app.handle(), options, (vx, vy, vw, vh));
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
   Runtime loop — จอย → core → injector → hud:state
   ============================================================ */

/// เริ่ม thread ที่อ่านจอย 250 Hz และส่ง `hud:state` ≤ 60 Hz (เฉพาะเมื่อเปลี่ยน)
fn spawn_runtime(
    app: &tauri::AppHandle,
    options: &Options,
    desktop: (i32, i32, u32, u32),
) {
    let handle = app.clone();
    let fake_pad = options.fake_pad;
    let pad_log = options.pad_log;
    let no_scancode = options.no_scancode;
    std::thread::spawn(move || {
        // 250 Hz ต้องใช้ sleep ละเอียด 1 ms (Windows ปกติ ~15 ms)
        let timer = rnitype_win::timer::TimerResolution::new(1);
        println!(
            "[timer] 1 ms resolution: {}",
            if timer.is_active() { "เปิด" } else { "ไม่สำเร็จ (loop จะหยาบขึ้น)" }
        );
        let _timer = timer;
        let source: Box<dyn PadSource> = if fake_pad {
            println!("[pad] ใช้จอยปลอม (--fake-pad) — ⚠ จะพิมพ์ลงหน้าต่างที่ focus อยู่ตอนนี้");
            Box::new(ScriptSource::typing_script(&[
                (0.0, 0.9),  // ↑
                (0.7, 0.7),  // ↗
                (0.9, 0.0),  // →
                (0.7, -0.7), // ↘
                (0.0, -0.9), // ↓
            ]))
        } else {
            match GilrsSource::open() {
                Ok(source) => Box::new(source),
                Err(err) => {
                    eprintln!("[pad] {err} — HUD จะ idle (ลอง --fake-pad เพื่อตรวจท่อ)");
                    return;
                }
            }
        };

        let config = handle.state::<AppState>().config.lock().unwrap().clone();
        let mut session = build_session(&config, desktop, no_scancode);
        println!(
            "[inject] key path: {}",
            if no_scancode {
                "Unicode (บังคับด้วย --no-scancode)"
            } else {
                "auto: scancode เมื่อ layout เป้าหมายพิมพ์ได้ · Unicode fallback"
            }
        );
        println!(
            "[pad] พร้อม: {} · language={:?} · languages={:?}",
            if source.name().is_empty() { "ยังไม่พบจอย" } else { "ต่อแล้ว" },
            session.engine().lang(),
            session.engine().languages().len()
        );

        run_loop(&handle, source, &mut session, (desktop.0, desktop.1), pad_log);
    });
}

/// สร้าง engine + session จาก config (`defaults.typing`, `languages`)
fn build_session(config: &Value, desktop: (i32, i32, u32, u32), no_scancode: bool) -> Session {
    let typing = config.get("defaults").and_then(|d| d.get("typing"));
    let dwell = typing
        .and_then(|t| t.get("commitDwellMs"))
        .and_then(Value::as_u64)
        .unwrap_or(60)
        .clamp(40, 120) as u32;
    let mode = typing
        .and_then(|t| t.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("en");

    let mut cfg = RuntimeConfig::default();
    cfg.gesture.commit_dwell_ms = dwell;
    let mut engine = RuntimeEngine::new(cfg);
    if mode.starts_with("th") {
        engine.set_lang(Lang::Th);
    }

    let enabled = enabled_languages(config);
    let (px, py) = primary_monitor_center();

    // pivot/pointer home = กลางจอหลัก (RS click ใน Mouse/Nav = กลับมาที่นี่)
    let mut session = Session::with_homes(
        engine,
        make_injector(no_scancode),
        enabled,
        (px, py),
        (px, py),
    );
    // HUD เริ่มที่ตำแหน่ง pointer จริง ไม่ใช่กลางจอ (กันจุด pointer เพี้ยนทั้ง session)
    if let Some(position) = rnitype_win::cursor_position() {
        session.set_pointer(position);
    }
    // pivot ต้องไม่ล้นจอ: เว้นรัศมีวง HUD (340px × scale / 2 ≈ 172px) จากขอบ virtual desktop
    apply_pivot_clamp(&mut session, config, desktop);
    session.set_keymap_overrides(&config::all_keymap_overrides(config));
    session
}

/// จำกัด pivot ตามขนาดวงล่าสุด — เรียกทั้งตอนสร้าง session และตอน config เปลี่ยน
fn apply_pivot_clamp(session: &mut Session, config: &Value, desktop: (i32, i32, u32, u32)) {
    let (vx, vy, vw, vh) = desktop;
    let margin = 172.0 * config::hud_scale(config) as f32;
    session.set_pivot_clamp(
        (vx as f32 + margin, vy as f32 + margin),
        (
            vx as f32 + vw as f32 - margin,
            vy as f32 + vh as f32 - margin,
        ),
    );
}

/// payload `app:status` (ipc.md §2) — ใช้ทั้ง command และ event
fn status_payload(state: &AppState) -> Value {
    let device = state.device.lock().unwrap().clone();
    json!({
        "device": if device.is_empty() { Value::Null } else { json!(device) },
        "connected": state.connected.load(Ordering::Relaxed),
        "paused": state.paused.load(Ordering::Relaxed),
        "admin": rnitype_win::target::own_elevated(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// push `app:status` ให้ Settings (on change)
fn emit_status(handle: &tauri::AppHandle) {
    let payload = {
        let state = handle.state::<AppState>();
        status_payload(&state)
    };
    let _ = handle.emit_to("settings", "app:status", &payload);
}

/// ภาษาที่เปิดใน config (`languages[].enabled`) — EN เป็น base เสมอ
///
/// `languages` ไม่มี/ว่าง = ค่าเริ่มต้น EN+TH · มีรายการ = ใช้ตามที่เปิด (EN บังคับเปิด)
fn enabled_languages(config: &Value) -> Vec<Lang> {
    let list = config
        .get("languages")
        .and_then(Value::as_array)
        .filter(|list| !list.is_empty());
    let Some(list) = list else {
        return vec![Lang::En, Lang::Th];
    };

    let enabled = |code: &str| {
        list.iter().any(|entry| {
            entry.get("code").and_then(Value::as_str) == Some(code)
                && entry.get("enabled").and_then(Value::as_bool).unwrap_or(false)
        })
    };

    let mut out = vec![Lang::En];
    if enabled("th") {
        out.push(Lang::Th);
    }
    out
}

/// ข้อมูล keymap ของภาษา (64 ช่อง/ชั้น + default + override) สำหรับแท็บ Keymap
///
/// `cells` = ค่าที่ใช้จริง (override ชนะ) · `defaults` = ตารางใน core · `""` = ช่องว่าง
fn keymap_payload(config: &Value, code: &str) -> Result<Value, String> {
    let lang = config::keymap_lang(code).ok_or_else(|| format!("{code}: ยังไม่มีตาราง 8×8"))?;
    let overrides = config::keymap_overrides(config, code);

    let mut layers = Vec::new();
    for layer in 1..=2u8 {
        let mut cells = Vec::new();
        let mut defaults = Vec::new();
        for group in 0..8usize {
            for sector in 0..8usize {
                let dir_group = Dir8::from_index(group);
                let dir_sector = Dir8::from_index(sector);
                let default = lookup_layer(lang, layer, dir_group, dir_sector);
                let key = overrides
                    .iter()
                    .find(|(l, g, s, _)| *l == layer && *g as usize == group && *s as usize == sector)
                    .map_or(default, |(_, _, _, key)| *key);
                defaults.push(default.map(label_of).unwrap_or_default());
                cells.push(key.map(label_of).unwrap_or_default());
            }
        }
        layers.push(json!({
            "name": if layer == 1 { "base" } else { "layer 2" },
            "cells": cells,
            "defaults": defaults,
        }));
    }

    let overrides_json: serde_json::Map<String, Value> = overrides
        .iter()
        .map(|(layer, group, sector, key)| {
            (
                format!("{layer}-{group}-{sector}"),
                json!(key.map(label_of).unwrap_or_default()),
            )
        })
        .collect();

    Ok(json!({
        "code": code,
        "label": if lang == Lang::En { "English" } else { "Thai" },
        "groups": group_names_for(lang, 1),
        "groups2": group_names_for(lang, 2),
        "layers": layers,
        "overrides": overrides_json,
    }))
}

#[cfg(windows)]
fn make_injector(no_scancode: bool) -> Box<dyn Injector + Send> {    Box::new(rnitype_win::SendInputInjector::new().with_scancode(!no_scancode))
}

#[cfg(not(windows))]
fn make_injector(_no_scancode: bool) -> Box<dyn Injector + Send> {
    Box::new(rnitype_win::NullInjector::default())
}

/// ส่ง state ซ้ำทุก N ms แม้ไม่เปลี่ยน — กัน HUD ที่หลุด event หรือถูก throttle
const HUD_RESEND_MS: u64 = 2_000;

/// loop หลัก: อ่านจอย → tick → inject → emit HUD
fn run_loop(
    handle: &tauri::AppHandle,
    mut source: Box<dyn PadSource>,
    session: &mut Session,
    origin: (i32, i32),
    pad_log: bool,
) {
    let start = Instant::now();
    let mut last_ms = 0u64;
    let mut last_emit_ms = 0u64;
    let mut last_heartbeat_ms = 0u64;
    let mut last_pad_log_ms = 0u64;
    let mut last_resend_ms = 0u64;
    let mut last_json = String::new();
    let mut had_pad = false;
    let mut was_paused = false;
    let own_elevated = rnitype_win::target::own_elevated();
    let mut last_guard_ms = 0u64;
    let mut last_config_rev = handle.state::<AppState>().config_rev.load(Ordering::Relaxed);
    let mut last_stats_push_ms = 0u64;
    let mut last_stats_log_ms = 0u64;
    let mut step_sum_us: u128 = 0;
    let mut step_count = 0u64;
    let mut step_max_us = 0u64;
    if own_elevated {
        println!("[guard] แอปรันแบบ admin — UIPI guard จะไม่บล็อกหน้าต่าง elevated");
    }

    loop {
        let now = start.elapsed().as_millis() as u64;
        let dt = (now - last_ms).clamp(1, 50) as u32;
        last_ms = now;

        let paused = handle.state::<AppState>().paused.load(Ordering::Relaxed);
        if paused != was_paused {
            println!(
                "[pad] {}",
                if paused { "พักการพิมพ์ (tray)" } else { "กลับมาพิมพ์" }
            );
            if paused {
                session.pause();
            } else {
                session.unpause();
            }
            was_paused = paused;
            emit_status(handle);
        }

        // exclusion list / UIPI — ตรวจทุก 250 ms (ไม่ต้อง query process token ทุกเฟรม 250 Hz)
        if now.saturating_sub(last_guard_ms) >= 250 {
            last_guard_ms = now;
            let app_state = handle.state::<AppState>();
            let exclusion = {
                let config = app_state.config.lock().unwrap();
                config::exclusion_list(&config)
            };

            // config เปลี่ยนจาก Settings → ภาษา/ขนาด pivot มีผลสดโดยไม่ต้องรีสตาร์ต
            let rev = app_state.config_rev.load(Ordering::Relaxed);
            if rev != last_config_rev {
                last_config_rev = rev;
                let config = app_state.config.lock().unwrap().clone();
                session.set_enabled_languages(enabled_languages(&config));
                session.set_keymap_overrides(&config::all_keymap_overrides(&config));
                apply_pivot_clamp(session, &config, virtual_desktop());
                println!(
                    "[config] runtime reload: languages={:?} hudScale={:.2}",
                    session.engine().languages().len(),
                    config::hud_scale(&config)
                );
            }

            let target = rnitype_win::target::foreground_target();
            let reason =
                rnitype_win::target::guard_reason(target.as_ref(), &exclusion, own_elevated);
            if reason != session.guard() {
                match reason {
                    Some(rnitype_win::target::BlockReason::Exclusion) => println!(
                        "[guard] หยุดส่งคีย์: {} อยู่ใน exclusion list",
                        target
                            .as_ref()
                            .and_then(|target| target.exe.clone())
                            .unwrap_or_default()
                    ),
                    Some(rnitype_win::target::BlockReason::AdminRequired) => {
                        println!("[guard] หยุดส่งคีย์: หน้าต่างเป้าหมาย elevated — รันแอปแบบ admin")
                    }
                    None => println!("[guard] กลับมาส่งคีย์ได้"),
                }
                session.set_guard(reason);
            }
        }

        let sample = source.sample(now);
        let has_pad = sample.is_some();
        if has_pad != had_pad {
            if has_pad {
                println!("[pad] จอยต่อแล้ว: {}", source.name());
                session.on_pad_found();
            } else {
                println!(
                    "[pad] จอยหลุด — ปล่อยคีย์ทั้งหมดและพัก (ส่งไปแล้ว {} คีย์)",
                    session.sent()
                );
                session.on_pad_lost();
            }
            had_pad = has_pad;
            last_emit_ms = 0; // บังคับ emit รอบถัดไป
            let app_state = handle.state::<AppState>();
            *app_state.device.lock().unwrap() =
                if has_pad { source.name() } else { String::new() };
            app_state.connected.store(has_pad, Ordering::Relaxed);
            emit_status(handle);
        }

        // ไม่มีจอย = บอกทุก 5 วิ (ให้รู้ว่าโปรเซสยังมีชีวิต ไม่ใช่ค้าง)
        if !has_pad && now.saturating_sub(last_heartbeat_ms) >= 5000 {
            last_heartbeat_ms = now;
            println!("[pad] ยังไม่พบจอย — กดปุ่มที่จอยเพื่อปลุก (หรือรัน rnitype-probe ดูค่าดิบ)");
        }

        let pad = if paused {
            PadState::default()
        } else {
            sample.unwrap_or_default()
        };

        // --pad-log: ค่าดิบที่เข้ามา + สถานะ engine (ใช้หาว่าปุ่ม/แกนถูกอ่านหรือไม่)
        if pad_log && now.saturating_sub(last_pad_log_ms) >= 250 {
            last_pad_log_ms = now;
            let buttons: Vec<&str> = pad.buttons.names().collect();
            let names = if buttons.is_empty() {
                "-".to_string()
            } else {
                buttons.join(",")
            };
            println!(
                "[pad-log] LS=({:+.2},{:+.2}) RS=({:+.2},{:+.2}) [{}] mode={:?} state={:?} lang={:?} sent={}",
                pad.ls.x,
                pad.ls.y,
                pad.rs.x,
                pad.rs.y,
                names,
                session.engine().mode(),
                session.engine().gesture_state(),
                session.engine().lang(),
                session.sent()
            );
        }

        if session.guard().is_none() {
            let step_start = Instant::now();
            let actions = session.step(&pad, dt);
            let elapsed_us = step_start.elapsed().as_micros() as u64;
            step_sum_us += u128::from(elapsed_us);
            step_count += 1;
            step_max_us = step_max_us.max(elapsed_us);
            for action in actions {
                log_action(&action);
            }
        }

        // สถิติ: อัปเดตให้ Settings ทุก 1 วิ · log ทุก 10 วิ (WPM = 5 ตัวอักษร/คำ)
        if now.saturating_sub(last_stats_push_ms) >= 1_000 {
            last_stats_push_ms = now;
            let avg_us = if step_count > 0 {
                (step_sum_us / u128::from(step_count)) as u64
            } else {
                0
            };
            let uptime = start.elapsed().as_secs();
            let payload = json!({
                "chars": session.chars(),
                "wpm": (session.wpm() * 10.0).round() / 10.0,
                "actions": session.sent(),
                "stepAvgUs": avg_us,
                "stepMaxUs": step_max_us,
                "uptimeSecs": uptime,
            });
            *handle.state::<AppState>().stats.lock().unwrap() = payload;

            if now.saturating_sub(last_stats_log_ms) >= 10_000 {
                last_stats_log_ms = now;
                println!(
                    "[stats] chars={} wpm={:.1} actions={} step avg={} max={} uptime={}",
                    session.chars(),
                    session.wpm(),
                    session.sent(),
                    stats::format_ms(avg_us),
                    stats::format_ms(step_max_us),
                    stats::format_uptime(uptime)
                );
            }
            step_sum_us = 0;
            step_count = 0;
            step_max_us = 0;
        }

        // HUD ≤ 60 Hz และเฉพาะเมื่อ payload เปลี่ยน (ipc.md §1)
        if now.saturating_sub(last_emit_ms) >= 16 {
            last_emit_ms = now;
            let app_state = handle.state::<AppState>();

            // HUD เพิ่ง handshake/รีโหลด → ล้าง cache ให้ส่งซ้ำรอบนี้
            if app_state.hud_need_resend.swap(false, Ordering::Relaxed) {
                last_json.clear();
            }

            // heartbeat: ส่งซ้ำทุก 2 วิ (1 KB/2 วิ) — HUD ไม่มีทางค้างสถานะเก่าค้างจอ
            if now.saturating_sub(last_resend_ms) >= HUD_RESEND_MS {
                last_resend_ms = now;
                last_json.clear();
            }

            let style = {
                let config = app_state.config.lock().unwrap();
                HudStyle {
                    scale: config::hud_scale(&config),
                    opacity: config::hud_opacity(&config),
                }
            };
            let state = session.hud_state(style, origin);
            if let Ok(text) = serde_json::to_string(&state)
                && text != last_json
            {
                last_json = text;
                if let Err(err) = handle.emit_to("hud", "hud:state", &state) {
                    eprintln!("[pad] ส่ง hud:state ไม่สำเร็จ: {err}");
                }
            }
        }

        // ~250 Hz (dt จริงวัดจากนาฬิกา จึงไม่เพี้ยนถ้า sleep ยืด)
        std::thread::sleep(Duration::from_millis(3));
    }
}

/// log เฉพาะ action ที่ทำให้เกิดผลจริง (คีย์/คลิก) — ไม่ log ทุกเฟรม
fn log_action(action: &rnitype_core::Action) {
    use rnitype_core::Action;
    match action {
        Action::Key(press) => println!(
            "[type] {}{}{}{}",
            if press.ctrl { "Ctrl+" } else { "" },
            if press.shift { "Shift+" } else { "" },
            if press.alt { "Alt+" } else { "" },
            press.key
        ),
        Action::KeyDown(press) => println!("[type] down {}", press.key),
        Action::KeyUp(press) => println!("[type] up {}", press.key),
        Action::MouseButton { button, down } => {
            println!("[mouse] {button:?} {}", if *down { "down" } else { "up" })
        }
        Action::MouseMove { .. } | Action::MovePivot { .. } => {}
        Action::MouseWheel { delta } => println!("[mouse] wheel {delta}"),
        Action::MouseHWheel { delta } => println!("[mouse] hwheel {delta}"),
        Action::RecenterPointer => println!("[mouse] recenter pointer"),
        Action::ResetPivot => println!("[nav] reset pivot"),
        other => println!("[pad] {other:?}"),
    }
}

/* ============================================================
   Tray
   ============================================================ */

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let pause_item = MenuItem::with_id(app, "pause", "Pause / Resume typing", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit RniType", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&pause_item, &settings_item, &quit_item])?;

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
        .on_tray_icon_event(|tray, event| {
            // คลิกซ้ายที่ไอคอน = เปิด Settings (ทางลัดแบบ Logi Options+)
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                println!("[tray] คลิกซ้ายไอคอน → เปิด Settings");
                if let Err(err) = open_settings(tray.app_handle()) {
                    eprintln!("[tray] เปิด Settings ไม่สำเร็จ: {err}");
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "pause" => {
                let state = app.state::<AppState>();
                let paused = state.paused.fetch_xor(true, Ordering::Relaxed);
                let paused = !paused;
                println!("[tray] {}", if paused { "พักการพิมพ์" } else { "กลับมาพิมพ์" });
            }
            "settings" => {
                println!("[tray] เลือก Settings → เปิดหน้าต่างตั้งค่า");
                if let Err(err) = open_settings(app) {
                    eprintln!("[tray] เปิด Settings ไม่สำเร็จ: {err}");
                }
            }
            "quit" => {
                println!("[tray] เลือก Quit → ปิดแอป");
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// หน้าต่าง Settings (P5 slice 2) — หน้าต่างจริง มีขอบ มี focus (ต่างจาก HUD)
fn open_settings(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("settings/index.html".into()),
    )
    .title("RniType Settings")
    .inner_size(760.0, 640.0)
    .min_inner_size(680.0, 480.0)
    .focused(true)
    .center()
    .build()?;

    // ให้หน้าต่างขึ้นมาเหนือหน้าต่างอื่นเสมอ (บางครั้ง Windows ไม่ยกให้เอง)
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();

    Ok(())
}

/* ============================================================
   Commands (ipc.md §3)
   ============================================================ */

/// HUD แจ้งว่าพร้อมรับ state (ipc.md §3) — log + ส่ง state ให้ HUD ทันทีหลัง handshake
/// (HUD เป็น read-only จึงไม่มี command อื่นที่เปลี่ยน state ของแอป)
#[tauri::command]
fn hud_ready(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Value {
    let first = !state
        .hud_ready
        .swap(true, std::sync::atomic::Ordering::SeqCst);
    println!(
        "[hud] hud_ready{} — HUD โหลดแล้วและมี window.__TAURI__ (handshake สำเร็จ)",
        if first { " (ครั้งแรก)" } else { "" }
    );

    // ipc.md §4 — หลัง handshake ต้องได้ state ทันที แม้ค่าจะไม่เปลี่ยนจากรอบก่อน
    // (HUD อาจบูตช้ากว่าที่แอปเริ่มส่ง หรือเพิ่งรีโหลด)
    state
        .hud_need_resend
        .store(true, std::sync::atomic::Ordering::SeqCst);

    // ipc.md §4 — หลัง handshake แอปส่ง state ให้ HUD ทันที (state ปัจจุบัน + scale จาก config)
    let scale = {
        let config = state.config.lock().unwrap();
        config::hud_scale(&config)
    };
    if let Some(payload) = snapshot_for_hud(&app, scale) {
        let mode = payload.get("mode").and_then(Value::as_str).unwrap_or("?");
        let st = payload.get("state").and_then(Value::as_str).unwrap_or("?");
        match app.emit_to("hud", "hud:state", &payload) {
            Ok(()) => println!("[hud] ส่ง hud:state ให้ HUD หลัง handshake (mode={mode} state={st})"),
            Err(err) => eprintln!("[hud] ส่ง hud:state หลัง handshake ไม่สำเร็จ: {err}"),
        }
    }

    json!({ "ok": true })
}

/// typography ปัจจุบันของ HUD — `{ textScale, font }` (ipc.md §3 `hud_style`)
/// HUD เรียกครั้งเดียวตอน boot เพื่อกันหลุด event ที่ยิงก่อนหน้าต่างพร้อม
#[tauri::command]
fn hud_style(state: tauri::State<'_, AppState>) -> Value {
    config::ui_style(&state.config.lock().unwrap())
}

/// Settings อ่าน config ทั้งไฟล์ (ipc.md §3 `get_config`)
#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Value {
    json!({ "ok": true, "data": state.config.lock().unwrap().clone() })
}

/// Settings บันทึก config (ipc.md §3 `save_config`) → เก็บลงดิสก์ + push style ให้ HUD
#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: Value,
) -> Value {
    if let Err(message) = config::validate_ui(&config) {
        eprintln!("[config] ปฏิเสธการบันทึก: {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }
    if let Err(message) = config::validate_keymaps(&config) {
        eprintln!("[config] ปฏิเสธ keymap: {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }

    let style = config::ui_style(&config);
    {
        let mut guard = state.config.lock().unwrap();
        *guard = config.clone();
        if let Err(message) = config::save(&guard) {
            eprintln!("[config] {message}");
            return json!({ "ok": false, "error": { "code": "CONFIG_IO", "message": message } });
        }
    }

    // typography เปลี่ยน → บอก HUD ทันที (ไม่ต้องรอ state เฟรมถัดไป)
    if let Err(err) = app.emit_to("hud", "hud:style", &style) {
        eprintln!("[config] ส่ง hud:style ไม่สำเร็จ: {err}");
    }
    // ขนาดวงอยู่ใน state (scale) → ส่ง state ปัจจุบันซ้ำทันที ให้ slider เห็นผลทันที
    let scale = config::hud_scale(&config);
    if let Some(payload) = snapshot_for_hud(&app, scale) {
        let _ = app.emit_to("hud", "hud:state", &payload);
    }
    // หน้าต่าง Settings อื่น ๆ (ถ้าเปิดหลายตัว) ให้เห็นค่าใหม่
    let _ = app.emit_to("settings", "settings:config", &config);

    // runtime loop เห็นค่าใหม่รอบถัดไป (ภาษา/ขนาด pivot/exclusion)
    state.touch_config();

    println!(
        "[config] บันทึกแล้ว: {}",
        serde_json::to_string(&style).unwrap_or_default()
    );
    json!({ "ok": true, "data": config })
}

/// General tab — สถานะจอย/pause/admin/version (ipc.md §3 `get_status`)
#[tauri::command]
fn get_status(state: tauri::State<'_, AppState>) -> Value {
    json!({ "ok": true, "data": status_payload(&state) })
}

/// Keymap tab — อ่านตาราง 8×8 ของภาษา (default + override)
#[tauri::command]
fn get_keymap(state: tauri::State<'_, AppState>, code: String) -> Value {
    let config = state.config.lock().unwrap().clone();
    match keymap_payload(&config, &code) {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(message) => {
            json!({ "ok": false, "error": { "code": "LANG_NOT_FOUND", "message": message } })
        }
    }
}

/// Keymap tab — แก้ช่องเดียว (`text` ว่าง = ลบคีย์ เฉพาะ layer 2)
#[tauri::command]
fn set_keymap_cell(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    code: String,
    layer: u8,
    group: u8,
    sector: u8,
    text: String,
) -> Value {
    let mut next = state.config.lock().unwrap().clone();
    if let Err(message) = config::set_keymap_cell(&mut next, &code, layer, group, sector, &text) {
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }
    if let Err(message) = config::validate_keymaps(&next) {
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }
    if let Err(message) = config::save(&next) {
        eprintln!("[config] {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_IO", "message": message } });
    }

    *state.config.lock().unwrap() = next.clone();
    state.touch_config();
    let _ = app.emit_to("settings", "settings:config", &next);
    println!("[keymap] {code} L{layer} G{group} S{sector} = {text:?}");
    match keymap_payload(&next, &code) {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(message) => {
            json!({ "ok": false, "error": { "code": "INTERNAL", "message": message } })
        }
    }
}

/// Keymap tab — Reset ทั้งภาษา หรือเฉพาะกลุ่ม
#[tauri::command]
fn reset_keymap(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    code: String,
    group: Option<u8>,
) -> Value {
    let mut next = state.config.lock().unwrap().clone();
    if !config::reset_keymap(&mut next, &code, group) {
        return json!({
            "ok": false,
            "error": { "code": "LANG_NOT_FOUND", "message": format!("ไม่พบภาษา {code}") }
        });
    }
    if let Err(message) = config::save(&next) {
        eprintln!("[config] {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_IO", "message": message } });
    }

    *state.config.lock().unwrap() = next.clone();
    state.touch_config();
    let _ = app.emit_to("settings", "settings:config", &next);
    println!("[keymap] reset {code} group={group:?}");
    match keymap_payload(&next, &code) {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(message) => {
            json!({ "ok": false, "error": { "code": "LANG_NOT_FOUND", "message": message } })
        }
    }
}

/// General tab — ตัวเลข usage ล่าสุด (chars/WPM/latency/uptime)
#[tauri::command]
fn get_stats(state: tauri::State<'_, AppState>) -> Value {
    let data = state.stats.lock().unwrap().clone();
    json!({ "ok": true, "data": data })
}

/// General tab — พัก/กลับมาพิมพ์จาก Settings (เท่ากับเมนู tray)
#[tauri::command]
fn set_paused(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    paused: bool,
) -> Value {
    state.paused.store(paused, Ordering::Relaxed);
    let payload = status_payload(&state);
    let _ = app.emit_to("settings", "app:status", &payload);
    println!(
        "[settings] {}",
        if paused { "พักการพิมพ์" } else { "กลับมาพิมพ์" }
    );
    json!({ "ok": true, "data": payload })
}

/// Languages tab — ตรวจ layout/IME ที่ติดตั้ง แล้วรวมกับ config เป็นแถวของ UI
#[tauri::command]
fn rescan_languages(state: tauri::State<'_, AppState>) -> Value {
    let detected = rnitype_win::language::installed_languages();
    let config = state.config.lock().unwrap().clone();
    let rows = config::language_rows(&config, &detected);
    let enabled = rows
        .iter()
        .filter(|row| row.get("enabled").and_then(Value::as_bool).unwrap_or(false))
        .count();
    println!(
        "[languages] ตรวจพบ {} layout/IME · เปิดใช้ {enabled}",
        detected.len()
    );
    for row in &rows {
        println!(
            "[languages] {} {} ({}) {}",
            row.get("code").and_then(Value::as_str).unwrap_or("?"),
            row.get("label").and_then(Value::as_str).unwrap_or(""),
            row.get("klid").and_then(Value::as_str).unwrap_or(""),
            if row.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                "enabled"
            } else {
                "off"
            }
        );
    }
    json!({ "ok": true, "data": rows })
}

/// Languages tab — เปิด/ปิดภาษา (เก็บลง config + ให้ runtime เห็นทันที)
#[tauri::command]
fn set_language_enabled(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    code: String,
    enabled: bool,
) -> Value {
    let mut next = state.config.lock().unwrap().clone();
    if !config::set_language_enabled(&mut next, &code, enabled) {
        return json!({
            "ok": false,
            "error": {
                "code": "CONFIG_INVALID",
                "message": format!("เปิดได้สูงสุด {} ภาษา", config::MAX_ENABLED_LANGUAGES)
            }
        });
    }
    if let Err(message) = config::validate_languages(&next) {
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }
    if let Err(message) = config::save(&next) {
        eprintln!("[config] {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_IO", "message": message } });
    }

    *state.config.lock().unwrap() = next.clone();
    state.touch_config();
    let _ = app.emit_to("settings", "settings:config", &next);
    println!(
        "[languages] {code}: {}",
        if enabled { "enabled" } else { "disabled" }
    );
    json!({ "ok": true, "data": next })
}

/// Reset เฉพาะส่วน (General/HUD/Languages) หรือทั้งหมด — คืน config ใหม่
#[tauri::command]
fn reset_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    section: String,
) -> Value {
    let mut next = state.config.lock().unwrap().clone();
    if !config::reset_section(&mut next, &section) {
        return json!({
            "ok": false,
            "error": { "code": "CONFIG_INVALID", "message": format!("ไม่รู้จักส่วน {section}") }
        });
    }
    if let Err(message) = config::validate_languages(&next) {
        return json!({ "ok": false, "error": { "code": "CONFIG_INVALID", "message": message } });
    }
    if let Err(message) = config::save(&next) {
        eprintln!("[config] {message}");
        return json!({ "ok": false, "error": { "code": "CONFIG_IO", "message": message } });
    }

    *state.config.lock().unwrap() = next.clone();
    state.touch_config();
    let style = config::ui_style(&next);
    let _ = app.emit_to("hud", "hud:style", &style);
    let _ = app.emit_to("settings", "settings:config", &next);
    println!("[config] reset section={section}");
    json!({ "ok": true, "data": next })
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

/// จุดกลางจอหลัก (พิกัดหน้าจอ) — ตำแหน่งเริ่มต้นของ HUD pivot
#[cfg(windows)]
fn primary_monitor_center() -> (f32, f32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN) as f32 / 2.0,
            GetSystemMetrics(SM_CYSCREEN) as f32 / 2.0,
        )
    }
}

#[cfg(not(windows))]
fn primary_monitor_center() -> (f32, f32) {
    (960.0, 540.0)
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

#[cfg(test)]
mod enabled_language_tests {
    use super::*;

    #[test]
    fn missing_language_list_defaults_to_en_and_th() {
        assert_eq!(enabled_languages(&json!({})), vec![Lang::En, Lang::Th]);
        assert_eq!(
            enabled_languages(&json!({ "languages": [] })),
            vec![Lang::En, Lang::Th]
        );
    }

    #[test]
    fn disabled_thai_leaves_english_as_the_only_language() {
        let config = json!({ "languages": [ { "code": "th", "enabled": false } ] });
        assert_eq!(enabled_languages(&config), vec![Lang::En]);
    }

    #[test]
    fn enabled_thai_is_added_after_english() {
        let config = json!({ "languages": [ { "code": "th", "enabled": true } ] });
        assert_eq!(enabled_languages(&config), vec![Lang::En, Lang::Th]);
    }
}
