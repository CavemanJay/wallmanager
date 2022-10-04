use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use log::{debug, info, LevelFilter};
use notify::{self, EventKind, RecursiveMode, Watcher};
use rand::seq::SliceRandom;
use rand::{self, thread_rng};
use serde::Serialize;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::atomic::AtomicU8;
use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use wallmanager::{ get_folders, load_wallpapers, BG_ROOT};
use wallpaper;

const DEFAULT_INTERVAL_SECS: AtomicU8 = AtomicU8::new(10);

#[derive(Serialize)]
struct AppState {
    available_folders: Mutex<Vec<String>>,
    selected_folders: Mutex<HashSet<String>>,
    wallpapers: Mutex<HashSet<String>>,
    duration: AtomicU8,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            available_folders: get_folders().into(),
            selected_folders: HashSet::from([BG_ROOT.to_string()]).into(),
            wallpapers: load_wallpapers().unwrap().into(),
            duration: DEFAULT_INTERVAL_SECS,
        }
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    // let files = Arc::new(Mutex::new(load_wallpapers()?));
    // let thread_files = files.clone();
    // let watcher_files = files.clone();
    // // let chosen_folders = Arc::new(Mutex::new(HashSet::from([BG_ROOT.to_string()])));
    // let chosen_folders = Arc::new(Mutex::new(Vec::from([BG_ROOT.to_string()])));
    // let t_chosen_folders = chosen_folders.clone();

    // let mut watcher =
    //     notify::recommended_watcher(move |e: Result<notify::Event, notify::Error>| {
    //         if e.is_err() {
    //             return;
    //         }

    //         let event = e.unwrap();

    //         if let EventKind::Create(_) = event.kind {
    //             let mut wallpapers = watcher_files.lock().unwrap();
    //             for file in event.paths {
    //                 let path = file.to_string_lossy();
    //                 if wallpapers.insert(path.to_string()) {
    //                     debug!("New wallpaper: {}", path);
    //                 }
    //             }
    //         }
    //     })
    //     .unwrap();

    // watcher
    //     .watch(Path::new(BG_ROOT), RecursiveMode::Recursive)
    //     .unwrap();

    // thread::spawn(move || loop {
    //     return;
    //     let chosen_wallpaper = loop {
    //         let folders = t_chosen_folders.lock().unwrap();
    //         let mut wallpapers = thread_files.lock().unwrap();
    //         let matches = wallpapers
    //             .iter()
    //             .filter(|file| folders.iter().any(|folder| file.starts_with(folder)))
    //             .collect::<Vec<_>>();

    //         let mut rng = thread_rng();
    //         let selected = matches.choose(&mut rng).unwrap().to_string();

    //         if !Path::new(&selected).exists() {
    //             debug!("Removing nonexistant wallpaper: {}", selected);
    //             wallpapers.remove(selected.as_str());
    //             continue;
    //         }

    //         break selected;
    //     };

    //     info!("Updating wallpaper: {}", chosen_wallpaper);
    //     wallpaper::set_from_path(&chosen_wallpaper).unwrap();
    //     thread::sleep(Duration::from_secs(INTERVAL_SECS.into()));
    // });

    // let update_chosen_folders = |folders: Vec<String>, append: bool| {
    //     let folders = if folders.len() == 0 || (folders.len() == 1 && folders[0] == "") {
    //         vec![BG_ROOT.to_string()]
    //     } else {
    //         folders
    //     };

    //     let mut selected = chosen_folders.lock().unwrap();
    //     let new_selection = if append {
    //         let mut combined = [selected.clone(), folders].concat();
    //         combined.sort_unstable();
    //         combined.dedup();
    //         combined
    //     } else {
    //         folders
    //     };

    //     info!("New selection: {:?}", new_selection);
    //     *selected = new_selection;
    // };
    let state = web::Data::new(AppState::default());

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(list_folders)
            .service(list_selected)
            .service(update_selected)
    })
    .bind(("192.168.88.240", 8080))?
    .run()
    .await
}

#[get("/folders")]
async fn list_folders(data: Data<AppState>) -> impl Responder {
    let folders = data.available_folders.lock().unwrap();
    let folder_list = folders.iter().collect::<Vec<_>>();
    let serialized = serde_json::to_string(&folder_list).unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serialized)
}

#[get("/selected")]
async fn list_selected(data: Data<AppState>) -> impl Responder {
    let folders = data.selected_folders.lock().unwrap();
    let folder_list = folders.iter().collect::<Vec<_>>();
    let serialized = serde_json::to_string(&folder_list).unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serialized)
}

#[post("/selected")]
async fn update_selected(
    data: Data<AppState>,
    new_selection: web::Json<Vec<String>>,
) -> impl Responder {
    let folders = data.available_folders.lock().unwrap();
    let mut selected = data.selected_folders.lock().unwrap();

    *selected= new_selection
        .0
        .iter()
        .filter(|path| folders.binary_search(path).is_ok())
        .map(ToString::to_string)
        .collect();

    let folder_list = selected.iter().collect::<Vec<_>>();
    let serialized = serde_json::to_string(&folder_list).unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serialized)

}
