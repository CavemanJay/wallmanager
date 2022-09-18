use rand::seq::SliceRandom;
use rand::{self, thread_rng};
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
    let files = Arc::new(Mutex::new(load_wallpapers()?));
    let thread_files = files.clone();
    let chosen_folders = Arc::new(Mutex::new(vec![BG_ROOT.to_string()]));
    let t_chosen_folders = chosen_folders.clone();

    thread::spawn(move || {
        loop {
            let wallpapers_guard = thread_files.lock().unwrap();
            let folder_selection_guard = t_chosen_folders.lock().unwrap();
            let folders = &*folder_selection_guard;
            let wallpapers = &*wallpapers_guard;

            let matches = wallpapers
                .iter()
                .filter(|file| folders.iter().any(|folder| file.starts_with(folder)))
                .collect::<Vec<_>>();

            let mut rng = thread_rng();
            let selected = matches.choose(&mut rng).unwrap();
            wallpaper::set_from_path(selected).unwrap();

            // Drop guards so we can unlock the mutex (prevent deadlock)
            drop(folder_selection_guard);
            drop(wallpapers_guard);

            thread::sleep(Duration::from_secs(INTERVAL_SECS.into()));
        }
    });

    let update_chosen_folders = |folders| {
        println!("New selection: {:?}", folders);
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
        "c" => InputAction::PrintCurrent,
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

fn load_wallpapers() -> io::Result<Vec<String>> {
    let output = Command::new("fd")
        .args(["-t", "f", "jpg|png", BG_ROOT])
        .output()?;

    let out = output.stdout;
    let out_str = String::from_utf8_lossy(&out);
    let files = out_str
        .split('\n')
        .map(|s| s.replace("\\", "/"))
        .collect::<Vec<_>>();

    println!("Loaded {} files", files.len());

    Ok(files)
}
