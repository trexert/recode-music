#![feature(mpmc_channel)]
use std::{
    fs::FileType,
    path::{Path, PathBuf},
    str::FromStr,
    sync::mpmc,
    thread,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use walkdir::WalkDir;

const THREAD_COUNT: usize = 1;
const MUSIC_EXTENSIONS: [&str; 2] = ["mp3", "flac"];

fn main() {
    let args = Args::parse();

    let (sender, receiver) = mpmc::sync_channel(1);

    let workers: Vec<_> = (0..THREAD_COUNT)
        .map(|_| {
            let rx = receiver.clone();
            thread::spawn(move || worker(rx, args.encoding))
        })
        .collect();

    producer(sender.clone(), &args.in_folder, &args.out_folder);
    drop(sender);

    workers.into_iter().for_each(|w| {
        w.join().unwrap();
        println!("Joined worker");
    });
}

fn process_file(in_file: &Path, out_file: &Path, encoding: Encoding) {
    println!("in: {:?}, out: {:?}", in_file, out_file)
}

fn producer(sender: mpmc::Sender<(PathBuf, PathBuf)>, in_folder: &str, out_folder: &str) {
    let in_prefix = PathBuf::from(in_folder);
    let out_prefix = PathBuf::from(out_folder);
    for entry in WalkDir::new(in_folder) {
        let unw_entry = entry.unwrap();
        if should_process(&unw_entry) {
            let inf = unw_entry.into_path();
            let outf = out_prefix.join(inf.strip_prefix(&in_prefix).unwrap());
            sender.send((inf, outf)).unwrap();
        }
    }
}

fn worker(receiver: mpmc::Receiver<(PathBuf, PathBuf)>, encoding: Encoding) {
    for (in_file, out_file) in receiver {
        process_file(&in_file, &out_file, encoding);
    }
    println!("Worker exited")
}

fn should_process(file_entry: &walkdir::DirEntry) -> bool {
    file_entry.path().is_file()
        && MUSIC_EXTENSIONS.contains(&file_entry.path().extension().unwrap().to_str().unwrap())
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(required = true)]
    in_folder: String,

    #[arg(required = true)]
    out_folder: String,

    #[arg(required = true)]
    encoding: Encoding,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Encoding {
    MP3,
    OPUS,
}

impl Encoding {
    fn ext(self) -> &'static str {
        match self {
            Self::MP3 => ".mp3",
            Self::OPUS => ".opus",
        }
    }
}
