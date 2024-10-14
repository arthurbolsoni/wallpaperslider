// #![windows_subsystem = "windows"]

use trayicon::{Icon, MenuBuilder, TrayIconBuilder};

use core::mem::MaybeUninit;

use winapi::um::winuser;

use open;

mod services;
use services::helper;
use services::wallpaper_wide;
use services::windows_wallpaper;

async fn change_wallpaper(download_path: String) {
    println!("Changing wallpaper...");

    let screen_size = "2560x1080";

    let _last_wallpaper_wide = wallpaper_wide::last_wallpaper_wide(screen_size)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Erro ao obter o último wallpaper: {}", e);
            String::from("")
        });
    if _last_wallpaper_wide == "" {
        return;
    }

    let wallpaper_path: String =
        wallpaper_wide::download_wallpaper(&_last_wallpaper_wide, &download_path).await;

    windows_wallpaper::set_wallpaper_from_path(&wallpaper_path).await;

    println!("Wallpaper set: {}", _last_wallpaper_wide);
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Events {
    RightClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    ChangeWallpaper,
    DownloadFolder,
}

#[tokio::main]
async fn main() {
    print!("Starting...");

    let (s, r) = std::sync::mpsc::channel::<Events>();

    let icon = Icon::from_buffer(include_bytes!("../icon.ico"), None, None).unwrap();
    let icon_loading =
        Icon::from_buffer(include_bytes!("../icon_loading.ico"), None, None).unwrap();

    println!("Tray Icon");

    let mut tray_icon = TrayIconBuilder::new()
        .sender(move |e: &Events| {
            let _ = s.send(*e);
        })
        .icon(icon.clone())
        .tooltip("Wallpapers Slider")
        .on_right_click(Events::RightClickTrayIcon)
        .on_double_click(Events::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("Change Wallpaper", Events::ChangeWallpaper)
                .separator()
                .item("Download Folder", Events::DownloadFolder)
                .item("Exit", Events::Exit),
        )
        .build()
        .unwrap();

    println!("Event Loop");

    tokio::task::spawn(async move {
        println!("Starting...");

        let download_path = helper::user_images_folder();
        println!("Download Path: {}", download_path);

        change_wallpaper(download_path.clone()).await;

        while let Ok(event) = r.recv() {
            println!("Event");
            match event {
                Events::DoubleClickTrayIcon => {
                    println!("Double Click Tray Icon");
                    tray_icon.set_icon(&icon_loading).unwrap();

                    change_wallpaper(download_path.clone()).await;

                    tray_icon.set_icon(&icon).unwrap();
                }
                Events::RightClickTrayIcon => {
                    tray_icon.show_menu().unwrap();
                }
                Events::Exit => {
                    println!("Exit");
                    std::process::exit(0);
                }
                Events::ChangeWallpaper => {
                    println!("Change Wallpaper");
                    tray_icon.set_icon(&icon_loading).unwrap();

                    change_wallpaper(download_path.clone()).await;
                    
                    tray_icon.set_icon(&icon).unwrap();
                }
                Events::DownloadFolder => {
                    println!("Download Folder");
                    match open::that(&download_path) {
                        Ok(_) => println!("Diretório aberto: {:?}", &download_path),
                        Err(e) => eprintln!("Erro ao abrir diretório: {}", e),
                    }
                }
            }
        }
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
}
