#![feature(mpmc_channel)]
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::mpmc,
    thread,
};

use clap::{Parser, ValueEnum};
use walkdir::WalkDir;

const THREAD_COUNT: usize = 1;
const MUSIC_EXTENSIONS: [&str; 4] = ["mp3", "flac", "m4a", "m4p"];
const IMG_EXTENSIONS: [&str; 3] = ["jpg", "jpeg", "png"];

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
    });
}

fn process_file(in_file: &Path, out_path: &Path, encoding: Encoding) {
    if let Some(in_ext) = in_file.extension().and_then(|osstr| osstr.to_str()) {
        if MUSIC_EXTENSIONS.contains(&in_ext) && in_ext != encoding.ext() {
            // Transcode file
            fs::create_dir_all(out_path).unwrap();
            let in_filename = in_file
                .file_name()
                .and_then(|osstr| osstr.to_str())
                .unwrap();
            let out_file = out_path.join(in_filename.replace(in_ext, encoding.ext()));
            if fs::exists(&out_file).unwrap() {
                println!("Skipping existing {:?}", &out_file);
            } else {
                println!("Transcoding {:?}", &in_file);
                let mut ffmpeg_command = Command::new("ffmpeg");
                ffmpeg_command.arg("-i").arg(in_file.to_str().unwrap());
                encoding.add_encoding_args(&mut ffmpeg_command);
                ffmpeg_command.arg(out_file);
                ffmpeg_command.output().unwrap();
            }
        } else if IMG_EXTENSIONS.contains(&in_ext) || in_ext == encoding.ext() {
            // Copy file as-is
            let out_file = out_path.join(in_file.file_name().unwrap());
            fs::create_dir_all(out_path).unwrap();
            fs::copy(in_file, out_file).unwrap();
        } else {
            println!("Unexpected ext found: {:?}", in_ext);
        }
    } else {
        println!("Found file without extension: {:?}", in_file);
    }
}

fn producer(sender: mpmc::Sender<(PathBuf, PathBuf)>, in_folder: &str, out_folder: &str) {
    let in_prefix = PathBuf::from(in_folder);
    let out_prefix = PathBuf::from(out_folder);
    let mut extensions = HashSet::new();
    for entry in WalkDir::new(in_folder) {
        let unw_entry = entry.unwrap();
        if unw_entry.file_type().is_file() {
            let ext = unw_entry.path().extension().map(|optext| optext.to_owned());
            extensions.insert(ext);
            let inf = unw_entry.into_path();
            let outpath = out_prefix.join(
                inf.strip_prefix(&in_prefix)
                    .ok()
                    .and_then(|p| p.parent())
                    .unwrap(),
            );
            sender.send((inf, outpath)).unwrap();
        }
    }

    println!("Found extensions: {:?}", extensions);
}

fn worker(receiver: mpmc::Receiver<(PathBuf, PathBuf)>, encoding: Encoding) {
    for (in_file, out_path) in receiver {
        process_file(&in_file, &out_path, encoding);
    }
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
            Self::MP3 => "mp3",
            Self::OPUS => "opus",
        }
    }

    fn add_encoding_args(self, ffmpeg_command: &mut Command) {
        match self {
            Self::MP3 => ffmpeg_command.arg("-aq").arg("0"),
            Self::OPUS => ffmpeg_command.arg("-b").arg(128_000.to_string()),
        };
    }
}
