//! config.json — ส่วนที่แอปใช้จริงตอนนี้คือ `ui.*` (typography ของ HUD)
//!
//! path: `%APPDATA%\RniType\config.json` (ตรงกับ contract config.schema.json)
//! * ถ้าไฟล์ยังไม่มี → ใช้ config เริ่มต้น (schema-valid) แต่ **ยังไม่เขียนลงดิสก์**
//!   จนกว่าผู้ใช้จะแก้ค่าจริง (กันการไปสร้างไฟล์ทับของ P5)
//! * save = merge ลง object เดิม → key ที่ยังไม่รู้จักไม่หาย

use std::path::PathBuf;

use rnitype_win::language::LanguageInfo;
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

/// ภาษาที่เปิดได้สูงสุด (ADR-14 §2)
pub const MAX_ENABLED_LANGUAGES: usize = 8;

/// รวม layout ที่ Windows ตรวจพบกับ `config.languages` → แถวสำหรับ Settings
///
/// - `detected: true` = มี layout นี้ในเครื่อง · `false` = เคยเปิดไว้แต่ layout ถูกถอด
/// - `enabled` มาจาก config; ถ้า config ยังไม่มีรายการเลย → เปิด EN/TH ให้เป็นค่าเริ่มต้น
/// - `supported: false` = ยังไม่มีตาราง 8×8 (core มีแค่ EN/TH)
pub fn language_rows(config: &Value, detected: &[LanguageInfo]) -> Vec<Value> {
    let configured = config
        .get("languages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let enabled_of = |code: &str, klid: &str| -> bool {
        configured
            .iter()
            .find(|entry| {
                entry.get("code").and_then(Value::as_str) == Some(code)
                    || entry.get("klid").and_then(Value::as_str) == Some(klid)
            })
            .and_then(|entry| entry.get("enabled"))
            .and_then(Value::as_bool)
            // ไม่มี entry = ค่าเริ่มต้น: EN/TH เปิด (ตารางมีในแอป) · ภาษาอื่นปิด
            .unwrap_or_else(|| matches!(code, "en" | "th"))
    };

    let mut rows: Vec<Value> = detected
        .iter()
        .map(|info| {
            let enabled = enabled_of(&info.code, &info.klid);
            json!({
                "code": info.code,
                "klid": info.klid,
                "label": info.label,
                "kind": info.kind.as_str(),
                "detected": true,
                "builtin": false,
                "supported": info.typing_supported(),
                "enabled": enabled,
            })
        })
        .collect();

    // EN/TH มีตาราง 8×8 ในแอป — แสดงเสมอแม้ Windows ไม่ได้ติดตั้ง layout นั้น
    for (code, label) in [("en", "English"), ("th", "Thai")] {
        if rows.iter().any(|row| row.get("code").and_then(Value::as_str) == Some(code)) {
            continue;
        }
        rows.push(json!({
            "code": code,
            "klid": "",
            "label": label,
            "kind": "layout",
            "detected": false,
            "builtin": true,
            "supported": true,
            "enabled": enabled_of(code, ""),
        }));
    }

    // layout ที่ config เคยมีแต่เครื่องนี้ไม่มีแล้ว — แสดงไว้ให้ปิด/ลบ
    for entry in &configured {
        let code = entry.get("code").and_then(Value::as_str).unwrap_or("");
        if code.is_empty() || rows.iter().any(|row| row.get("code").and_then(Value::as_str) == Some(code)) {
            continue;
        }
        rows.push(json!({
            "code": code,
            "klid": entry.get("klid").and_then(Value::as_str).unwrap_or(""),
            "label": entry
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(code),
            "kind": entry.get("kind").and_then(Value::as_str).unwrap_or("layout"),
            "detected": false,
            "builtin": false,
            "supported": matches!(code, "en" | "th"),
            "enabled": entry.get("enabled").and_then(Value::as_bool).unwrap_or(false),
        }));
    }

    // EN/TH ขึ้นก่อนตามม็อกอัป แล้วภาษาที่เหลือคงลำดับของ Windows
    rows.sort_by_key(|row| {
        let code = row.get("code").and_then(Value::as_str).unwrap_or("");
        match code {
            "en" => 0,
            "th" => 1,
            _ => 2,
        }
    });

    // EN = base language ของแอป — เปิดตลอด ปิดไม่ได้ (ADR-14 §1)
    for row in &mut rows {
        let is_en = row.get("code").and_then(Value::as_str) == Some("en");
        row["locked"] = json!(is_en);
        if is_en {
            row["enabled"] = json!(true);
        }
    }
    rows
}

/// เปิด/ปิดภาษาใน `config.languages` (สร้าง entry ให้ถ้ายังไม่มี)
///
/// คืน `false` เมื่อจะเปิดเกิน [`MAX_ENABLED_LANGUAGES`] — caller ตอบ error
pub fn set_language_enabled(config: &mut Value, code: &str, enabled: bool) -> bool {
    if !config.is_object() {
        return false;
    }
    if config.get("languages").and_then(Value::as_array).is_none() {
        config["languages"] = json!([]);
    }
    let list = config["languages"].as_array_mut().expect("array just set");

    let enabled_now = list
        .iter()
        .filter(|entry| entry.get("enabled").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let already_on = list
        .iter()
        .any(|entry| {
            entry.get("code").and_then(Value::as_str) == Some(code)
                && entry.get("enabled").and_then(Value::as_bool).unwrap_or(false)
        });
    if enabled && !already_on && enabled_now >= MAX_ENABLED_LANGUAGES {
        return false;
    }

    if let Some(entry) = list
        .iter_mut()
        .find(|entry| entry.get("code").and_then(Value::as_str) == Some(code))
    {
        entry["enabled"] = json!(enabled);
    } else {
        list.push(json!({ "code": code, "enabled": enabled }));
    }
    true
}

/// ตรวจ `languages` — เปิดเกิน 8 / code ว่าง = error แบบ ipc.md
pub fn validate_languages(config: &Value) -> Result<(), String> {
    let list = config
        .get("languages")
        .and_then(Value::as_array)
        .ok_or_else(|| "languages: ต้องเป็น array".to_string())?;
    let mut enabled = 0usize;
    for entry in list {
        let code = entry
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(|| "languages[].code: ต้องเป็น string".to_string())?;
        if code.trim().is_empty() {
            return Err("languages[].code: ห้ามว่าง".to_string());
        }
        if entry
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            enabled += 1;
        }
    }
    if enabled > MAX_ENABLED_LANGUAGES {
        return Err(format!(
            "languages: เปิดได้สูงสุด {MAX_ENABLED_LANGUAGES} ภาษา"
        ));
    }
    Ok(())
}

/// รีเซ็ตเฉพาะส่วน (ปุ่ม Reset this page / Reset everything)
///
/// - `hud` = ขนาด/ฟอนต์/ความเข้มของ HUD
/// - `general` = `defaults.*` + autostart/runAsAdmin (เก็บ exclusion list ไว้)
/// - `languages` = กลับไปใช้ค่าเริ่มต้น EN/TH
/// - `all` = ทั้งไฟล์
///
/// คืน `false` เมื่อชื่อส่วนไม่รู้จัก (caller ตอบ error)
pub fn reset_section(config: &mut Value, section: &str) -> bool {
    if section == "all" {
        *config = default_config();
        return true;
    }
    if !config.is_object() {
        *config = default_config();
    }
    match section {
        "hud" => {
            let defaults = default_config();
            let default_ui = defaults.get("ui").cloned().unwrap_or_else(|| json!({}));
            let ui = config
                .as_object_mut()
                .expect("object")
                .entry("ui")
                .or_insert_with(|| json!({}));
            if !ui.is_object() {
                *ui = json!({});
            }
            for key in ["hudScale", "hudTextScale", "hudFontFamily", "hudOpacity"] {
                if let Some(value) = default_ui.get(key) {
                    ui[key] = value.clone();
                }
            }
            true
        }
        "general" => {
            let defaults = default_config();
            let root = config.as_object_mut().expect("object");
            root.insert(
                "defaults".to_string(),
                defaults.get("defaults").cloned().unwrap_or_else(|| json!({})),
            );
            let system = root.entry("system").or_insert_with(|| json!({}));
            if !system.is_object() {
                *system = json!({});
            }
            system["autostart"] = json!(false);
            system["runAsAdmin"] = json!(false);
            true
        }
        "languages" => {
            config.as_object_mut().expect("object")["languages"] = json!([]);
            true
        }
        _ => false,
    }
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
    use rnitype_win::language::LanguageKind;

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

    /* ---------------- P5 slice 2: languages / reset ---------------- */

    fn detected(code: &str, klid: &str, kind: LanguageKind) -> LanguageInfo {
        LanguageInfo {
            code: code.to_string(),
            klid: klid.to_string(),
            label: format!("{code}-label"),
            kind,
        }
    }

    fn sample_detected() -> Vec<LanguageInfo> {
        vec![
            detected("th", "0000041E", LanguageKind::Layout),
            detected("en", "00000409", LanguageKind::Layout),
            detected("ja", "00000411", LanguageKind::Ime),
        ]
    }

    #[test]
    fn language_rows_default_to_en_and_th_enabled() {
        let rows = language_rows(&json!({}), &sample_detected());
        let codes: Vec<&str> = rows
            .iter()
            .map(|row| row["code"].as_str().unwrap())
            .collect();
        assert_eq!(codes, vec!["en", "th", "ja"], "EN/TH ต้องขึ้นก่อน");
        assert_eq!(rows[0]["enabled"], true);
        assert_eq!(rows[1]["enabled"], true);
        assert_eq!(rows[2]["enabled"], false);
        assert_eq!(rows[2]["kind"], "ime");
        assert_eq!(rows[2]["supported"], false);
        assert!(rows.iter().all(|row| row["detected"] == true));
    }

    #[test]
    fn language_rows_always_include_builtin_en_and_th() {
        // เครื่องนี้มีแค่ EN layout — TH ยังต้องแสดงเพราะมีตารางในแอป
        let detected = vec![detected("en", "00000409", LanguageKind::Layout)];
        let rows = language_rows(&json!({}), &detected);
        let th = rows.iter().find(|row| row["code"] == "th").unwrap();
        assert_eq!(th["builtin"], true);
        assert_eq!(th["detected"], false);
        assert_eq!(th["supported"], true);
        assert_eq!(th["enabled"], true, "ค่าเริ่มต้นเปิด EN/TH");
    }

    #[test]
    fn language_rows_respect_config_and_show_removed_layouts() {
        let config = json!({
            "languages": [
                { "code": "en", "enabled": false },
                { "code": "ru", "label": "Russian", "kind": "layout", "enabled": true }
            ]
        });
        let rows = language_rows(&config, &sample_detected());
        let en = rows.iter().find(|row| row["code"] == "en").unwrap();
        assert_eq!(en["enabled"], true, "EN เป็น base — config ปิดไม่ได้");
        assert_eq!(en["locked"], true);
        let ru = rows.iter().find(|row| row["code"] == "ru").unwrap();
        assert_eq!(ru["detected"], false);
        assert_eq!(ru["label"], "Russian");
        assert_eq!(ru["enabled"], true);
        // TH ไม่มีใน config → เปิดตามค่าเริ่มต้น (มีตารางในแอป)
        let th = rows.iter().find(|row| row["code"] == "th").unwrap();
        assert_eq!(th["enabled"], true);
    }

    #[test]
    fn set_language_enabled_creates_and_updates_entries() {
        let mut config = default_config();
        assert!(set_language_enabled(&mut config, "th", true));
        assert_eq!(config["languages"][0]["code"], "th");
        assert_eq!(config["languages"][0]["enabled"], true);

        assert!(set_language_enabled(&mut config, "th", false));
        assert_eq!(config["languages"][0]["enabled"], false);
    }

    #[test]
    fn set_language_enabled_stops_at_eight() {
        let mut config = default_config();
        for index in 0..MAX_ENABLED_LANGUAGES {
            assert!(set_language_enabled(&mut config, &format!("x{index}"), true));
        }
        assert!(!set_language_enabled(&mut config, "overflow", true));
        // ปิดซ้ำ/เปิดตัวที่มีอยู่แล้วไม่นับเพิ่ม
        assert!(set_language_enabled(&mut config, "x0", true));
    }

    #[test]
    fn validate_languages_checks_codes_and_the_cap() {
        assert!(validate_languages(&default_config()).is_ok());
        assert!(validate_languages(&json!({ "languages": [ { "code": "" } ] })).is_err());
        assert!(validate_languages(&json!({ "languages": [ { "code": 7 } ] })).is_err());
        let mut too_many = default_config();
        let list: Vec<Value> = (0..=MAX_ENABLED_LANGUAGES)
            .map(|index| json!({ "code": format!("x{index}"), "enabled": true }))
            .collect();
        too_many["languages"] = json!(list);
        assert!(validate_languages(&too_many).is_err());
    }

    #[test]
    fn reset_section_touches_only_its_own_part() {
        let mut config = default_config();
        config["ui"]["hudTextScale"] = json!(2.0);
        config["ui"]["hudFontFamily"] = json!("system");
        config["ui"]["language"] = json!("th");
        config["system"]["exclusion"] = json!(["game.exe"]);
        config["defaults"]["typing"]["commitDwellMs"] = json!(120);
        config["languages"] = json!([{ "code": "th", "enabled": true }]);

        assert!(reset_section(&mut config, "hud"));
        assert_eq!(config["ui"]["hudTextScale"], TEXT_SCALE_DEFAULT);
        assert_eq!(config["ui"]["hudFontFamily"], FONT_DEFAULT);
        assert_eq!(config["ui"]["language"], "th", "ไม่แตะภาษาของ UI");
        assert_eq!(config["languages"][0]["code"], "th");

        assert!(reset_section(&mut config, "general"));
        assert_eq!(config["defaults"]["typing"]["commitDwellMs"], 60);
        assert_eq!(
            config["system"]["exclusion"],
            json!(["game.exe"]),
            "general ไม่ล้าง exclusion list"
        );

        assert!(reset_section(&mut config, "languages"));
        assert_eq!(config["languages"], json!([]));

        assert!(reset_section(&mut config, "all"));
        assert_eq!(config, default_config());
        assert!(!reset_section(&mut config, "nope"));
    }
}
