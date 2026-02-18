# BananaBot

A Discord bot written in Rust using [Serenity](https://github.com/serenity-rs/serenity). BananaBot automatically assigns a role to new server members, supports reaction roles via a slash command, and restricts itself to a set of allowed guilds.

## Features

- **Auto-role assignment** — Automatically assigns a configured role to users when they join the server.
- **Reaction roles** — Admins can create a reaction role post via the `/reactionrole` slash command. When users react to the post with the specified emoji they receive a role; removing the reaction removes the role. Configuration is persisted to a JSON file so it survives bot restarts.
- **Guild allowlist** — Only operates in explicitly allowed guilds; leaves any unauthorized guild it is added to.
- **Graceful shutdown** — Handles SIGINT/SIGTERM (Unix) and Ctrl+C (Windows).

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition)
- A Discord bot token with the **Server Members** privileged intent enabled
- The bot's OAuth2 URL must include the `bot` and `applications.commands` scopes
- The bot needs the **Send Messages**, **Add Reactions**, and **Manage Roles** permissions in your server

## Configuration

BananaBot is configured via environment variables:

| Variable | Description |
|---|---|
| `BANANABOT_DISCORD_TOKEN` | Discord bot token |
| `BANANABOT_AUTO_ROLE_ID` | ID of the role to assign to new members |
| `BANANABOT_ALLOWED_GUILD_IDS` | Comma-separated list of guild IDs the bot is allowed to operate in |

## Slash Commands

### `/reactionrole`

Creates a reaction role post in a specified channel. Only available to users with the **Manage Roles** permission.

| Option | Type | Description |
|---|---|---|
| `channel` | Channel | Channel to post the reaction role message in |
| `role` | Role | Role to assign when users react |
| `emoji` | String | Emoji to react with (e.g. a unicode emoji or custom emoji name) |
| `message` | String | Text content of the reaction role post |

When the command is run, the bot:

1. Posts the message in the chosen channel
2. Reacts to its own message with the specified emoji
3. Saves the configuration to `reaction_role.json`

From that point on, any user who reacts with the same emoji on that message receives the role. Removing the reaction removes the role.

Only one reaction role post is active at a time. Running the command again replaces the previous configuration.

## Building

```sh
cargo build --release
```

## Running

```sh
export BANANABOT_DISCORD_TOKEN="your-token"
export BANANABOT_AUTO_ROLE_ID="123456789"
export BANANABOT_ALLOWED_GUILD_IDS="111111111,222222222"

cargo run --release
```

## Deployment (systemd)

A systemd unit file is included in [bananabot.service](bananabot.service). To deploy:

1. Build the release binary and copy it to `/opt/bananabot/`.
2. Create an `.env` file at `/opt/bananabot/.env` with the environment variables listed above.
3. Create a dedicated system user:
   ```sh
   sudo useradd -r -s /usr/sbin/nologin bananabot
   ```
4. Install and enable the service:
   ```sh
   sudo cp bananabot.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now bananabot
   ```
