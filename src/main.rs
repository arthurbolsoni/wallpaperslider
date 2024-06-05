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
use winapi::um::winuser;

use wallpaper;

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

    return format!("{}/{}.jpg", download_path, file_name);
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

    tokio::task::spawn_blocking(move || {
        wallpaper::set_from_path(&wallpaper_path).unwrap();
        wallpaper::set_mode(wallpaper::Mode::Crop).unwrap();
    });
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Events {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    ChangeWallpaper,
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
