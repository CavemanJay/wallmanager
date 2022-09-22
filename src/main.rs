use log::{debug, info, LevelFilter};
use notify::{self, EventKind, RecursiveMode, Watcher};
use rand::seq::SliceRandom;
use rand::{self, thread_rng};
use std::collections::HashSet;
use std::path::Path;
use std::process::Stdio;
use std::{
    io,
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use wallpaper;

const INTERVAL_SECS: u8 = 10;
const BG_ROOT: &str = "S:/Backgrounds";

enum InputAction {
    PrintCurrent,
    SetRoot,
    ChooseFolders(String),
    ReloadWallpapers,
}

fn main() -> io::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let files = Arc::new(Mutex::new(load_wallpapers()?));
    let thread_files = files.clone();
    let watcher_files = files.clone();
    let chosen_folders = Arc::new(Mutex::new(vec![BG_ROOT.to_string()]));
    let t_chosen_folders = chosen_folders.clone();

    let mut watcher =
        notify::recommended_watcher(move |e: Result<notify::Event, notify::Error>| {
            if e.is_err() {
                return;
            }

            let event = e.unwrap();

            if let EventKind::Create(_) = event.kind {
                let mut wallpapers = watcher_files.lock().unwrap();
                for file in event.paths {
                    let path = file.to_string_lossy();
                    if wallpapers.insert(path.to_string()) {
                        debug!("New wallpaper: {}", path);
                    }
                }
            }
        })
        .unwrap();

    watcher
        .watch(Path::new(BG_ROOT), RecursiveMode::Recursive)
        .unwrap();

    thread::spawn(move || loop {
        let chosen_wallpaper = loop {
            let folders = t_chosen_folders.lock().unwrap();
            let mut wallpapers = thread_files.lock().unwrap();
            let matches = wallpapers
                .iter()
                .filter(|file| folders.iter().any(|folder| file.starts_with(folder)))
                .collect::<Vec<_>>();

            let mut rng = thread_rng();
            let selected = matches.choose(&mut rng).unwrap().to_string();

            if !Path::new(&selected).exists() {
                debug!("Removing nonexistant wallpaper: {}", selected);
                wallpapers.remove(selected.as_str());
                continue;
            }

            break selected;
        };

        info!("Updating wallpaper: {}", chosen_wallpaper);
        wallpaper::set_from_path(&chosen_wallpaper).unwrap();
        thread::sleep(Duration::from_secs(INTERVAL_SECS.into()));
    });

    let update_chosen_folders = |folders| {
        info!("New selection: {:?}", folders);
        let mut guard = chosen_folders.lock().unwrap();
        *guard = folders;
    };

    loop {
        match handle_input() {
            InputAction::PrintCurrent => {
                println!("{}", wallpaper::get().unwrap());
            }
            InputAction::SetRoot => update_chosen_folders(vec![BG_ROOT.to_string()]),
            InputAction::ChooseFolders(input) => {
                let new_selection = get_folders(&input);
                update_chosen_folders(new_selection)
            }
            InputAction::ReloadWallpapers => {
                let mut wallpapers = files.lock().unwrap();
                *wallpapers = load_wallpapers()?;
            }
        }
    }
}

fn handle_input() -> InputAction {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input = input.trim().to_string();

    match input.as_str() {
        "." => InputAction::SetRoot,
        "c" | "p" => InputAction::PrintCurrent,
        "r" => InputAction::ReloadWallpapers,
        _ => InputAction::ChooseFolders(input),
    }
}

fn get_folders(input: &str) -> Vec<String> {
    let fd = Command::new("fd")
        .args(["-t", "d", ".", BG_ROOT])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let fzf = Command::new("fzf")
        .args(["-1", "-0", "-f", input])
        .stdin(fd.stdout.unwrap())
        .output()
        .unwrap();

    let output = String::from_utf8_lossy(&fzf.stdout)
        .trim()
        .split("\n")
        .map(|s| s.replace("\\", "/"))
        .collect();
    output
}

fn load_wallpapers() -> io::Result<HashSet<String>> {
    let output = Command::new("fd")
        .args(["-t", "f", "jpg|png", BG_ROOT])
        .output()?;

    let out = output.stdout;
    let out_str = String::from_utf8_lossy(&out);
    let mut files = out_str
        .split('\n')
        .map(|s| s.replace("\\", "/"))
        .collect::<HashSet<_>>();

    files.reserve(100);

    debug!("Loaded {} files", files.len());

    Ok(files)
}
