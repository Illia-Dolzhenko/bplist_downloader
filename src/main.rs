pub mod downloader;

use core::time;
use std::{env, time::SystemTime};

use bmbf_quest_utills::*;
use chrono::Utc;
use downloader::*;
use reqwest::Client;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if let Some(bplist_path) = args.get(1) {
        let client = Client::new();
        let playlist = read_bplist(bplist_path);

        if let Some(playlist) = playlist {
            let mut songs: Vec<&Song> = playlist.songs.iter().collect();
            println!("Playlist contains {} songs", songs.len());
            let existing_hashes = get_existing_song_hashes();
            existing_hashes.iter().for_each(|hash| {
                println!("{}", &hash);
            });
            songs.retain(|song| !existing_hashes.contains(&song.hash));
            println!("Songs to download: {}", songs.len());

            create_dir("songs");

            for (index, future) in songs
                .iter()
                .filter(|song| song.key.is_some())
                .map(|song| {
                    download_async(
                        &client,
                        format!(
                            "https://api.beatsaver.com/download/key/{}",
                            song.key.as_ref().unwrap()
                        ),
                        format!("songs/{}.zip", song.hash),
                    )
                })
                .enumerate()
            {
                println!("Downloading: {}/{}", index + 1, songs.len());
                match future.await {
                    Ok(_) => (),
                    Err(err) => println!("Error: {}", err),
                }
            }

            remove_unpacked_songs();
            unpack_songs(&songs);
            remove_downloaded_songs();
            let time = Utc::now();

            save_playlist(
                &playlist,
                &format!("{}_pld.json", time.format("%d_%m_%y_%H%M%S")),
            );
        } else {
            println!(
                "Failed to read a bplist file from the provided path: {}",
                bplist_path
            );
        }
    } else {
        println!("Add a correct path to a bplist as an argument!");
    }
}
