use log::debug;
use std::{
    collections::HashSet,
    io,
    process::{Command, Stdio},
};

#[cfg(windows)]
pub const BG_ROOT: &str = "S:/Backgrounds";

pub fn get_folders() -> Vec<String> {
    let fd = Command::new("fd")
        .args(["-t", "d", ".", BG_ROOT])
        .output()
        .unwrap();

    let mut folders: Vec<_> = String::from_utf8_lossy(&fd.stdout)
        .replace("\\", "/")
        .split("\n")
        .filter(|x| x.trim() != "")
        .chain([BG_ROOT])
        .map(ToString::to_string)
        .collect();

    folders.sort();
    folders
}

pub fn fuzzy_search_folders(input: &str) -> Vec<String> {
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
        .collect::<Vec<_>>();

    output
}

pub fn load_wallpapers() -> io::Result<HashSet<String>> {
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
