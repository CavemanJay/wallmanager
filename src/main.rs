use anyhow::{Context, Result};
use extensions::LineSplitter;
use inquire::Editor;
use log::{debug, info, LevelFilter};
use notify::{self, EventKind, RecursiveMode, Watcher};
use rand::seq::SliceRandom;
use rand::{self, thread_rng};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Stdio;
use std::{env, fs};
use std::{
    io,
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

mod extensions;

const INTERVAL_SECS: u8 = 10;
#[cfg(windows)]
const BG_ROOT: &str = "S:/Backgrounds";

const CFG_FILE: &str = "cfg.json";

#[cfg(windows)]
const EDITOR_CMD: &str = r"C:\WINDOWS\gvim.bat";
#[cfg(not(windows))]
const EDITOR_CMD: &str = "vim";

type Config = Vec<String>;

enum InputAction {
    PrintCurrent,
    EditCurrent,
    SetRoot,
    ChooseFolders(String),
    ReloadWallpapers,
    AppendSelection(String),
    Nop,
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let cfg = read_cfg().with_context(|| "Failed to read initial config file")?;

    let files = Arc::new(Mutex::new(load_wallpapers()?));
    let thread_files = files.clone();
    let watcher_files = files.clone();
    let chosen_folders = Arc::new(Mutex::new(cfg));
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

    let update_chosen_folders = |folders: Vec<String>, append: bool| {
        let folders = if folders.is_empty() || (folders.len() == 1 && folders[0].is_empty()) {
            vec![BG_ROOT.to_string()]
        } else {
            folders
        };

        let mut selected = chosen_folders.lock().unwrap();
        let new_selection = if append {
            let mut combined = [selected.clone(), folders].concat();
            combined.sort_unstable();
            combined.dedup();
            combined
        } else {
            folders
        };

        info!("New selection: {:?}", new_selection);
        write_cfg(&new_selection).expect("Could not update config file");
        *selected = new_selection;
    };

    let edit_selected = || {
        let chosen = {
            let folders = chosen_folders.lock().unwrap();
            Editor::new("")
                .with_editor_command(OsStr::new(EDITOR_CMD))
                .with_predefined_text(&folders.join("\n"))
                .prompt_skippable()
                .unwrap()
                .unwrap_or_default()
                .as_str()
                .trim()
                .split_lines()
        };

        update_chosen_folders(chosen, false);
    };

    loop {
        match handle_input() {
            InputAction::PrintCurrent => {
                info!("Folders: {:?}", chosen_folders.lock().unwrap());
                info!("Wallpaper: {}", wallpaper::get().unwrap());
            }
            InputAction::SetRoot => update_chosen_folders(vec![BG_ROOT.to_string()], false),
            InputAction::ChooseFolders(input) => {
                let new_selection = get_folders(&input);
                update_chosen_folders(new_selection, false)
            }
            InputAction::ReloadWallpapers => {
                let mut wallpapers = files.lock().unwrap();
                *wallpapers = load_wallpapers()?;
            }
            InputAction::EditCurrent => edit_selected(),
            InputAction::AppendSelection(input) => {
                let input = &input[2..].trim();
                let new_selection = get_folders(input);
                update_chosen_folders(new_selection, true)
            }
            InputAction::Nop => {}
        }
    }
}

fn handle_input() -> InputAction {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input = input.trim().to_string();

    match input.as_str() {
        "." | "/" => InputAction::SetRoot,
        "c" | "p" => InputAction::PrintCurrent,
        "r" => InputAction::ReloadWallpapers,
        "e" => InputAction::EditCurrent,
        n if n.starts_with("a ") => InputAction::AppendSelection(input),
        "" => InputAction::Nop,
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
        .split('\n')
        .map(|s| s.replace('\\', "/"))
        .collect::<Vec<_>>();

    match output.len() {
        1 => output,
        _ => {
            let received = Editor::new("Select backgrounds")
                .with_predefined_text(&output.join("\n"))
                .with_editor_command(OsStr::new(r"C:\WINDOWS\gvim.bat"))
                // .with_editor_command(OsStr::new(r"C:\WINDOWS\vim.bat"))
                .prompt_skippable()
                .unwrap()
                .unwrap_or_default()
                .as_str()
                .split_lines();
            received
        }
    }
}

fn load_wallpapers() -> Result<HashSet<String>> {
    let output = Command::new("fd")
        .args(["-t", "f", "jpg|png", BG_ROOT])
        .output()?;

    let out = output.stdout;
    let out_str = String::from_utf8_lossy(&out);
    let mut files = out_str
        .split('\n')
        .map(|s| s.replace('\\', "/"))
        .collect::<HashSet<_>>();

    files.reserve(100);

    debug!("Loaded {} files", files.len());

    Ok(files)
}

fn read_cfg() -> Result<Vec<String>> {
    let path = env::current_dir()
        .context("Failed to retrieve current directory.")?
        .join(CFG_FILE);

    if !path.exists() {
        debug!("Creating initial config file");
        let default = vec![BG_ROOT.into()];
        write_cfg(&default)?;
        return Ok(default);
    }

    let mut contents = String::new();
    let mut cfg_file = fs::File::options()
        .read(true)
        .open(path)
        .context("Failed to open config file for reading")?;

    cfg_file
        .read_to_string(&mut contents)
        .context("Failed to read file contents to string")?;

    let cfg = serde_json::from_str(&contents).context("Failed to parse config file")?;
    Ok(cfg)
}

fn write_cfg(val: &Config) -> Result<()> {
    let mut cfg_file = fs::File::options()
        .write(true)
        .create(true)
        .open(CFG_FILE)
        .context("Failed to open config file for writing")?;

    let stringified = serde_json::to_string(&val).context("Failed to stringify config value")?;
    cfg_file
        .write(stringified.as_bytes())
        .context("Failed to write config file")?;
    Ok(())
}
