mod commands;
mod utils;

use std::{io::Write, path::Path};

use serde::{Deserialize, Serialize};
use serenity::all::{ClientBuilder, GatewayIntents, UserId};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, BufReader},
};
use tracing::{error, info, warn};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    discord_token: String,
    admin_list: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discord_token: "".to_string(),
            admin_list: vec!["921066050009833572".to_string()],
        }
    }
}

impl Config {
    pub async fn load_or_create(path: &str) -> io::Result<Self> {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path).await?;
            let config = serde_json::from_str(&data)?;
            Ok(config)
        } else {
            let token = Self::ask_token().await?;

            let config = Config {
                discord_token: token,
                ..Default::default()
            };
            config.save(path).await?;
            Ok(config)
        }
    }

    async fn ask_token() -> io::Result<String> {
        print!("DISCORD TOKEN => ");
        std::io::stdout().flush().unwrap();

        let mut token = String::new();
        let mut reader = BufReader::new(io::stdin());
        reader.read_line(&mut token).await?;
        Ok(token.trim().to_string())
    }

    pub async fn save(&self, path: &str) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json).await
    }
}

pub struct Data {
    pub config: Config,
}

async fn dm_admins_error(ctx: crate::Context<'_>, error: &str) {
    let data = ctx.data();
    let admin_list = data.config.admin_list.clone();
    for admin_str in admin_list {
        if let Ok(admin_id) = admin_str.parse::<u64>() {
            let channel = UserId::new(admin_id).create_dm_channel(ctx.http()).await;
            if let Ok(channel) = channel {
                let _ = channel
                    .say(ctx.http(), format!("An error occurred: {}", error))
                    .await;
                warn!("An error occurred: {}", error);
            }
        }
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            dm_admins_error(ctx, &error.to_string()).await
        }
        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
            dm_admins_error(ctx, &format!("PANIC: {}", payload.unwrap_or_default())).await
        }
        error => {
            error!("ERROR: {:#?}", error.to_string());
        }
    }
}

#[tokio::main]
async fn main() {
    let config = Config::load_or_create("config.json")
        .await
        .expect("Failed to load or create config");

    let token = config.discord_token.clone();

    tracing_subscriber::fmt::init();

    let opt = poise::FrameworkOptions {
        commands: vec![commands::test(), commands::stop()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: None,
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                info!("STARTING COMMAND: {}", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                info!("FINISHED COMMAND: {}", ctx.command().qualified_name);
            })
        },
        command_check: Some(|ctx| {
            // TODO: make it load the whitelist from the config file
            Box::pin(async move {
                if ctx.author().id == 921066050009833572 {
                    return Ok(true);
                }
                Ok(false)
            })
        }),
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!("EVENT RECEIVED: {:?}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            let config = config.clone();
            Box::pin(async move {
                info!("LOGGED IN AS: {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { config })
            })
        })
        .options(opt)
        .build();

    let intents = GatewayIntents::non_privileged();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
