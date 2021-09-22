use std::time::Duration;

use serenity::{
    framework::{
        standard::{CommandResult, macros::{command, group}}
    }, 
    model::channel::{Message}, 
    prelude::*,
};
use songbird::{Event, TrackEvent};
use crate::{commands::music::{self, MusicKey}, utils::checked_msg};

#[group]
#[commands(summon, leave)]
pub struct General;

#[command]
#[only_in(guilds)]
async fn summon(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            checked_msg(&msg.channel_id, &ctx.http, "You must be in a voice channel").await;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await
        .expect("Songbird client should be in context").clone();
    
    if manager.get(guild.id).is_some() {
        if let Err(err) = manager.remove(guild.id).await {
            log::error!("Failed to remove songbird handlers: {:?}", err);
            checked_msg(&msg.channel_id, &ctx.http, "Failed to join channel").await;
            return Ok(());
        };
        
    }

    let channel_name = guild.channels.get(&connect_to).unwrap().mention();
    let join_msg = format!("Joining channel {}", channel_name);
    checked_msg(&msg.channel_id, &ctx.http, join_msg.as_str()).await;

    let (handle_lock, res) = manager.join(guild.id, connect_to).await;

    //TODO: Add event callbacks.
    if let Ok(_) = res {
        let ctx_data = ctx.data.read().await;
        let data = ctx_data.get::<MusicKey>().unwrap().clone();

        

        let mut curr_song = data.current_song.write().await;

        if curr_song.is_some() {
            log::warn!("Current song found. Pushing to front of queue!");
            let curr_uri = curr_song.as_ref().unwrap().uri.clone();
            *curr_song = None;

            let mut new_vec: Vec<String> = vec![curr_uri];
            new_vec.extend(data.queue.read().await.iter().map(|str| str.clone()));

            *data.queue.write().await = new_vec;
        }   
        

        let mut handle = handle_lock.lock().await;
        handle.add_global_event(Event::Track(TrackEvent::End), music::SongFinish {
            data: data.clone(),
        });

        handle.add_global_event(Event::Periodic(Duration::from_secs(1), None), music::PlayNext {
            songbird_manager: manager.clone(),
            data: data.clone(),
            guild_id: guild.id,
            http: ctx.http.clone(),
        });

        handle.add_global_event(Event::Periodic(Duration::from_secs(1), None), music::SongVolumeUpdate(data.clone()));

    } else {
        let err = res.unwrap_err();
        log::error!("Failed to join channel: {:?}", err);
        checked_msg(&msg.channel_id, &ctx.http, "Failed to join channel").await;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let manager = songbird::get(ctx).await
        .expect("Songbird client should be in context").clone();
    
    if manager.get(guild.id).is_some() {
        if let Err(err) = manager.remove(guild.id).await {
            checked_msg(&msg.channel_id, &ctx.http, format!("Failed to leave voice channel: {:?}", err).as_str()).await;
            return Ok(());
        }

        checked_msg(&msg.channel_id, &ctx.http, "Left voice channel").await;
    } else {
        checked_msg(&msg.channel_id, &ctx.http, "Not in a voice channel").await;
    }

    Ok(())
}
