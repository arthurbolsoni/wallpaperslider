use home;
use std::fs;

pub fn user_images_folder() -> String {
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

    download_path.to_str().unwrap().to_string()
}
