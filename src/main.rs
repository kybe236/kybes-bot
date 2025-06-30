mod commands;
mod config;
mod utils;

use std::{sync::Arc, vec};

use poise::FrameworkError;
use serenity::all::{CacheHttp, ClientBuilder, GatewayIntents, UserId};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::{config::Config, utils::git::get_git_hash};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug)]
pub struct Data {
    pub config: Arc<RwLock<Config>>,
}

/// Notify all configured admins about an error via DM
async fn dm_admins_error(ctx: Context<'_>, msg: &str) {
    let data = ctx.data();
    let admins = data.config.read().await.admin_list.clone();

    for id_str in admins {
        if let Ok(id) = id_str.parse::<u64>() {
            if let Ok(channel) = UserId::new(id).create_dm_channel(ctx.http()).await {
                let _ = channel.say(ctx.http(), format!("Error: {}", msg)).await;
            }
            warn!("Sent error to admin {}: {}", id, msg);
        }
    }
}

async fn on_error(error: FrameworkError<'_, Data, Error>) {
    match error {
        FrameworkError::Setup { error, .. } => panic!("Bot failed to start: {:?}", error),
        FrameworkError::Command { error, ctx, .. } => {
            dm_admins_error(ctx, &error.to_string()).await
        }
        FrameworkError::CommandPanic { payload, ctx, .. } => {
            let details = payload.unwrap_or_default();
            dm_admins_error(ctx, &format!("PANIC: {}", details)).await;
        }
        other => error!("Unhandled framework error: {:?}", other),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load configuration
    let config = Config::load_or_create("config.json").await?;
    let token = &config.discord_token.clone();

    tracing_subscriber::fmt::init();

    // Build framework options
    let framework_opts = poise::FrameworkOptions {
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
            commands::reminder(),
            commands::reminders(),
            commands::delete_reminder(),
            commands::github(),
            commands::translate(),
            commands::print(),
            commands::list_alias(),
            commands::delete_alias(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: None,
            ..Default::default()
        },
        on_error: |err| Box::pin(on_error(err)),
        pre_command: |ctx| {
            Box::pin(async move {
                info!(
                    "Starting: {} inside {}",
                    ctx.command().qualified_name,
                    ctx.guild_id()
                        .map_or("UNKNOWN".to_string(), |g| g.to_string())
                );
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                info!(
                    "Finished: {} inside {}",
                    ctx.command().qualified_name,
                    ctx.guild_id()
                        .map_or("UNKNOWN".to_string(), |g| g.to_string())
                );
            })
        },
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!("Event: {:?}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    config.save("config.json").await?;

    let framework = poise::Framework::builder()
        .options(framework_opts)
        .setup(|ctx, ready, framework| {
            let cfg_lock = Arc::new(RwLock::new(config));
            Box::pin(async move {
                let git_hash = get_git_hash().await.unwrap_or_default();
                info!("Logged in as {} (hash {})", ready.user.name, git_hash);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // Notify admins of startup
                for id_str in &cfg_lock.read().await.admin_list {
                    if let Ok(id) = id_str.parse::<u64>() {
                        if let Ok(dm) = UserId::new(id).create_dm_channel(ctx.http()).await {
                            let _ = dm
                                .say(ctx.http(), format!("Bot started: {}", git_hash))
                                .await;
                        }
                    }
                }

                // Load saved messages from disk into memory here:
                if let Err(e) = crate::commands::load_messages_from_file().await {
                    error!("Failed to load saved messages: {:?}", e);
                }

                commands::start_reminder_loop(ctx.clone()).await;
                Ok(Data { config: cfg_lock })
            })
        })
        .build();

    // Launch client
    let intents = GatewayIntents::non_privileged();
    ClientBuilder::new(token, intents)
        .framework(framework)
        .await?
        .start()
        .await?;

    Ok(())
}
