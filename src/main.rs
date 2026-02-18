use std::env;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType, Interaction, ResolvedValue};
use serenity::model::channel::{Message, Reaction, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member};
use serenity::model::id::{GuildId, RoleId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::debug;

const REACTION_ROLE_CONFIG_PATH: &str = "reaction_role.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReactionRoleConfig {
    channel_id: u64,
    message_id: u64,
    role_id: u64,
    emoji: String,
}

struct ReactionRoleConfigKey;

impl TypeMapKey for ReactionRoleConfigKey {
    type Value = Option<ReactionRoleConfig>;
}

fn load_reaction_role_config() -> Option<ReactionRoleConfig> {
    let path = Path::new(REACTION_ROLE_CONFIG_PATH);
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_reaction_role_config(config: &ReactionRoleConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(REACTION_ROLE_CONFIG_PATH, data)?;
    Ok(())
}

struct Handler {
    auto_role_id: RoleId,
    allowed_guild_ids: Vec<GuildId>,
}

async fn handle_reactionrole_command(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = command.data.options();

    let mut channel_id = None;
    let mut role_id = None;
    let mut emoji_str = None;
    let mut message_text = None;

    for opt in &options {
        match opt.name {
            "channel" => {
                if let ResolvedValue::Channel(ch) = &opt.value {
                    channel_id = Some(ch.id);
                }
            }
            "role" => {
                if let ResolvedValue::Role(role) = &opt.value {
                    role_id = Some(role.id);
                }
            }
            "emoji" => {
                if let ResolvedValue::String(s) = &opt.value {
                    emoji_str = Some(s.to_string());
                }
            }
            "message" => {
                if let ResolvedValue::String(s) = &opt.value {
                    message_text = Some(s.to_string());
                }
            }
            _ => {}
        }
    }

    let channel_id = channel_id.ok_or("Missing channel option")?;
    let role_id = role_id.ok_or("Missing role option")?;
    let emoji_str = emoji_str.ok_or("Missing emoji option")?;
    let message_text = message_text.ok_or("Missing message option")?;

    // Post the message in the specified channel
    let posted_message = channel_id
        .send_message(&ctx.http, CreateMessage::new().content(&message_text))
        .await?;

    // React to the posted message with the emoji
    let reaction_type = emoji_str
        .parse::<ReactionType>()
        .unwrap_or_else(|_| ReactionType::Unicode(emoji_str.clone()));
    posted_message.react(&ctx.http, reaction_type).await?;

    // Save config to file and update in-memory state
    let config = ReactionRoleConfig {
        channel_id: channel_id.get(),
        message_id: posted_message.id.get(),
        role_id: role_id.get(),
        emoji: emoji_str.clone(),
    };

    save_reaction_role_config(&config)?;

    {
        let mut data = ctx.data.write().await;
        data.insert::<ReactionRoleConfigKey>(Some(config));
    }

    // Respond with ephemeral confirmation
    let response = CreateInteractionResponseMessage::new()
        .content(format!(
            "Reaction role post created in <#{channel_id}>. Users can react with {emoji_str} to get <@&{role_id}>."
        ))
        .ephemeral(true);

    command
        .create_response(&ctx.http, CreateInteractionResponse::Message(response))
        .await?;

    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, _msg: Message) {
        // TODO: handle messages
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let config = {
            let data = ctx.data.read().await;
            data.get::<ReactionRoleConfigKey>().cloned().flatten()
        };

        let Some(config) = config else {
            return;
        };

        if reaction.message_id.get() != config.message_id {
            return;
        }

        let emoji_matches = match &reaction.emoji {
            ReactionType::Unicode(val) => *val == config.emoji,
            ReactionType::Custom { name: Some(name), .. } => *name == config.emoji,
            _ => false,
        };
        if !emoji_matches {
            return;
        }

        let Some(guild_id) = reaction.guild_id else {
            return;
        };
        let Some(user_id) = reaction.user_id else {
            return;
        };

        let role_id = RoleId::new(config.role_id);
        if let Err(why) = ctx.http.add_member_role(guild_id, user_id, role_id, None).await {
            debug!("Failed to add reaction role to {user_id}: {why}");
        } else {
            debug!("Added reaction role to {user_id}");
        }
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        let config = {
            let data = ctx.data.read().await;
            data.get::<ReactionRoleConfigKey>().cloned().flatten()
        };

        let Some(config) = config else {
            return;
        };

        if reaction.message_id.get() != config.message_id {
            return;
        }

        let emoji_matches = match &reaction.emoji {
            ReactionType::Unicode(val) => *val == config.emoji,
            ReactionType::Custom { name: Some(name), .. } => *name == config.emoji,
            _ => false,
        };
        if !emoji_matches {
            return;
        }

        let Some(guild_id) = reaction.guild_id else {
            return;
        };
        let Some(user_id) = reaction.user_id else {
            return;
        };

        let role_id = RoleId::new(config.role_id);
        if let Err(why) = ctx.http.remove_member_role(guild_id, user_id, role_id, None).await {
            debug!("Failed to remove reaction role from {user_id}: {why}");
        } else {
            debug!("Removed reaction role from {user_id}");
        }
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

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };

        if command.data.name.as_str() == "reactionrole" {
            if let Err(why) = handle_reactionrole_command(&ctx, &command).await {
                debug!("Error handling /reactionrole: {why}");
                let msg = CreateInteractionResponseMessage::new()
                    .content(format!("Error: {why}"))
                    .ephemeral(true);
                let _ = command
                    .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                    .await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);

        for &guild_id in &self.allowed_guild_ids {
            let command = CreateCommand::new("reactionrole")
                .description("Create a reaction role post")
                .default_member_permissions(Permissions::MANAGE_ROLES)
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel to post the reaction role message in",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "Role to assign when users react",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "emoji",
                        "Emoji to react with",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "message",
                        "Text content of the reaction role post",
                    )
                    .required(true),
                );

            if let Err(why) = guild_id.create_command(&ctx.http, command).await {
                debug!("Failed to create command for guild {guild_id}: {why}");
            }
        }
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

    // Load persisted reaction role config (if any)
    let reaction_role_config = load_reaction_role_config();
    if let Some(ref cfg) = reaction_role_config {
        debug!(
            "Loaded reaction role config: message_id={}, role_id={}, emoji={}",
            cfg.message_id, cfg.role_id, cfg.emoji
        );
    } else {
        debug!("No reaction role config found; feature inactive until /reactionrole is used");
    }

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            auto_role_id: RoleId::new(auto_role_id),
            allowed_guild_ids,
        })
        .await
        .expect("Error creating client");

    // Seed shared state with loaded config
    {
        let mut data = client.data.write().await;
        data.insert::<ReactionRoleConfigKey>(reaction_role_config);
    }

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
