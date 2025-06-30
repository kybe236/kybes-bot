use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{Context, Error};

const ERROR_THUMBNAIL: &str =
    "https://upload.wikimedia.org/wikipedia/commons/5/56/Bsodwindows10.png";

/// Defers the response, either ephemerally or normally, based on `direct`.
/// Returns `true` if ephemeral defer was used.
pub async fn defer_based_on_ephemeral(
    ctx: Context<'_>,
    direct: Option<bool>,
) -> Result<bool, Error> {
    let ephemeral = direct.unwrap_or(false);
    if ephemeral {
        ctx.defer_ephemeral().await.map_err(Box::new)?;
    } else {
        ctx.defer().await.map_err(Box::new)?;
    }
    Ok(ephemeral)
}

/// Generic whitelist checker: returns true if feature disabled or user in whitelist.
pub async fn check_whitelist<F, L>(ctx: Context<'_>, is_active: F, list: L) -> Result<bool, Error>
where
    F: Fn(&crate::Config) -> bool,
    L: Fn(&crate::Config) -> &Vec<String>,
{
    let data = ctx.data();
    let cfg = data.config.read().await;
    if !is_active(&cfg) {
        return Ok(true);
    }
    let user = ctx.author().id.to_string();
    Ok(list(&cfg).contains(&user))
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    check_whitelist(ctx, |_| true, |c| &c.admin_list).await
}

pub async fn is_deepseek(ctx: Context<'_>) -> Result<bool, Error> {
    check_whitelist(
        ctx,
        |c| c.deepseek_whitelist_active,
        |c| &c.deepseek_whitelist,
    )
    .await
}

pub async fn is_youtube(ctx: Context<'_>) -> Result<bool, Error> {
    check_whitelist(
        ctx,
        |c| c.youtube_whitelist_active,
        |c| &c.youtube_whitelist,
    )
    .await
}

pub async fn is_ping(ctx: Context<'_>) -> Result<bool, Error> {
    check_whitelist(ctx, |c| c.ping_whitelist_active, |c| &c.ping_whitelist).await
}

/// Sends an error embed, optionally with description, and returns Err(e) if provided.
async fn send_error(ctx: &Context<'_>, ephemeral: bool, title: &str, description: Option<&str>) {
    let mut embed = CreateEmbed::default()
        .thumbnail(ERROR_THUMBNAIL)
        .title(title);
    if let Some(desc) = description {
        embed = embed.description(desc);
    }
    let _ = ctx
        .send(CreateReply::default().embed(embed).ephemeral(ephemeral))
        .await;
}
pub async fn error_and_return<E>(ctx: &Context<'_>, ephemeral: bool, e: E) -> Result<(), Error>
where
    E: std::error::Error + Send + Sync + 'static,
{
    send_error(ctx, ephemeral, "AN ERROR OCCURRED", None).await;
    Err(Box::new(e))
}

pub async fn error(ctx: &Context<'_>, ephemeral: bool) {
    send_error(ctx, ephemeral, "AN ERROR OCCURRED", None).await;
}

pub async fn error_and_return_text<E>(
    ctx: &Context<'_>,
    ephemeral: bool,
    e: E,
    text: &str,
) -> Result<(), Error>
where
    E: std::error::Error + Send + Sync + 'static,
{
    send_error(ctx, ephemeral, "AN ERROR OCCURRED", Some(text)).await;
    Err(Box::new(e))
}

pub async fn error_text(ctx: &Context<'_>, ephemeral: bool, text: &str) {
    send_error(ctx, ephemeral, "AN ERROR OCCURRED", Some(text)).await;
}
