
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

const DISCORD_TOKEN_ENV: &str = "DISCORD_TOKEN";


struct Handler;

#[async_trait]
impl EventHandler for Handler {
    
}

#[tokio::main]
async fn main() {
    let token = std::env::var(DISCORD_TOKEN_ENV)
        .expect(format!("Bad token in env-var \"{}\"", DISCORD_TOKEN_ENV).as_str());

    let mut client = Client::builder(&token).event_handler(Handler).await.expect("Client is failing");

    if let Err(err) = client.start().await {
        log::error!("Client error: {:?}", err);
    }
}
