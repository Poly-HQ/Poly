use std::borrow::Cow;

use anyhow::Result;
use gpui::{App, AssetSource, SharedString};

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

pub struct Assets;

impl Assets {
    pub fn load_fonts(&self, cx: &App) -> Result<()> {
        #[cfg(windows)]
        ensure_windows_user_fonts();

        let embedded = FONTS
            .iter()
            .map(|(_, bytes)| Cow::Borrowed(*bytes))
            .collect();

        cx.text_system().add_fonts(embedded)
    }
}

#[cfg(windows)]
fn ensure_windows_user_fonts() {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_WRITE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegSetValueExW,
    };
    use windows::core::PCWSTR;

    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let fonts_dir = std::path::PathBuf::from(local_app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Fonts");
    let _ = std::fs::create_dir_all(&fonts_dir);

    let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts\0"
        .encode_utf16()
        .collect();
    let mut hkey = Default::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            None,
            None,
            Default::default(),
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if status.is_err() {
        return;
    }

    for (name, bytes) in FONTS {
        if !name.contains("NotoSansThai") {
            continue;
        }
        let file_name = std::path::Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name);
        let dest = fonts_dir.join(file_name);
        if !dest.exists() {
            let _ = std::fs::write(&dest, bytes);
        }
        let reg_name = match file_name {
            "NotoSansThai-Regular.ttf" => "Noto Sans Thai Regular (TrueType)",
            "NotoSansThai-Medium.ttf" => "Noto Sans Thai Medium (TrueType)",
            "NotoSansThai-SemiBold.ttf" => "Noto Sans Thai SemiBold (TrueType)",
            "NotoSansThai-Bold.ttf" => "Noto Sans Thai Bold (TrueType)",
            _ => continue,
        };
        let mut reg_key_wide: Vec<u16> = reg_name.encode_utf16().collect();
        reg_key_wide.push(0);
        let mut dest_wide: Vec<u16> = dest.as_os_str().encode_wide().collect();
        dest_wide.push(0);
        let val_bytes = unsafe {
            std::slice::from_raw_parts(
                dest_wide.as_ptr() as *const u8,
                dest_wide.len() * std::mem::size_of::<u16>(),
            )
        };
        unsafe {
            let _ = RegSetValueExW(
                hkey,
                PCWSTR(reg_key_wide.as_ptr()),
                None,
                REG_SZ,
                Some(val_bytes),
            );
        }
    }
    unsafe {
        let _ = RegCloseKey(hkey);
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = icons::asset(path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        if let Some((_, bytes)) = FONTS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        log::warn!("assets: {path} is not registered");
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut listed = icons::Assets.list(path)?;
        listed.extend(
            FONTS
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| SharedString::new_static(name)),
        );
        Ok(listed)
    }
}
