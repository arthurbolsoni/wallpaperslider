use rand::Rng;
use rand::thread_rng;
use regex::Regex;

use reqwest;
use reqwest::Error;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn last_wallpaper_wide(screen_size: &str) -> Result<String, Error> {
    let body = reqwest::get("https://wallpaperswide.com/rss/2560x1080-ds_wallpapers-r-random-p-1")
        .await?
        .text()
        .await?;

    let re_link = Regex::new(r#"<link>(.*?)</link>"#).unwrap();
    let links = re_link
        .captures_iter(&body)
        .map(|x| x[1].to_string())
        .collect::<Vec<_>>();

    let random_int = thread_rng().gen_range(0..links.len());

    let link = links[random_int].clone();
    
    let re_image_name = Regex::new(r"https://wallpaperswide.com/(.*?)-wallpapers.html").unwrap();
    let image_name = re_image_name
        .captures_iter(&link)
        .map(|x| x[1].to_string())
        .collect::<Vec<_>>()[0]
        .to_string();

    Ok(format!(
        "https://wallpaperswide.com/download/{}-{}.html",
        image_name, screen_size
    ))
}

pub async fn download_wallpaper(download_url: &str, download_path: &str) -> String {
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