#![windows_subsystem = "windows"]

use home;
use reqwest;
use std::error::Error;
use std::{fs, time::Duration};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::interval;
use tray_item::{IconSource, TrayItem};

use winapi::um::wincon::GetConsoleWindow;
use winapi::um::winuser::{ShowWindow, SW_HIDE, SW_MINIMIZE};

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

    let mut out = match File::create(format!("{}/wallpaper.jpg", download_path)).await {
        Ok(file) => file,
        Err(err) => return format!("Erro ao criar o arquivo: {:?}", err),
    };

    if let Err(err) = out.write_all(&bytes).await {
        return format!("Erro ao escrever no arquivo: {:?}", err);
    }

    return format!("{}/wallpaper.jpg", download_path);
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
    } else {
        println!("Diretório já existe: {:?}", download_path);
    }

    let wallpaper_path =
        download_wallpaper(&_last_wallpaper_wide, &download_path.to_str().unwrap()).await;

    tokio::task::spawn_blocking(move || {
        wallpaper::set_from_path(&wallpaper_path).unwrap();
        wallpaper::set_mode(wallpaper::Mode::Crop).unwrap();
    });
}

async fn setup_tray_icon(tx: mpsc::Sender<i32>) -> Result<TrayItem, Box<dyn Error>> {
    let mut tray = TrayItem::new("Wallpapers Slider", IconSource::Resource("id"))?;

    tray.add_label("Wallpapers Slider").unwrap();

    let tx_1 = tx.clone();
    tray.add_menu_item("Change Wallpaper", move || {
        tx_1.try_send(1).unwrap();
    })
    .unwrap();

    let tx_2 = tx.clone();
    tray.add_menu_item("Exit", move || {
        tx_2.try_send(2).unwrap();
    })
    .unwrap();

    Ok(tray)
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(1);

    let tray = setup_tray_icon(tx).await.unwrap();

    task::spawn(async move {
        while let Some(message) = rx.recv().await {
            match message {
                1 => {
                    change_wallpaper().await;
                }
                2 => {
                    std::process::exit(0);
                }
                _ => {
                    println!("Mensagem desconhecida: {}", message);
                }
            }
        }
    });

    let mut interval = interval(Duration::from_secs(60 * 60 * 1));

    unsafe {
        let console_window = GetConsoleWindow();
            // ShowWindow(console_window, SW_MINIMIZE);
            // ShowWindow(console_window, SW_HIDE);

    }

    loop {
        interval.tick().await;
        change_wallpaper().await;
    }
}
