use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winreg::enums::*;
use winreg::RegKey;

// pub enum Mode {
//     Center,
//     Tile,
//     Stretch,
//     Fill,
//     Fit,
//     Span,
//     Crop,
// }

pub async fn set_wallpaper_from_path(path: &str) {
    unsafe {
        let path = OsStr::new(path)
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect::<Vec<_>>();

        let result = winapi::um::winuser::SystemParametersInfoW(
            winapi::um::winuser::SPI_SETDESKWALLPAPER,
            0,
            path.as_ptr() as *mut _,
            winapi::um::winuser::SPIF_UPDATEINIFILE | winapi::um::winuser::SPIF_SENDCHANGE,
        );

        if result == 0 {
            eprintln!(
                "Erro ao setar o wallpaper: {}",
                std::io::Error::last_os_error()
            );
        }
    }

    // Atualiza o registro do Windows
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path_subkey = "Control Panel\\Desktop";
    match hkcu.open_subkey_with_flags(path_subkey, KEY_WRITE) {
        Ok(desktop_key) => {
            if let Err(e) = desktop_key.set_value("Wallpaper", &path) {
                eprintln!("Erro ao definir o valor do registro: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Erro ao abrir a chave do registro: {}", e);
        }
    }

    // Wallpaper Style
    match hkcu.open_subkey_with_flags(path_subkey, KEY_WRITE) {
        Ok(desktop_key) => {
            let style = "0";
            // let style = match mode {
            //     Mode::Center => "0",
            //     Mode::Tile => "0",
            //     Mode::Stretch => "2",
            //     Mode::Fill => "10",
            //     Mode::Fit => "6",
            //     Mode::Span => "22",
            //     Mode::Crop => "2",
            // };

            if let Err(e) = desktop_key.set_value("WallpaperStyle", &style) {
                eprintln!("Erro ao definir o valor do registro: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Erro ao abrir a chave do registro: {}", e);
        }
    }
}
