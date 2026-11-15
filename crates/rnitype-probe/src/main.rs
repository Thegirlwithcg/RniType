//! rnitype-probe — Spike A
//!
//! ตรวจว่าจอยที่ต่ออยู่ส่งค่าครบตาม contract ของ `rnitype-core`:
//! LS/RS (8 ทิศ), trigger แบบ analog, ปุ่มครบ 8 + D-pad, deadzone/drift
//!
//! ```text
//! cargo run -p rnitype-probe -- --seconds 20 --raw
//! ```

use std::collections::BTreeSet;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use rnitype_core::normalizer::{Dir8, Normalizer, NormalizerConfig, SectorTracker};
use rnitype_core::pad::{Buttons, PadState, Vec2};

/// แกนที่ probe จะรายงาน (บางรุ่นใช้ LeftZ/RightZ, บางรุ่นใช้ LeftZ2/RightZ2)
const AXES: [(Axis, &str); 6] = [
    (Axis::LeftStickX, "LSx"),
    (Axis::LeftStickY, "LSy"),
    (Axis::RightStickX, "RSx"),
    (Axis::RightStickY, "RSy"),
    (Axis::LeftZ, "LZ"),
    (Axis::RightZ, "RZ"),
];

const BUTTONS: [(Button, &str); 16] = [
    (Button::South, "A/South"),
    (Button::East, "B/East"),
    (Button::West, "X/West"),
    (Button::North, "Y/North"),
    (Button::LeftTrigger, "LB"),
    (Button::RightTrigger, "RB"),
    (Button::LeftTrigger2, "LT"),
    (Button::RightTrigger2, "RT"),
    (Button::LeftThumb, "LS"),
    (Button::RightThumb, "RS"),
    (Button::DPadUp, "D-Up"),
    (Button::DPadDown, "D-Down"),
    (Button::DPadLeft, "D-Left"),
    (Button::DPadRight, "D-Right"),
    (Button::Select, "View"),
    (Button::Start, "Menu"),
];

struct Args {
    seconds: f32,
    print_hz: f32,
    raw: bool,
}

impl Args {
    fn parse() -> Self {
        let mut args = Args {
            seconds: 20.0,
            print_hz: 5.0,
            raw: false,
        };
        let mut it = std::env::args().skip(1);
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--seconds" => {
                    if let Some(v) = it.next().and_then(|v| v.parse().ok()) {
                        args.seconds = v;
                    }
                }
                "--hz" => {
                    if let Some(v) = it.next().and_then(|v| v.parse().ok()) {
                        args.print_hz = v;
                    }
                }
                "--raw" => args.raw = true,
                "--help" | "-h" => {
                    println!("usage: rnitype-probe [--seconds N] [--hz N] [--raw]");
                    std::process::exit(0);
                }
                other => eprintln!("ไม่รู้จัก argument: {other}"),
            }
        }
        args
    }
}

#[derive(Default)]
struct AxisStats {
    min: f32,
    max: f32,
    touched: bool,
}

impl AxisStats {
    fn feed(&mut self, v: f32) {
        if !self.touched {
            self.min = v;
            self.max = v;
            self.touched = true;
        } else {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
    }
}

#[derive(Default)]
struct PadStats {
    name: String,
    axes: Vec<AxisStats>,
    buttons: BTreeSet<&'static str>,
    sectors: BTreeSet<&'static str>,
    idle_peak: f32,
    max_magnitude: f32,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(err) => {
            eprintln!("FAIL: เปิด gilrs ไม่ได้: {err}");
            return ExitCode::from(2);
        }
    };

    println!(
        "RniType probe (spike A) — gilrs {}",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "args: seconds={} hz={} raw={}",
        args.seconds, args.print_hz, args.raw
    );

    let mut detected: Vec<(GamepadId, String)> = gilrs
        .gamepads()
        .map(|(id, pad)| (id, pad.name().to_string()))
        .collect();

    if detected.is_empty() {
        println!("pads: 0 — รอจอย 15 วินาที (เปิดจอย/กดปุ่ม guide ได้เลย)…");
        let wait_until = Instant::now() + Duration::from_secs(15);
        while detected.is_empty() && Instant::now() < wait_until {
            while gilrs.next_event().is_some() {}
            sleep(Duration::from_millis(250));
            detected = gilrs
                .gamepads()
                .map(|(id, pad)| (id, pad.name().to_string()))
                .collect();
        }
    }

    if detected.is_empty() {
        println!("pads: 0");
        println!("FAIL: ไม่พบจอย — ตรวจตามลำดับ:");
        println!("  1) ต่อ USB ก่อน แล้วค่อยลอง Bluetooth");
        println!("  2) กดปุ่ม Xbox/PS ให้จอยตื่น");
        println!("  3) ปิด Steam Input / DS4Windows / โปรแกรมที่จับจอยอยู่");
        println!("  4) ดู Device Manager ว่า Windows เห็นจอย");
        println!("  5) รันจาก repo root: cargo run -p rnitype-probe -- --seconds 20 --raw");
        return ExitCode::from(2);
    }

    println!("pads: {}", detected.len());
    for (id, name) in &detected {
        println!("  [{id}] {name} — kind={:?}", kind_of(name));
    }
    let mut pads = detected;
    println!("เริ่มเก็บค่า… กด LS/RS ครบ 8 ทิศ, trigger, ปุ่มหน้า, D-pad");

    let mut normalizer = Normalizer::new(NormalizerConfig::default());
    let mut ls_tracker = SectorTracker::new(9.0);
    let mut rs_tracker = SectorTracker::new(9.0);
    let mut stats: Vec<PadStats> = Vec::new();

    let start = Instant::now();
    let mut next_print = start;
    let print_every = Duration::from_secs_f32(1.0 / args.print_hz.max(1.0));

    loop {
        while let Some(ev) = gilrs.next_event() {
            match ev.event {
                EventType::Connected => {
                    let name = gilrs.gamepad(ev.id).name().to_string();
                    println!("+ connected [{:?}] {}", ev.id, name);
                }
                EventType::Disconnected => println!("- disconnected [{:?}]", ev.id),
                EventType::ButtonPressed(b, _) => {
                    let label = BUTTONS
                        .iter()
                        .find(|(btn, _)| *btn == b)
                        .map(|(_, label)| *label)
                        .unwrap_or("?");
                    println!("  press   [{:?}] {label}", ev.id);
                }
                EventType::ButtonReleased(b, _) => {
                    let label = BUTTONS
                        .iter()
                        .find(|(btn, _)| *btn == b)
                        .map(|(_, label)| *label)
                        .unwrap_or("?");
                    println!("  release [{:?}] {label}", ev.id);
                }
                EventType::AxisChanged(axis, value, _) if args.raw => {
                    println!("  axis    [{:?}] {axis:?} = {value:+.3}", ev.id);
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let current: Vec<(GamepadId, String)> = gilrs
            .gamepads()
            .map(|(id, pad)| (id, pad.name().to_string()))
            .collect();
        if current.len() != pads.len() {
            pads = current;
        }

        if now >= next_print {
            next_print = now + print_every;
            print_snapshot(
                &mut gilrs,
                &pads,
                &args,
                &mut normalizer,
                &mut ls_tracker,
                &mut rs_tracker,
                &mut stats,
            );
        }

        if args.seconds > 0.0 && start.elapsed().as_secs_f32() >= args.seconds {
            break;
        }
        sleep(Duration::from_millis(4));
    }

    println!();
    println!("--- สรุป ({} s) ---", start.elapsed().as_secs_f32());
    print_summary(&pads, &stats);
    if pads.is_empty() {
        return ExitCode::from(2);
    }
    println!("OK: probe รันจบ — ส่ง log นี้ให้ agent เพื่อปิด spike A");
    ExitCode::SUCCESS
}

fn print_snapshot(
    gilrs: &mut Gilrs,
    pads: &[(GamepadId, String)],
    args: &Args,
    normalizer: &mut Normalizer,
    ls_tracker: &mut SectorTracker,
    rs_tracker: &mut SectorTracker,
    stats: &mut Vec<PadStats>,
) {
    for (slot, (id, name)) in pads.iter().enumerate() {
        while stats.len() <= slot {
            stats.push(PadStats::default());
        }
        let pad = gilrs.gamepad(*id);
        let state = read_state(&pad);
        let normalized = normalizer.apply(&state);
        let ls_dir = ls_tracker.update(normalized.ls, 0.35);
        let rs_dir = rs_tracker.update(normalized.rs, 0.40);

        let slot_stats = &mut stats[slot];
        if slot_stats.name.is_empty() {
            slot_stats.name = name.clone();
            slot_stats.axes = (0..AXES.len()).map(|_| AxisStats::default()).collect();
        }
        for (i, (axis, _)) in AXES.iter().enumerate() {
            slot_stats.axes[i].feed(pad.value(*axis));
        }
        for (btn, label) in BUTTONS {
            if pad.is_pressed(btn) {
                slot_stats.buttons.insert(label);
            }
        }
        for dir in [ls_dir, rs_dir].into_iter().flatten() {
            slot_stats.sectors.insert(dir.symbol());
        }
        let mag = normalized.ls.magnitude().max(normalized.rs.magnitude());
        slot_stats.max_magnitude = slot_stats.max_magnitude.max(mag);
        if mag < 0.2 {
            slot_stats.idle_peak = slot_stats.idle_peak.max(mag);
        }

        let pressed: Vec<&str> = state.buttons.names().collect();
        println!(
            "[{}] {:<32} LS({:+.2},{:+.2}) RS({:+.2},{:+.2}) LT {:.2} RT {:.2} | {} | ls={} rs={}",
            id,
            truncate(name, 32),
            normalized.ls.x,
            normalized.ls.y,
            normalized.rs.x,
            normalized.rs.y,
            normalized.lt_analog,
            normalized.rt_analog,
            if pressed.is_empty() {
                "-".to_string()
            } else {
                pressed.join("+")
            },
            ls_dir.map(Dir8::symbol).unwrap_or("·"),
            rs_dir.map(Dir8::symbol).unwrap_or("·"),
        );

        if args.raw {
            let raw: Vec<String> = AXES
                .iter()
                .map(|(axis, label)| format!("{label}={:+.3}", pad.value(*axis)))
                .collect();
            println!("     raw: {}", raw.join(" "));
            println!("     power: {:?}", pad.power_info());
        }
    }
}

fn print_summary(pads: &[(GamepadId, String)], stats: &[PadStats]) {
    for (slot, (id, name)) in pads.iter().enumerate() {
        let s = match stats.get(slot) {
            Some(s) => s,
            None => continue,
        };
        println!(
            "[{id}] {}",
            if s.name.is_empty() {
                name.as_str()
            } else {
                s.name.as_str()
            }
        );
        for (i, (_axis, label)) in AXES.iter().enumerate() {
            if let Some(a) = s.axes.get(i)
                && a.touched
            {
                println!(
                    "  {label:<4} range {min:+.2} .. {max:+.2}",
                    min = a.min,
                    max = a.max
                );
            }
        }
        println!(
            "  idle_peak={:.3} (deadzone 0.15 {})  max_magnitude={:.2}",
            s.idle_peak,
            if s.idle_peak < 0.15 {
                "OK"
            } else {
                "สูงเกิน — ต้องขยับ deadzone"
            },
            s.max_magnitude
        );
        println!(
            "  buttons: {}",
            if s.buttons.is_empty() {
                "-".into()
            } else {
                s.buttons.iter().copied().collect::<Vec<_>>().join(", ")
            }
        );
        println!(
            "  sectors: {}",
            if s.sectors.is_empty() {
                "-".into()
            } else {
                s.sectors.iter().copied().collect::<Vec<_>>().join(" ")
            }
        );
        println!(
            "  expected buttons: {}",
            BUTTONS
                .iter()
                .map(|(_, l)| *l)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn read_state(pad: &gilrs::Gamepad) -> PadState {
    let lt = pad.value(Axis::LeftZ).clamp(0.0, 1.0);
    let rt = pad.value(Axis::RightZ).clamp(0.0, 1.0);

    PadState {
        ls: Vec2::new(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY)),
        rs: Vec2::new(pad.value(Axis::RightStickX), pad.value(Axis::RightStickY)),
        lt_analog: lt,
        rt_analog: rt,
        buttons: Buttons {
            a: pad.is_pressed(Button::South),
            b: pad.is_pressed(Button::East),
            x: pad.is_pressed(Button::West),
            y: pad.is_pressed(Button::North),
            lb: pad.is_pressed(Button::LeftTrigger),
            rb: pad.is_pressed(Button::RightTrigger),
            lt: pad.is_pressed(Button::LeftTrigger2),
            rt: pad.is_pressed(Button::RightTrigger2),
            ls_click: pad.is_pressed(Button::LeftThumb),
            rs_click: pad.is_pressed(Button::RightThumb),
            dpad_up: pad.is_pressed(Button::DPadUp),
            dpad_down: pad.is_pressed(Button::DPadDown),
            dpad_left: pad.is_pressed(Button::DPadLeft),
            dpad_right: pad.is_pressed(Button::DPadRight),
            view: pad.is_pressed(Button::Select),
            menu: pad.is_pressed(Button::Start),
        },
    }
}

fn kind_of(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("dualsense") || lower.contains("dualshock") || lower.contains("ps5") {
        "PlayStation"
    } else if lower.contains("xbox") || lower.contains("xinput") {
        "Xbox"
    } else {
        "Unknown"
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
