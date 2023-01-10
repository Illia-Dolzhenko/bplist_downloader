use bmbf_quest_utills::Song;
use bytes::Bytes;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{blocking::Client, header::HeaderValue, StatusCode};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use zip_extract::ZipExtractError;

const CHUNK_SIZE: u64 = 10240;
pub const UNPACKED_PATH: &str = "songs_unpacked";
pub const DOWNLOADED_PATH: &str = "songs";

pub async fn download_async(
    client: &reqwest::Client,
    url: String,
    path: String,
) -> Result<(), String> {
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|_| format!("Failed to GET from '{}'", &url))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    let progress_bar = ProgressBar::new(total_size);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").map_err(|_| "Failed to create progress bar")?
        .progress_chars("#>-"));
    progress_bar.set_message(format!("Downloading {}", url));

    let mut file = File::create(&path).map_err(|_| format!("Failed to create file '{}'", &path))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    let mut data: Vec<Bytes> = Vec::new();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|_| "Error while downloading file")?;

        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        data.push(chunk);
        downloaded = new;
        progress_bar.set_position(new);
    }

    for chunk in data {
        file.write_all(&chunk)
            .map_err(|_| "Error while writing to file")?;
    }

    progress_bar.finish_with_message(format!("Downloaded {} to {}", url, path));
    Ok(())
}

pub fn download(client: &Client, url: &str, path: &str) -> Result<(), String> {
    let response = client
        .head(url)
        .send()
        .map_err(|_| format!("Failed to send head request {}", url))?;

    let length = response
        .content_length()
        .ok_or("Response doesn't have the content length")?;

    let mut output_file =
        File::create(path).map_err(|_| format!("Failed to create output file {}", path))?;

    let mut start: u64 = 0;
    let end: u64 = length - 1;

    while start < end {
        let prev_start = start;
        start += CHUNK_SIZE.min(end - start + 1);

        let range_header = HeaderValue::from_str(&format!("bytes={}-{}", prev_start, start - 1))
            .map_err(|_| "Failed to construct range header")?;

        println!("Downloading: {}/{}", start, end);

        let mut response = client
            .get(url)
            .header(reqwest::header::RANGE, &range_header)
            .send()
            .map_err(|_| {
                format!(
                    "Failed to send request to {} with range header {}",
                    url,
                    range_header.to_str().unwrap_or("error")
                )
            })?;

        let status = response.status();

        if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
            return Err(format!("Unexpected server response: {}", status));
        }
        std::io::copy(&mut response, &mut output_file)
            .map_err(|_| "Error while writing to the output file")?;
    }

    let content = response
        .text()
        .map_err(|_| "Failed to get text from head response")?;
    std::io::copy(&mut content.as_bytes(), &mut output_file)
        .map_err(|_| "Errow while writing response text to the output file")?;

    Ok(())
}

pub fn create_dir(path: &str) {
    if !Path::new(path).is_dir() {
        match fs::create_dir(path) {
            Ok(_) => println!("{} directory created", path),
            Err(_) => println!("Failed to create directory {}", path),
        }
    }
}

pub fn remove_dir(path: &str) {
    if Path::new(path).is_dir() {
        match fs::remove_dir_all(path) {
            Ok(_) => println!("{} directory removed with all its contents", path),
            Err(_) => println!("Failed to remove directory {}", path),
        }
    }
}

pub fn unpack_zip(path: &str, name: &str) {
    match fs::read(path) {
        Ok(zip) => {
            let target_dir = PathBuf::from(format!("{}/{}", UNPACKED_PATH, name));
            //create_dir(target_dir.to_str().unwrap_or_default());
            match zip_extract::extract(Cursor::new(zip), &target_dir, false) {
                Ok(_) => println!(
                    "Archive {} unpacked to {}",
                    path,
                    target_dir.to_str().unwrap_or_default()
                ),
                Err(err) => match err {
                    ZipExtractError::Io(err) => println!(
                        "Failed to extract archive {} to {}, IO error {}",
                        path,
                        target_dir.to_str().unwrap_or_default(),
                        err
                    ),
                    ZipExtractError::Zip(_) => println!(
                        "Failed to extract archive {} to {}, ZIP error",
                        path,
                        target_dir.to_str().unwrap_or_default()
                    ),
                    ZipExtractError::StripToplevel {
                        toplevel,
                        path,
                        error,
                    } => println!(
                        "Failed to extract archive {} to {}, StripTopLevel error",
                        path.to_str().unwrap_or_default(),
                        target_dir.to_str().unwrap_or_default()
                    ),
                },
            }
        }
        Err(_) => println!("Failed to read zip file {}", path),
    }
}

pub fn unpack_songs(songs: &[&Song]) {
    songs.iter().for_each(|song| {
        create_dir(UNPACKED_PATH);
        unpack_zip(&format!("songs/{}.zip", &song.hash), &song.hash);
    })
}

pub fn remove_downloaded_songs() {
    remove_dir(DOWNLOADED_PATH)
}

pub fn remove_unpacked_songs() {
    remove_dir(UNPACKED_PATH)
}
