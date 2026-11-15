//! Spike C — SendInput Unicode path
//!
//! วิธีรัน:
//!   cargo run --release --manifest-path spikes/sendinput/Cargo.toml
//!   → เปิด Notepad วาง cursor แล้วรอ countdown 3 วินาที
//!
//! ผ่านเมื่อ: "กขคง Hello ไทย 123" ปรากฏถูกทุกตัวใน Notepad
//! (ถ้า Notepad ได้รับตัวอักษรแต่เป็นคนละตัว → ปัญหา layout/IME ไม่ใช่ SendInput)

use std::time::Duration;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    SendInput, VK_F8,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let text = arg_value(&args, "--text").unwrap_or_else(|| "กขคง Hello ไทย 123".to_string());
    let delay_ms = arg_value(&args, "--delay")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(8);

    println!("Spike C — SendInput Unicode");
    println!("ข้อความ: {text:?} ({} UTF-16 units)", text.encode_utf16().count());
    println!();
    println!("1) เปิด Notepad");
    println!("2) คลิกใน Notepad ให้ cursor อยู่ตรงนั้น");
    println!("3) กด F8 เพื่อเริ่มส่ง (ไม่ต้องกลับมาที่ terminal)");
    wait_for_f8();
    println!("เริ่มส่ง…");
    std::thread::sleep(Duration::from_millis(300));

    let mut sent_units = 0usize;
    for unit in text.encode_utf16() {
        send(unit, false);
        std::thread::sleep(Duration::from_millis(delay_ms));
        send(unit, true);
        std::thread::sleep(Duration::from_millis(delay_ms));
        sent_units += 1;
    }

    println!("ส่งแล้ว {sent_units} units — ตรวจ Notepad:");
    println!("  ผ่าน = ข้อความตรงทุกตัว");
    println!("  ไม่ผ่าน = จดสิ่งที่ได้ + เวอร์ชัน Windows/Notepad ไว้ใน HANDOFF");
}

fn wait_for_f8() {
    loop {
        let down = (unsafe { GetAsyncKeyState(VK_F8 as i32) } as u16 & 0x8000) != 0;
        if down {
            // รอให้ปล่อยปุ่มก่อน กันส่งซ้ำ
            while (unsafe { GetAsyncKeyState(VK_F8 as i32) } as u16 & 0x8000) != 0 {
                std::thread::sleep(Duration::from_millis(20));
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn send(unit: u16, up: bool) {
    let mut input: INPUT = unsafe { std::mem::zeroed() };
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: 0,
        wScan: unit,
        dwFlags: KEYEVENTF_UNICODE | if up { KEYEVENTF_KEYUP } else { 0 },
        time: 0,
        dwExtraInfo: 0,
    };

    let sent = unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
    if sent != 1 {
        eprintln!("SendInput ล้มเหลว (unit=0x{unit:04X}, up={up})");
    }
}
