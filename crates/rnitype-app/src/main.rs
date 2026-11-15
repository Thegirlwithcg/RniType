//! rnitype-app — runtime skeleton
//!
//! P1: มีไว้เพื่อ freeze workspace layout และ dependency direction
//! P3: ต่อ device → core → injector → HUD ที่นี่ (thread 250 Hz + tray + config)

fn main() {
    let _ = rnitype_win::NullInjector::default();
    eprintln!("rnitype-app: runtime skeleton — P3 will wire device → core → injector → HUD");
    std::process::exit(2);
}
