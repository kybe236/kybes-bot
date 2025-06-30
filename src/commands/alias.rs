use std::{collections::HashMap, path::Path};

use once_cell::sync::Lazy;
use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::all::{Colour, CreateEmbed};
use tokio::{fs, sync::RwLock};

use crate::{
    Context, Error,
    utils::bot::{self, error_text},
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

/// Load saved messages from disk into memory at startup.
pub async fn load_messages_from_file() -> Result<(), std::io::Error> {
    if Path::new(SAVE_FILE_PATH).exists() {
        let data = fs::read_to_string(SAVE_FILE_PATH).await?;
        let map: UserMessages = serde_json::from_str(&data)?;
        let mut store = SAVED_MESSAGES.write().await;
        *store = map;
    }
    Ok(())
}

/// Saves all in-memory saved messages to disk as pretty JSON.
async fn save_messages_to_file() -> Result<(), std::io::Error> {
    let store = SAVED_MESSAGES.read().await;
    let json = serde_json::to_string_pretty(&*store)?;
    fs::write(SAVE_FILE_PATH, json).await?;
    Ok(())
}

/// Parses a hex color string (with or without leading '#') into a u32.
/// Returns None if the input is invalid.
fn parse_color(color_str: &str) -> Option<u32> {
    let trimmed = color_str.trim_start_matches('#');
    match trimmed.len() {
        3 => {
            let expanded: String = trimmed
                .chars()
                .flat_map(|c| std::iter::repeat(c).take(2))
                .collect();
            u32::from_str_radix(&expanded, 16).ok()
        }
        6 => u32::from_str_radix(trimmed, 16).ok(),
        _ => None,
    }
}

#[poise::command(slash_command)]
/// Saves a custom message under a user-defined alias.
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

    // Validate color format if provided
    let color_int = if let Some(ref c) = color {
        match parse_color(c) {
            Some(parsed) => Some(parsed),
            None => {
                error_text(
                    &ctx,
                    ephemeral,
                    "Invalid color format. Use 3 or 6 digit hex (with or without '#')",
                )
                .await;
                return Ok(());
            }
        }
    } else {
        None
    };

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

    println!("Saving messages to file...");
    if let Err(e) = save_messages_to_file().await {
        error_text(&ctx, ephemeral, &format!("Failed to save: {}", e)).await;
    } else {
        ctx.send(
            CreateReply::default()
                .content(format!("✅ Saved message with alias `{}`.", alias))
                .ephemeral(ephemeral),
        )
        .await?;
    }
    println!("Messages saved to file successfully.");

    Ok(())
}

#[poise::command(slash_command)]
/// Retrieves and displays a saved message by its alias.
pub async fn alias(
    ctx: Context<'_>,
    #[description = "Alias of the saved message"] alias: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    let user_id = ctx.author().id.get();
    let store = SAVED_MESSAGES.read().await;

    match store.get(&user_id).and_then(|m| m.get(&alias)) {
        Some(saved) => {
            let mut embed = CreateEmbed::default()
                .title(&saved.title)
                .description(saved.content.replace("\\n", "\n"));

            if let Some(ref image_url) = saved.image_url {
                embed = embed.image(image_url);
            }

            if let Some(color) = saved.color {
                embed = embed.color(Colour(color));
            }

            ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
                .await?;
        }
        None => {
            error_text(
                &ctx,
                ephemeral,
                &format!("No saved message found for alias `{}`.", alias),
            )
            .await;
        }
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_alias(
    ctx: Context<'_>,
    #[description = "Alias to delete"] alias: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    let user_id = ctx.author().id.get();
    let removed = {
        let mut store = SAVED_MESSAGES.write().await;
        store
            .get_mut(&user_id)
            .and_then(|m| m.remove(&alias))
            .is_some()
    };

    if !removed {
        error_text(
            &ctx,
            ephemeral,
            &format!("No saved message found for alias `{}`.", alias),
        )
        .await;
        return Ok(());
    }

    match save_messages_to_file().await {
        Ok(_) => {
            ctx.send(
                CreateReply::default()
                    .content(format!("🗑️ Deleted saved message with alias `{}`.", alias))
                    .ephemeral(ephemeral),
            )
            .await?;
        }
        Err(e) => {
            error_text(&ctx, ephemeral, &format!("Failed to save deletion: {}", e)).await;
        }
    }

    Ok(())
}

#[poise::command(slash_command)]
/// Lists all saved aliases for the user.
pub async fn list_alias(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    let user_id = ctx.author().id.get();
    let store = SAVED_MESSAGES.read().await;

    let aliases = store
        .get(&user_id)
        .map(|m| m.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    if aliases.is_empty() {
        ctx.send(
            CreateReply::default()
                .content("You have no saved aliases.")
                .ephemeral(ephemeral),
        )
        .await?;
    } else {
        let embed = CreateEmbed::default()
            .title("Your saved aliases")
            .description(aliases.join(", "));

        ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
            .await?;
    }

    Ok(())
}
