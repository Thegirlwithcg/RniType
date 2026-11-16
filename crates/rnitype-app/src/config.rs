//! config.json — ส่วนที่แอปใช้จริงตอนนี้คือ `ui.*` (typography ของ HUD)
//!
//! path: `%APPDATA%\RniType\config.json` (ตรงกับ contract config.schema.json)
//! * ถ้าไฟล์ยังไม่มี → ใช้ config เริ่มต้น (schema-valid) แต่ **ยังไม่เขียนลงดิสก์**
//!   จนกว่าผู้ใช้จะแก้ค่าจริง (กันการไปสร้างไฟล์ทับของ P5)
//! * save = merge ลง object เดิม → key ที่ยังไม่รู้จักไม่หาย

use std::path::PathBuf;

use serde_json::{json, Value};

/// ฟอนต์ที่ HUD รองรับ (ตรงกับ ui/shared/fonts.css)
pub const FONTS: [&str; 3] = ["ibm-plex-sans-thai", "noto-sans-thai", "system"];
pub const FONT_DEFAULT: &str = "ibm-plex-sans-thai";

/// ขนาดตัวหนังสือของ HUD (0.8–2.0) — ตัวคูณแยกจากขนาดวง
pub const TEXT_SCALE_DEFAULT: f64 = 1.4;
pub const TEXT_SCALE_MIN: f64 = 0.8;
pub const TEXT_SCALE_MAX: f64 = 2.0;

/// ขนาดวงเริ่มต้น — 1.0 = 340px (ขนาดที่ออกแบบ) · owner 2026-11-15: 1.2 ใหญ่ไปนิด
pub const HUD_SCALE_DEFAULT: f64 = 1.0;
pub const HUD_SCALE_MIN: f64 = 0.5;
pub const HUD_SCALE_MAX: f64 = 3.0;

pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
    Some(base.join("RniType").join("config.json"))
}

/// config เริ่มต้น — ตรงกับ contract (ว่างได้ แต่ต้องผ่าน schema)
pub fn default_config() -> Value {
    json!({
        "version": 1,
        "defaults": {
            "typing": { "mode": "th", "commitDwellMs": 60, "hud": true, "ringAlways": false, "layer": 1 },
            "mouse": { "vmax": 1600, "gamma": 1.7, "deadzone": 0.15 },
            "hotkeys": { "emergency": "LB+RB", "hudToggle": "LT+RT" }
        },
        "languages": [],
        "keymaps": {},
        "profiles": [],
        "ui": {
            "language": "en",
            "hudScale": HUD_SCALE_DEFAULT,
            "hudOpacity": 0.92,
            "hudTextScale": TEXT_SCALE_DEFAULT,
            "hudFontFamily": FONT_DEFAULT
        },
        "system": { "autostart": false, "runAsAdmin": false, "exclusion": [] }
    })
}

/// อ่าน config จากดิสก์ (ไม่มี/พัง = ค่าเริ่มต้น — แอปต้องรันได้เสมอ)
pub fn load() -> Value {
    let Some(path) = config_path() else {
        return default_config();
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(value) if value.is_object() => value,
            Ok(_) => {
                eprintln!("[config] {} ไม่ใช่ object — ใช้ค่าเริ่มต้น", path.display());
                default_config()
            }
            Err(err) => {
                eprintln!("[config] อ่าน {} ไม่ได้: {err} — ใช้ค่าเริ่มต้น", path.display());
                default_config()
            }
        },
        Err(_) => default_config(),
    }
}

/// typography ที่ HUD ต้องใช้ — `{ textScale, font }` (ipc.md `hud_style`)
pub fn ui_style(config: &Value) -> Value {
    let ui = config.get("ui");
    let text_scale = ui
        .and_then(|ui| ui.get("hudTextScale"))
        .and_then(Value::as_f64)
        .filter(|v| (TEXT_SCALE_MIN..=TEXT_SCALE_MAX).contains(v))
        .unwrap_or(TEXT_SCALE_DEFAULT);
    let font = ui
        .and_then(|ui| ui.get("hudFontFamily"))
        .and_then(Value::as_str)
        .filter(|v| FONTS.contains(v))
        .unwrap_or(FONT_DEFAULT);

    json!({ "textScale": text_scale, "font": font })
}

/// ขนาดวง (`ui.hudScale`) — แอปประทับลง `HudState.scale` ทุกเฟรม
pub fn hud_scale(config: &Value) -> f64 {
    config
        .get("ui")
        .and_then(|ui| ui.get("hudScale"))
        .and_then(Value::as_f64)
        .filter(|v| (HUD_SCALE_MIN..=HUD_SCALE_MAX).contains(v))
        .unwrap_or(HUD_SCALE_DEFAULT)
}

/// ความเข้มของ HUD (`ui.hudOpacity`)
pub fn hud_opacity(config: &Value) -> f64 {
    config
        .get("ui")
        .and_then(|ui| ui.get("hudOpacity"))
        .and_then(Value::as_f64)
        .filter(|v| (0.1..=1.0).contains(v))
        .unwrap_or(0.92)
}

/// รายการ exe ที่ห้ามส่งคีย์ (`system.exclusion`) — trim และตัดค่าว่าง
pub fn exclusion_list(config: &Value) -> Vec<String> {
    config
        .get("system")
        .and_then(|system| system.get("exclusion"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// settings ของ HUD ที่ถูกต้อง (ใช้ตอน save) — คืน error แบบ ipc.md
pub fn validate_ui(config: &Value) -> Result<(), String> {
    let ui = config
        .get("ui")
        .and_then(Value::as_object)
        .ok_or_else(|| "ui: ต้องเป็น object".to_string())?;

    if let Some(value) = ui.get("hudTextScale") {
        let n = value
            .as_f64()
            .ok_or_else(|| "ui.hudTextScale: ต้องเป็น number".to_string())?;
        if !(TEXT_SCALE_MIN..=TEXT_SCALE_MAX).contains(&n) {
            return Err(format!(
                "ui.hudTextScale: ต้องอยู่ระหว่าง {TEXT_SCALE_MIN}–{TEXT_SCALE_MAX}"
            ));
        }
    }

    if let Some(value) = ui.get("hudFontFamily") {
        let name = value
            .as_str()
            .ok_or_else(|| "ui.hudFontFamily: ต้องเป็น string".to_string())?;
        if !FONTS.contains(&name) {
            return Err(format!(
                "ui.hudFontFamily: ต้องเป็นหนึ่งใน {}",
                FONTS.join(" | ")
            ));
        }
    }

    if let Some(value) = ui.get("hudScale") {
        let n = value
            .as_f64()
            .ok_or_else(|| "ui.hudScale: ต้องเป็น number".to_string())?;
        if !(HUD_SCALE_MIN..=HUD_SCALE_MAX).contains(&n) {
            return Err(format!(
                "ui.hudScale: ต้องอยู่ระหว่าง {HUD_SCALE_MIN}–{HUD_SCALE_MAX}"
            ));
        }
    }

    Ok(())
}

/// เขียน config ลง `%APPDATA%\RniType\config.json` (สร้างโฟลเดอร์ให้)
pub fn save(config: &Value) -> Result<(), String> {
    let path = config_path().ok_or_else(|| "ไม่พบ %APPDATA%".to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| format!("สร้าง {} ไม่ได้: {err}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    std::fs::write(&path, text).map_err(|err| format!("เขียน {} ไม่ได้: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale_of(style: &Value) -> f64 {
        style.get("textScale").and_then(Value::as_f64).unwrap()
    }

    #[test]
    fn ui_style_falls_back_to_defaults() {
        let style = ui_style(&json!({}));
        assert_eq!(scale_of(&style), TEXT_SCALE_DEFAULT);
        assert_eq!(style["font"], FONT_DEFAULT);
    }

    #[test]
    fn hud_scale_reads_and_clamps() {
        assert_eq!(hud_scale(&json!({})), HUD_SCALE_DEFAULT);
        assert_eq!(hud_scale(&json!({ "ui": { "hudScale": 1.6 } })), 1.6);
        assert_eq!(hud_scale(&json!({ "ui": { "hudScale": 9.0 } })), HUD_SCALE_DEFAULT);
    }

    #[test]
    fn ui_style_reads_config_values() {
        let style = ui_style(&json!({
            "ui": { "hudTextScale": 1.55, "hudFontFamily": "noto-sans-thai" }
        }));
        assert_eq!(scale_of(&style), 1.55);
        assert_eq!(style["font"], "noto-sans-thai");
    }

    #[test]
    fn ui_style_ignores_out_of_range_values() {
        let style = ui_style(&json!({
            "ui": { "hudTextScale": 5.0, "hudFontFamily": "comic-sans" }
        }));
        assert_eq!(scale_of(&style), TEXT_SCALE_DEFAULT);
        assert_eq!(style["font"], FONT_DEFAULT);
    }

    #[test]
    fn validate_ui_accepts_defaults() {
        assert!(validate_ui(&default_config()).is_ok());
    }

    #[test]
    fn validate_ui_rejects_bad_values() {
        assert!(validate_ui(&json!({ "ui": { "hudTextScale": 0.1 } })).is_err());
        assert!(validate_ui(&json!({ "ui": { "hudFontFamily": "wingdings" } })).is_err());
        assert!(validate_ui(&json!({ "ui": { "hudScale": 9 } })).is_err());
        assert!(validate_ui(&json!({})).is_err());
    }

    #[test]
    fn default_config_matches_contract_shape() {
        let config = default_config();
        for key in ["version", "defaults", "languages", "keymaps", "profiles", "ui"] {
            assert!(config.get(key).is_some(), "ขาด key {key}");
        }
        assert_eq!(config["version"], 1);
    }

    #[test]
    fn exclusion_list_reads_and_cleans_the_config() {
        assert!(exclusion_list(&json!({})).is_empty());
        let list = exclusion_list(&json!({
            "system": { "exclusion": [" game.exe ", "", "C:\\x\\tool.exe"] }
        }));
        assert_eq!(list, vec!["game.exe", "C:\\x\\tool.exe"]);
    }
}
