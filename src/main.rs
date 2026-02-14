use std::env;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Guild;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::*;
use tracing::debug;

struct Handler {
    auto_role_id: RoleId,
    allowed_guild_ids: Vec<GuildId>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, _msg: Message) {
        // TODO: handle messages
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if let Err(why) = new_member.add_role(&ctx.http, self.auto_role_id).await {
            debug!("Failed to assign role to {}: {why}", new_member.user.name);
        } else {
            debug!("Assigned auto-role to {}", new_member.user.name);
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if !self.allowed_guild_ids.contains(&guild.id) {
            debug!("Leaving unauthorized guild: {} ({})", guild.name, guild.id);
            if let Err(why) = guild.id.leave(&ctx.http).await {
                debug!("Failed to leave guild {}: {why}", guild.id);
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);
    }
}

#[cfg(windows)]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to listen for SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");

    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    }
}

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "DEBUG");
    }
    tracing_subscriber::fmt::init();
    let token = env::var("BANANABOT_DISCORD_TOKEN")
        .expect("Expected BANANABOT_DISCORD_TOKEN in environment");

    let auto_role_id = env::var("BANANABOT_AUTO_ROLE_ID")
        .expect("Expected BANANABOT_AUTO_ROLE_ID in environment")
        .parse::<u64>()
        .expect("BANANABOT_AUTO_ROLE_ID must be a valid u64");

    let allowed_guild_ids: Vec<GuildId> = env::var("BANANABOT_ALLOWED_GUILD_IDS")
        .expect("Expected BANANABOT_ALLOWED_GUILD_IDS in environment")
        .split(',')
        .map(|id| {
            GuildId::new(
                id.trim()
                    .parse::<u64>()
                    .expect("Each guild ID in BANANABOT_ALLOWED_GUILD_IDS must be a valid u64"),
            )
        })
        .collect();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MEMBERS;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            auto_role_id: RoleId::new(auto_role_id),
            allowed_guild_ids,
        })
        .await
        .expect("Error creating client");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        shutdown_signal().await;
        debug!("Shutting down...");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        debug!("Client error: {why}");
    }
}
