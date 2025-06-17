use std::{collections::HashMap, path::Path};

use once_cell::sync::Lazy;
use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::all::{Colour, CreateEmbed};
use tokio::{fs, sync::RwLock};

use crate::{
    Context, Error,
    utils::bot::{self, error_and_return, error_text},
};

#[derive(Serialize, Deserialize, Clone)]
struct SavedMessage {
    title: String,
    content: String,
    image_url: Option<String>,
    color: Option<u32>,
}

type UserMessages = HashMap<u64, HashMap<String, SavedMessage>>;

static SAVED_MESSAGES: Lazy<RwLock<UserMessages>> = Lazy::new(|| RwLock::new(HashMap::new()));
const SAVE_FILE_PATH: &str = "saved_messages.json";

pub async fn load_saved_messages() -> Result<(), std::io::Error> {
    if !Path::new(SAVE_FILE_PATH).exists() {
        return Ok(());
    }
    let data = fs::read_to_string(SAVE_FILE_PATH).await?;
    let map: UserMessages = serde_json::from_str(&data)?;
    let mut store = SAVED_MESSAGES.write().await;
    *store = map;
    Ok(())
}

pub async fn save_messages_to_file() -> Result<(), std::io::Error> {
    let store = SAVED_MESSAGES.read().await;
    let json = serde_json::to_string_pretty(&*store)?;
    fs::write(SAVE_FILE_PATH, json).await?;
    Ok(())
}

fn parse_color(color_str: &str) -> Option<u32> {
    let trimmed = color_str.trim_start_matches('#');
    u32::from_str_radix(trimmed, 16).ok()
}

#[poise::command(slash_command)]
pub async fn save_alias(
    ctx: Context<'_>,
    #[description = "Alias to save this message under"] alias: String,
    #[description = "Title of the message"] title: String,
    #[description = "Content of the message"] content: String,
    #[description = "Optional image URL for the embed"] image_url: Option<String>,
    #[description = "Optional hex color for the embed, e.g. #FF0000 or FF0000"] color: Option<
        String,
    >,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    let user_id = ctx.author().id.get();
    let color_int = color.as_deref().and_then(parse_color);

    {
        let mut store = SAVED_MESSAGES.write().await;
        let user_map = store.entry(user_id).or_default();
        user_map.insert(alias.clone(), SavedMessage {
            title,
            content,
            image_url,
            color: color_int,
        });
    }

    if let Err(e) = save_messages_to_file().await {
        return error_and_return(&ctx, ephemeral, e).await;
    }

    ctx.send(
        CreateReply::default()
            .content(format!("✅ Saved message with alias `{}`.", alias))
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn alias(
    ctx: Context<'_>,
    #[description = "Alias of the saved message"] alias: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    load_saved_messages().await?;
    let user_id = ctx.author().id.get();
    let store = SAVED_MESSAGES.read().await;

    if let Some(user_map) = store.get(&user_id) {
        if let Some(saved) = user_map.get(&alias) {
            let mut embed = CreateEmbed::default()
                .title(saved.title.clone())
                .description(saved.content.replace("\\n", "\n"));

            if let Some(image_url) = &saved.image_url {
                embed = embed.image(image_url);
            }
            if let Some(color) = saved.color {
                embed = embed.color(Colour(color));
            }

            ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
                .await?;
            return Ok(());
        }
    }

    error_text(
        &ctx,
        ephemeral,
        &format!("No saved message found for alias `{}`.", alias),
    )
    .await;

    Ok(())
}
