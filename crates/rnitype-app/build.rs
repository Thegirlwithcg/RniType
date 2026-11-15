//! Tauri build script
//!
//! Workaround: the vault path contains an apostrophe (`Erin's Works`), and
//! Windows RC.EXE truncates the icon path at `'` (RC2135). Copy the icon to
//! `%TEMP%` (path without an apostrophe) and point Tauri at that copy.
//!
//! This keeps `cargo test --workspace` / `cargo check` working without the
//! `run.ps1` wrapper. The wrapper is still useful for dev runs.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("icons").join("icon.ico");
    let temp = std::env::temp_dir().join("rnitype-app-build-icon.ico");

    if let Err(err) = std::fs::copy(&source, &temp) {
        panic!(
            "copy icon {} -> {} failed: {err}",
            source.display(),
            temp.display()
        );
    }

    let icon = temp.to_string_lossy().replace('\\', "/");
    let config = format!(r#"{{"bundle":{{"icon":["{icon}"]}}}}"#);
    // SAFETY: build scripts are single-threaded here; this only affects the
    // `tauri_build::build()` call below.
    unsafe { std::env::set_var("TAURI_CONFIG", config) };

    tauri_build::build()
}
