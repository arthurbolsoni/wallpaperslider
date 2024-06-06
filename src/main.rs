#![windows_subsystem = "windows"]

use home;
use reqwest;
use std::{fs, time::Duration};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::interval;

use trayicon::{Icon, MenuBuilder, MenuItem, TrayIcon, TrayIconBuilder};

use core::mem::MaybeUninit;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::um::winuser;
use winreg::enums::*;
use winreg::RegKey;

use open;

use regex::Regex;

async fn last_wallpaper_wide(screen_size: &str) -> String {
    let body = reqwest::get("https://wallpaperswide.com/rss/2560x1080-ds_wallpapers-r-random-p-1")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let re_link = Regex::new(r#"<link>(.*?)</link>"#).unwrap();
    let link = re_link
        .captures_iter(&body)
        .map(|x| x[1].to_string())
        .collect::<Vec<_>>()[1]
        .to_string();

    println!("{:?}", link);

    let re_image_name = Regex::new(r"http://wallpaperswide.com/(.*?)-wallpapers.html").unwrap();
    let image_name = re_image_name
        .captures_iter(&link)
        .map(|x| x[1].to_string())
        .collect::<Vec<_>>()[0]
        .to_string();

    return format!(
        "https://wallpaperswide.com/download/{}-{}.html",
        image_name, screen_size
    );
}

async fn download_wallpaper(download_url: &str, download_path: &str) -> String {
    let client = reqwest::Client::new();
    let response = match client.get(download_url).send().await {
        Ok(resp) => resp,
        Err(err) => return format!("Erro ao enviar a requisição: {:?}", err),
    };

    if !response.status().is_success() {
        return format!("Falha ao baixar o wallpaper: {:?}", response.status());
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return String::from("Erro ao ler o stream de bytes."),
    };

    let file_name = download_url
        .split("/")
        .last()
        .unwrap()
        .split(".")
        .next()
        .unwrap();

    let mut out = match File::create(format!("{}/{}.jpg", download_path, file_name)).await {
        Ok(file) => file,
        Err(err) => return format!("Erro ao criar o arquivo: {:?}", err),
    };

    if let Err(err) = out.write_all(&bytes).await {
        return format!("Erro ao escrever no arquivo: {:?}", err);
    }

    // save as bmp
    let mut out = match File::create(format!("{}/wallpaper.bmp", download_path)).await {
        Ok(file) => file,
        Err(err) => return format!("Erro ao criar o arquivo: {:?}", err),
    };

    if let Err(err) = out.write_all(&bytes).await {
        return format!("Erro ao escrever no arquivo: {:?}", err);
    }

    return format!("{}\\wallpaper.bmp", download_path);
}

pub enum Mode {
    Center,
    Tile,
    Stretch,
    Fill,
    Fit,
    Span,
    Crop,
}

async fn set_wallpaper_from_path(path: &str, mode: Mode) {
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
            let style = match mode {
                Mode::Center => "0",
                Mode::Tile => "0",
                Mode::Stretch => "2",
                Mode::Fill => "10",
                Mode::Fit => "6",
                Mode::Span => "22",
                Mode::Crop => "2",
            };

            if let Err(e) = desktop_key.set_value("WallpaperStyle", &style) {
                eprintln!("Erro ao definir o valor do registro: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Erro ao abrir a chave do registro: {}", e);
        }
    }
}

async fn change_wallpaper() {
    println!("Changing wallpaper...");

    let screen_size = "2560x1080";

    let _last_wallpaper_wide = last_wallpaper_wide(screen_size).await;

    let download_path = home::home_dir()
        .unwrap()
        .join("Pictures")
        .join("WallpapersSlider");

    // Verifica se o diretório existe
    if !download_path.exists() {
        // Tenta criar o diretório e seus pais
        match fs::create_dir_all(&download_path) {
            Ok(_) => println!("Diretório criado: {:?}", download_path),
            Err(e) => eprintln!("Erro ao criar diretório: {}", e),
        }
    }

    let wallpaper_path =
        download_wallpaper(&_last_wallpaper_wide, &download_path.to_str().unwrap()).await;

    set_wallpaper_from_path(&wallpaper_path, Mode::Crop).await;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Events {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    ChangeWallpaper,
    DownloadFolder,
}

#[tokio::main]
async fn main() {
    let mut interval = interval(Duration::from_secs(60 * 60 * 1));

    let (s, r) = std::sync::mpsc::channel::<Events>();
    let second_icon = Icon::from_buffer(include_bytes!("../icon.ico"), None, None).unwrap();
    let first_icon = Icon::from_buffer(include_bytes!("../icon.ico"), None, None).unwrap();

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |e: &Events| {
            let _ = s.send(*e);
        })
        .icon_from_buffer(include_bytes!("../icon.ico"))
        .tooltip("Wallpapers Slider")
        .on_click(Events::LeftClickTrayIcon)
        .on_right_click(Events::RightClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("Change Wallpaper", Events::ChangeWallpaper)
                .separator()
                .item("Download Folder", Events::DownloadFolder)
                .item("Exit", Events::Exit),
        )
        .build()
        .unwrap();

    tokio::task::spawn(async move {
        println!("Starting...");
        change_wallpaper().await;

        r.iter().for_each(|m| match m {
            Events::RightClickTrayIcon => {
                tray_icon.show_menu().unwrap();
            }
            Events::LeftClickTrayIcon => {
                tokio::task::spawn(async move {
                    change_wallpaper().await;
                });
            }
            Events::Exit => {
                println!("Please exit");
                std::process::exit(0);
            }
            Events::ChangeWallpaper => {
                tokio::task::spawn(async move {
                    change_wallpaper().await;
                });
            }
            Events::DownloadFolder => {
                let download_path = home::home_dir()
                    .unwrap()
                    .join("Pictures")
                    .join("WallpapersSlider");

                if !download_path.exists() {
                    match fs::create_dir_all(&download_path) {
                        Ok(_) => println!("Diretório criado: {:?}", download_path),
                        Err(e) => eprintln!("Erro ao criar diretório: {}", e),
                    }
                }

                match open::that(&download_path) {
                    Ok(_) => println!("Diretório aberto: {:?}", download_path),
                    Err(e) => eprintln!("Erro ao abrir diretório: {}", e),
                }
            }
            e => {
                println!("{:?}", e);
            }
        });
    });

    loop {
        unsafe {
            let mut msg = MaybeUninit::uninit();
            let bret = winuser::GetMessageA(msg.as_mut_ptr(), 0 as _, 0, 0);
            if bret > 0 {
                winuser::TranslateMessage(msg.as_ptr());
                winuser::DispatchMessageA(msg.as_ptr());
            } else {
                break;
            }
        }
    }

    // loop {
    //     interval.tick().await;
    //     change_wallpaper().await;
    // }
}
