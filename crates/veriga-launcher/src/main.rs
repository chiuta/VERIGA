//! `VERIGAPortable.exe` — lansator conform PortableApps.com Format 3.9.
//!
//! Structura așteptată (relativ la folderul lansatorului):
//! `App\VERIGA\veriga-gui.exe` (x64), `App\VERIGAARM64\veriga-gui.exe` (ARM64),
//! `App\DefaultData\` (copiat în `Data\` dacă lipsește), `Data\` (setări și stare Guardian).
//!
//! Garanții de portabilitate: nu citește și nu scrie registrul, nu scrie în `%APPDATA%`;
//! directorul curent al aplicației este `Data\`, deci starea Guardian (`.veriga`) rămâne pe unitatea portabilă.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_X64: &str = "VERIGA";
const APP_ARM64: &str = "VERIGAARM64";
const EXE: &str = "veriga-gui.exe";
const DATA_DIRS: [&str; 3] = ["reports", "sessions", "guardian"];

/// Alege subfolderul aplicației după arhitectura *nativă* a Windows
/// (un lansator x64 poate rula emulat pe ARM64, așa că `PROCESSOR_ARCHITEW6432` are prioritate).
fn app_dir_name(arch: Option<&str>, wow_arch: Option<&str>) -> &'static str {
    let native = wow_arch.or(arch).unwrap_or("");
    if native.eq_ignore_ascii_case("ARM64") {
        APP_ARM64
    } else {
        APP_X64
    }
}

/// Copiază recursiv din `src` în `dst` doar fișierele care lipsesc (nu suprascrie niciodată datele utilizatorului).
fn copy_missing(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        let ty = e.file_type()?;
        if ty.is_symlink() {
            continue;
        }
        if ty.is_dir() {
            copy_missing(&e.path(), &to)?;
        } else if !to.exists() {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Pregătește `Data\`: creează folderele și copiază din `App\DefaultData` ce lipsește.
fn prepare_data(root: &Path) -> std::io::Result<PathBuf> {
    let data = root.join("Data");
    std::fs::create_dir_all(&data)?;
    for d in DATA_DIRS {
        std::fs::create_dir_all(data.join(d))?;
    }
    copy_missing(&root.join("App").join("DefaultData"), &data)?;
    Ok(data)
}

fn run(root: &Path, args: Vec<OsString>) -> Result<i32, String> {
    let arch = std::env::var("PROCESSOR_ARCHITECTURE").ok();
    let wow = std::env::var("PROCESSOR_ARCHITEW6432").ok();
    let app = root
        .join("App")
        .join(app_dir_name(arch.as_deref(), wow.as_deref()))
        .join(EXE);
    if !app.is_file() {
        return Err(format!(
            "Nu găsesc {} · {} not found",
            app.display(),
            app.display()
        ));
    }
    let data = prepare_data(root).map_err(|e| format!("Data\\: {e}"))?;
    let status = Command::new(&app)
        .args(args)
        .current_dir(&data)
        .env("VERIGA_PORTABLE", "1")
        .env("VERIGA_DATA", &data)
        .status()
        .map_err(|e| format!("{}: {e}", app.display()))?;
    Ok(status.code().unwrap_or(0))
}

#[cfg(target_os = "windows")]
fn show_error(msg: &str) {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    let w = |s: &str| {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let (t, c) = (w(msg), w("VERIGA Portable"));
    // SAFETY: pointerii arată către șiruri UTF-16 terminate cu NUL, valide pe durata apelului.
    unsafe { MessageBoxW(core::ptr::null_mut(), t.as_ptr(), c.as_ptr(), 0x10) };
}

#[cfg(not(target_os = "windows"))]
fn show_error(msg: &str) {
    eprintln!("VERIGA Portable: {msg}");
}

fn main() {
    let root = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    match run(&root, std::env::args_os().skip(1).collect()) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            show_error(&e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_native_architecture() {
        assert_eq!(app_dir_name(Some("AMD64"), None), APP_X64);
        assert_eq!(app_dir_name(Some("ARM64"), None), APP_ARM64);
        // lansator x64 emulat pe ARM64
        assert_eq!(app_dir_name(Some("AMD64"), Some("ARM64")), APP_ARM64);
        assert_eq!(app_dir_name(None, None), APP_X64);
    }

    #[test]
    fn default_data_never_overwrites_user_data() {
        let root = std::env::temp_dir().join(format!("veriga-launch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let def = root.join("App").join("DefaultData").join("guardian");
        std::fs::create_dir_all(&def).unwrap();
        std::fs::write(def.join("a.json"), "default").unwrap();
        std::fs::write(def.join("b.json"), "default").unwrap();
        std::fs::create_dir_all(root.join("Data").join("guardian")).unwrap();
        std::fs::write(root.join("Data").join("guardian").join("a.json"), "user").unwrap();
        let data = prepare_data(&root).unwrap();
        assert_eq!(
            std::fs::read_to_string(data.join("guardian/a.json")).unwrap(),
            "user"
        );
        assert_eq!(
            std::fs::read_to_string(data.join("guardian/b.json")).unwrap(),
            "default"
        );
        assert!(data.join("reports").is_dir() && data.join("sessions").is_dir());
        // Data\ se recreează dacă lipsește
        std::fs::remove_dir_all(&data).unwrap();
        assert!(
            prepare_data(&root)
                .unwrap()
                .join("guardian/b.json")
                .is_file()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
