use std::{sync::Arc};

use serenity::{async_trait, framework::{standard::{Args, CommandResult, macros::{command, group}}}, http::Http, model::{channel::{Message}, id::{ChannelId, GuildId}}, prelude::*};
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird, tracks::TrackHandle};
use crate::utils::checked_msg;


pub struct CurrentSong {
    pub uri: String,
    pub title: String,
    pub handle: TrackHandle,
}

pub struct MusicKey;

pub struct MusicParams {
    pub volume: RwLock<f32>,
    pub current_song: RwLock<Option<CurrentSong>>,
    pub queue: RwLock<Vec<String>>,
    pub text_channel: ChannelId,
}

impl TypeMapKey for MusicKey {
    type Value = Arc<MusicParams>;
}


#[group]
#[commands(play, volume)]
pub struct Music;

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let uri = match args.single::<String>() {
        Ok(vol) => vol,
        Err(_) => {
            checked_msg(&msg.channel_id, &ctx.http, "Url invalid").await;
            return Ok(());
        }
    };

    if !uri.starts_with("http") {
        checked_msg(&msg.channel_id, &ctx.http, "Url invalid").await;
        return Ok(());
    }

    if let Some(params) = get_music_params(&ctx).await {
        params.queue.write().await.push(uri.clone());
        checked_msg(&msg.channel_id, &ctx.http, format!("Added song to queue: {}", uri).as_str()).await;
    } else {
        log::error!("Internal music params failed to exist while adding song to queue");
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn volume(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {

    let volume = match args.single::<f32>() {
        Ok(vol) => vol,
        Err(err) => {
            log::warn!("Failed to parse volume: {}", err);
            checked_msg(&msg.channel_id, &ctx.http, "Input an actual volume (0.0-100.0)").await;
            return Ok(())
        }
    } / 100.0;
    if volume < 0.0 || volume > 1.0 {
        checked_msg(&msg.channel_id, &ctx.http, "Input an actual volume (0.0-100.0)").await;
        return Ok(());
    }
    let music_params = get_music_params(&ctx).await.unwrap();
    let mut inner = music_params.volume.write().await;

    log::info!("Volume changed. {} -> {}", inner, volume);
    checked_msg(&msg.channel_id, &ctx.http, format!("Changing volume to {}", (volume * 100.0)).as_str()).await;

    *inner = volume;
    
    Ok(())
}

async fn get_music_params(ctx: &Context) -> Option<Arc<MusicParams>> {
    ctx.data.read().await.get::<MusicKey>().cloned()
}

pub struct SongVolumeUpdate(pub Arc<MusicParams>);

#[async_trait]
impl VoiceEventHandler for SongVolumeUpdate {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        if let Some(song) = &*self.0.current_song.read().await {
            let volume = *self.0.volume.read().await;
            song.handle.set_volume(volume)
                .expect("Songbird failed to set volume.");
        }
        None
    }
}

pub struct SongFinish {
    pub data: Arc<MusicParams>,
}

#[async_trait]
impl VoiceEventHandler for SongFinish {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        log::info!("Song finished!");
        *self.data.current_song.write().await = None;
        None
    }
}

pub struct PlayNext {
    pub songbird_manager: Arc<Songbird>,
    pub data: Arc<MusicParams>,
    pub guild_id: GuildId,
    pub http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for PlayNext{
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        if self.data.current_song.read().await.is_some() {
            return None;
        }

        let mut song_queue = self.data.queue.write().await;
        if let Some(new_song) = song_queue.first().cloned() {
            
            let cloned_uri = new_song.clone();

            let source = match songbird::ytdl(&cloned_uri).await {
                Ok(src) => src,
                Err(err) => {
                    checked_msg(&self.data.text_channel, &self.http, format!("Failed to download audio: {}", cloned_uri).as_str()).await;
                    log::error!("Failed to download audio: {:?}", err);
                    return None;
                }
            };

            let title = source.metadata.title.as_ref().unwrap().clone();

            if let Some(lock_handle) = self.songbird_manager.get(self.guild_id) {
                let mut handle = lock_handle.lock().await;
                let track_handle = handle.play_only_source(source);
                    track_handle.set_volume(*self.data.volume.read().await)
                    .expect("Songbird failed to set volume.");
                
                *self.data.current_song.write().await = Some(CurrentSong {
                    uri: cloned_uri,
                    title: title.clone(),
                    handle: track_handle,
                });
                song_queue.remove(0);
                checked_msg(&self.data.text_channel, &self.http, format!("Now Playing: {}", title).as_str()).await;
            } else {
                log::info!("Bot tried to play song {} while not in channel.", title);
            }

        }

        None
    }
}