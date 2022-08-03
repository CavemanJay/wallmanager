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

fn main() -> Result<(), io::Error> {
    let output = Command::new("fd")
        .args(["-t", "f", "jpg|png", BG_ROOT])
        .output()?;

    let out = output.stdout;
    let out_str = String::from_utf8_lossy(&out);
    let files = Arc::new(
        out_str
            .split('\n')
            .map(|s| s.replace("\\", "/"))
            .collect::<Vec<_>>(),
    );

    println!("Loaded {} files", files.len());

    drop(out_str);
    let thread_files = files.clone();
    let chosen_folders = Arc::new(Mutex::new(vec![BG_ROOT.to_string()]));
    let t_chosen_folders = chosen_folders.clone();

    thread::spawn(move || {
        loop {
            let guard = t_chosen_folders.lock().unwrap();
            let folders = &*guard;
            // println!("Choosing wallpaper from: {:?}", folders);
            let matches = thread_files
                .iter()
                .filter(|file| folders.iter().any(|folder| file.starts_with(folder)))
                .collect::<Vec<_>>();

            // if !folders.contains(&BG_ROOT.to_string()) {
            //     println!("{:?}", matches);
            // }

            let mut rng = thread_rng();
            let selected = matches.choose(&mut rng).unwrap();
            // println!("Selected: {}", selected);
            wallpaper::set_from_path(selected).unwrap();

            // Drop guard so we can unlock the mutex
            drop(guard);
            thread::sleep(Duration::from_secs(INTERVAL_SECS.into()));
        }
    });

    loop {
        let folders = get_folders();

        if folders.is_empty() {
            continue;
        }

        println!("New selection: {:?}", folders);
        let mut guard = chosen_folders.lock().unwrap();
        *guard = folders;
    }
}

fn get_folders() -> Vec<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input = input.trim().to_string();

    if input == "." {
        return vec![BG_ROOT.into()];
    }

    let fd = Command::new("fd")
        .args(["-t", "d", ".", BG_ROOT])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // println!("{:?}", fd);

    let fzf = Command::new("fzf")
        .args(["-1", "-0", "-f", &input])
        .stdin(fd.stdout.unwrap())
        .output()
        .unwrap();
    // println!("{:?}", fzf);

    let output = String::from_utf8_lossy(&fzf.stdout)
        .trim()
        .split("\n")
        .map(|s| s.replace("\\", "/"))
        .collect();
    output
}
