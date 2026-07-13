use serde::Serialize;
use std::{fs::OpenOptions, io};

#[derive(Serialize)]
struct FileContent {
    name: String,
    url: String,
    method: String,
}

pub fn create_file(name: &str, parent_dir: &str) -> io::Result<String> {
    let path = format!("{parent_dir}/{name}.json");

    let file = OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if the file already exists
        .open(&path)?;

    let content = FileContent {
        name: name.to_string(),
        url: String::new(),
        method: "GET".to_string(),
    };

    serde_json::to_writer_pretty(file, &content).map_err(io::Error::other)?;

    Ok(path)
}
