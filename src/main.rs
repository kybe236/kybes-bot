mod commands;
mod utils;

use std::env::var;

use serenity::all::{ClientBuilder, GatewayIntents};
use tracing::{error, info};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    // TODO: Implement a class to store reminders and save it to a file
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        error => {
            // TODO: dm all bot admins about the error
            error!("ERROR: {:#?}", error.to_string()); // For now very basic logging
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // TODO: load a config file and provide a setup wizard if launched for the first time

    let opt = poise::FrameworkOptions {
        commands: vec![commands::test()],
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
            Box::pin(async move {
                info!("LOGGED IN AS: {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                // TODO: add data here
            })
            })
        })
        .options(opt)
        .build();

    // TODO: setup bot with token from a setting.file
    let token = var("DISCORD_TOKEN").expect("MISSING DISCORD TOKEN!");
    let intents = GatewayIntents::non_privileged();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
