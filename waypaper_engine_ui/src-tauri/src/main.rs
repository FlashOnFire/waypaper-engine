use linux_ipc::IpcChannel;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State, Window};
use waypaper_engine_shared::project::{WEProject, WallpaperType};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use waypaper_engine_shared::ipc::IPCRequest;

#[tauri::command]
fn set_wp(wp_id: u64, screen: String, channel: State<Mutex<IpcChannel>>) {
    println!("set wp {:?} {}", wp_id, screen);
    //let response = channel.lock().unwrap().send::<_, String>(format!("setWP {:?} {}", wp_id, screen)).expect("Failed to send message");
    let response = channel
        .lock()
        .unwrap()
        .send::<_, IPCRequest>(IPCRequest::SetWP { id: wp_id, screen })
        .expect("Failed to send message");
    
    if let Some(response) = response {
        println!("Received: {:#?}", response);
    }
}

#[tauri::command]
fn apply_filter(window: Window, search: String, wallpaper_infos: State<Mutex<Vec<WPInfo>>>) {
    let wallpaper_infos = wallpaper_infos.lock().unwrap();

    if search.is_empty() {
        window.emit("setWPs", wallpaper_infos.deref()).unwrap();
    } else {
        let search = search.to_lowercase();
        let mut clone = wallpaper_infos.clone();
        clone.retain(|wp| wp.title.to_lowercase().contains(&search));
        window.emit("setWPs", clone).unwrap();
    }
}

#[derive(Clone, Serialize)]
struct WPInfo {
    title: String,
    id: u64,
    preview_b64: String,
}

#[tauri::command]
fn loaded(
    window: Window,
    wallpapers: State<Mutex<Vec<WEProject>>>,
    wallpaper_infos: State<Mutex<Vec<WPInfo>>>,
) {
    let mut wallpapers = wallpapers.lock().unwrap();

    for entry in fs::read_dir(WP_DIR).unwrap().flatten() {
        if let Ok(aa) = File::open(entry.path().join("project.json")) {
            if let Ok(mut project) = serde_json::from_reader::<File, WEProject>(aa) {
                if project.workshop_id.is_none() {
                    project.workshop_id = entry.file_name().to_str().and_then(|s| s.parse().ok());
                }

                wallpapers.push(project);
            }
        }
    }

    wallpapers.retain(|wp| wp.wallpaper_type == WallpaperType::Video);

    let mut wallpaper_infos = wallpaper_infos.lock().unwrap();
    wallpapers
        .iter()
        .map(|project| {
            let id = project.workshop_id.unwrap();

            let preview_path = PathBuf::from(WP_DIR)
                .join(id.to_string())
                .join(&project.preview);

            let b64 = to_base64(&preview_path);

            let title = project.title.clone();

            WPInfo {
                id,
                title,
                preview_b64: b64,
            }
        })
        .for_each(|wp| wallpaper_infos.push(wp));

    window.emit("setWPs", wallpaper_infos.deref()).unwrap();
}

const WP_DIR: &str = "/home/flashonfire/.steam/steam/steamapps/workshop/content/431960/";

fn main() -> Result<(), Box<dyn Error>> {
    let wallpapers: Mutex<Vec<WEProject>> = Mutex::new(vec![]);
    let wallpaper_infos: Mutex<Vec<WPInfo>> = Mutex::new(vec![]);
    let channel = Mutex::new(IpcChannel::connect("/tmp/waypaper-engine.sock").unwrap());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![loaded, set_wp, apply_filter])
        .manage(wallpapers)
        .manage(wallpaper_infos)
        .manage(channel)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

pub fn to_base64(path: &Path) -> String {
    let mut file_type: String = path.extension().unwrap().to_str().unwrap().to_owned();

    if file_type == "jpg" {
        file_type = "jpeg".to_owned();
    }

    assert!(file_type == "jpeg" || file_type == "gif" || file_type == "png");

    let mut file = File::open(path).unwrap();
    let mut vector = vec![];
    let _ = file.read_to_end(&mut vector);
    let base64 = STANDARD.encode(vector);

    format!(
        "data:image/{};base64,{}",
        file_type,
        base64.replace("\r\n", "")
    )
}
