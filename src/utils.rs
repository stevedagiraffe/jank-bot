use std::{fs::File, io::{BufReader, Read}};

use serde::de::DeserializeOwned;
use serenity::{http::Http, model::id::ChannelId};

pub async fn checked_msg(channel_id: &ChannelId, http: &Http, message: &str) {
    if let Err(err) = channel_id.say(&http, message).await {
        log::error!("Failed to send message: {:?}", err);
    }
}

pub fn deserialise_from_file<T: DeserializeOwned>(file_path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    log::debug!("Opened file {:?}", file);
    let mut buf_reader = BufReader::new(file);
    let mut str = String::new();
    buf_reader.read_to_string(&mut str).unwrap();
    log::debug!("File contents: {}", str);
    Ok(ron::from_str(str.as_str())?)
}