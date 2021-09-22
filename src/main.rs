mod config;
mod commands;
mod utils;

use commands::{general, music};
use songbird::SerenityInit;

use std::{collections::HashSet, sync::Arc};
use serenity::{async_trait, framework::StandardFramework, http::Http, model::{gateway::Ready, id::ChannelId}, prelude::*};
use log::{self, LevelFilter};

use crate::{commands::music::{MusicKey, MusicParams}, config::BaseConfig};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {

    // TODO: Initialising logging
    env_logger::builder().filter_level(LevelFilter::Info).init();

    //Config loading
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        log::error!("Usage: \"./jank-bot ./path/to/config.ron\"");
        return;
    }
    let base_config_file = &args[1];
    log::info!("Getting config from {}", base_config_file);

    let config: BaseConfig = match utils::deserialise_from_file(&base_config_file) {
        Ok(conf) => conf,
        Err(err) => {
            log::error!("Failed to retrieve config: {}", err);
            return;
        }
    };

    log::info!("Loaded configuration file from {}", base_config_file);

    let http = Http::new_with_token(&config.discord_token);

    let (owners, _) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        },
        Err(err) => panic!("Failed to get bot application info: {:?}", err),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix(&config.prefix))
        .group(&general::GENERAL_GROUP)
        .group(&music::MUSIC_GROUP);
    
    let music_params = Arc::new(MusicParams {
        volume: RwLock::new(config.music_config.volume),
        current_song: RwLock::new(None),
        queue: RwLock::new(Vec::new()),
        text_channel: ChannelId(config.text_channel),
    });

    let mut client = Client::builder(&config.discord_token)
        .framework(framework)
        .event_handler(Handler)
        .register_songbird()
        .type_map_insert::<MusicKey>(music_params)
        .await
        .expect("Failed to create client.");

    tokio::spawn(async move {
        let _ = client.start().await.map_err(|err| log::error!("Client stopped: {:?}", err));
    });

    tokio::signal::ctrl_c().await.unwrap();
    println!("Received shutdown.");
}