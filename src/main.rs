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

fn main() {
    let args = Args::parse();

    let (sender, receiver) = mpmc::sync_channel(1);

    let workers = (0..THREAD_COUNT).map(|_| {
        let rx = receiver.clone();
        thread::spawn(move || worker(rx))
    });

    producer(sender.clone(), &args.in_folder, &args.out_folder);
    drop(sender);

    workers.for_each(|w| {
        w.join().unwrap();
        println!("Joined worker");
    });
}

fn process_file(in_file: &Path, out_file: &Path) {
    println!("in: {:?}, out: {:?}", in_file, out_file)
}

fn producer(sender: mpmc::Sender<(PathBuf, PathBuf)>, in_folder: &str, out_folder: &str) {
    for entry in WalkDir::new(in_folder) {
        let unw_entry = entry.unwrap();
        println!("Sender: {:?}", unw_entry);
        if unw_entry.file_type().is_file() {
            println!("Sending: {:?}", unw_entry.path());
            sender
                .send((
                    unw_entry.into_path(),
                    PathBuf::from_str(out_folder).unwrap(),
                ))
                .unwrap();
        }
    }

    // sender.send(("inf".into(), "outf".into())).unwrap();
}

fn worker(receiver: mpmc::Receiver<(PathBuf, PathBuf)>) {
    let (in_file, out_file) = receiver.recv().unwrap();
    process_file(&in_file, &out_file);
    println!("Worker exited")
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
