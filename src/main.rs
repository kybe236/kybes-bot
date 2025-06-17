mod commands;
mod utils;

use std::{io::Write, path::Path, sync::Arc, vec};

use serde::{Deserialize, Serialize};
use serenity::all::{CacheHttp, ClientBuilder, GatewayIntents, UserId};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, BufReader},
    sync::RwLock,
};
use tracing::{error, info, warn};

use crate::utils::git::get_git_hash;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    discord_token: String,
    deepseek_token: Option<String>,
    youtube_token: Option<String>,
    youtube_whitelist_active: bool,
    youtube_whitelist: Vec<String>,
    admin_list: Vec<String>,
    deepseek_whitelist_active: bool,
    deepseek_whitelist: Vec<String>,
    ping_whitelist_active: bool,
    ping_witelist: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discord_token: "".to_string(),
            admin_list: vec!["921066050009833572".to_string()],
            youtube_token: None,
            youtube_whitelist_active: false,
            youtube_whitelist: vec!["921066050009833572".to_string()],
            deepseek_token: None,
            deepseek_whitelist_active: true,
            deepseek_whitelist: vec!["921066050009833572".to_string()],
            ping_whitelist_active: false,
            ping_witelist: vec!["921066050009833572".to_string()],
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
            let youtube_token = Self::ask_youtube_token().await?;
            let deepseek_token = Self::ask_deepseek_token().await?;

            let youtube_token = if youtube_token.is_empty() {
                None
            } else {
                Some(youtube_token)
            };

            let deepseek_token = if deepseek_token.is_empty() {
                None
            } else {
                Some(deepseek_token)
            };

            let config = Config {
                discord_token: token,
                youtube_token,
                deepseek_token,
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

    async fn ask_deepseek_token() -> io::Result<String> {
        println!("https://platform.deepseek.com/api_keys");
        println!("keep empty to not set");
        print!("DEEPSEEK TOKEN => ");
        std::io::stdout().flush().unwrap();

        let mut token = String::new();
        let mut reader = BufReader::new(io::stdin());
        reader.read_line(&mut token).await?;
        Ok(token.trim().to_string())
    }

    async fn ask_youtube_token() -> io::Result<String> {
        println!(
            "https://console.cloud.google.com/apis/library/youtube.googleapis.com?inv=1&invt=Ab0XxQ&project=semiotic-lamp-376120"
        );
        println!("keep empty to not set");
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

    pub async fn reload(&mut self, path: &str) -> io::Result<()> {
        let data = fs::read_to_string(path).await?;
        let new_config: Config = serde_json::from_str(&data)?;

        *self = new_config;
        Ok(())
    }
}

pub struct Data {
    pub config: Arc<RwLock<Config>>,
}

async fn dm_admins_error(ctx: crate::Context<'_>, error: &str) {
    let data = ctx.data();
    let admin_list = data.config.read().await.admin_list.clone();
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
        commands: vec![
            commands::test(),
            commands::stop(),
            commands::version(),
            commands::morse(),
            commands::time(),
            commands::deepseek(),
            commands::reload_settings(),
            commands::yt_vid(),
            commands::ping(),
            commands::dump_ping(),
            commands::cat(),
            commands::save_alias(),
            commands::alias(),
        ],
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
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!("EVENT RECEIVED: {:?}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    config.save("config.json").await.unwrap();
    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            let config = Arc::new(RwLock::new(config));

            Box::pin(async move {
                let git_hash = match get_git_hash().await {
                    Some(v) => v,
                    None => "".to_string(),
                };
                info!("LOGGED IN AS: {} ON {}", ready.user.name, git_hash);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // Send git hash to all admins
                for admin_str in &config.read().await.admin_list {
                    if let Ok(admin_id) = admin_str.parse::<u64>() {
                        let user = UserId::new(admin_id);
                        if let Ok(channel) = user.create_dm_channel(ctx.http()).await {
                            let _ = channel
                                .say(ctx.http(), format!("Bot started! Git hash: {}", git_hash))
                                .await;
                        }
                    }
                }

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
