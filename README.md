# BananaBot

A Discord bot written in Rust using [Serenity](https://github.com/serenity-rs/serenity). BananaBot automatically assigns a role to new server members and restricts itself to a set of allowed guilds.

## Features

- **Auto-role assignment** — Automatically assigns a configured role to users when they join the server.
- **Guild allowlist** — Only operates in explicitly allowed guilds; leaves any unauthorized guild it is added to.
- **Graceful shutdown** — Handles SIGINT/SIGTERM (Unix) and Ctrl+C (Windows).

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition)
- A Discord bot token with the **Server Members** privileged intent enabled

## Configuration

BananaBot is configured via environment variables:

| Variable | Description |
|---|---|
| `BANANABOT_DISCORD_TOKEN` | Discord bot token |
| `BANANABOT_AUTO_ROLE_ID` | ID of the role to assign to new members |
| `BANANABOT_ALLOWED_GUILD_IDS` | Comma-separated list of guild IDs the bot is allowed to operate in |

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
