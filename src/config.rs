use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct BaseConfig {
    pub discord_token: String,
    pub prefix: String,
    pub text_channel: u64,
    pub music_config: MusicConfig,
}

#[derive(Serialize, Deserialize)]
pub struct MusicConfig {
    pub volume: f32,
}

