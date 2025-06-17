use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{Context, Error};

/// Defers the response either ephemerally or normally, based on `direct`.
/// Returns `true` if ephemeral defer was used, otherwise `false`.
pub async fn defer_based_on_ephemeral(
    ctx: Context<'_>,
    direct: Option<bool>,
) -> Result<bool, Error> {
    match direct.unwrap_or(false) {
        true => {
            ctx.defer_ephemeral()
                .await
                .map_err(|e| Box::new(e) as Error)?;
            Ok(true)
        }
        false => {
            ctx.defer().await.map_err(|e| Box::new(e) as Error)?;
            Ok(false)
        }
    }
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();
    let author_id = ctx.author().id.to_string();
    Ok(data.config.read().await.admin_list.contains(&author_id))
}

pub async fn is_deepseek(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();
    let author_id = ctx.author().id.to_string();
    Ok(data
        .config
        .read()
        .await
        .deepseek_whitelist
        .contains(&author_id))
}

pub async fn error_and_return<E: std::error::Error + Send + Sync + 'static>(
    ctx: &Context<'_>,
    ephemeral: bool,
    e: E,
) -> Result<(), Error> {
    let _ = ctx
        .send(
            CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .thumbnail(
                            "https://upload.wikimedia.org/wikipedia/commons/5/56/Bsodwindows10.png",
                        )
                        .title("AN ERROR OCCURRED"),
                )
                .ephemeral(ephemeral),
        )
        .await;

    Err(Box::new(e))
}

#[allow(unused)]
pub async fn error(ctx: &Context<'_>, ephemeral: bool) {
    let _ = ctx
        .send(
            CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .thumbnail(
                            "https://upload.wikimedia.org/wikipedia/commons/5/56/Bsodwindows10.png",
                        )
                        .title("AN ERROR OCCURRED"),
                )
                .ephemeral(ephemeral),
        )
        .await;
}

#[allow(unused)]
pub async fn error_and_return_text<E: std::error::Error + Send + Sync + 'static>(
    ctx: &Context<'_>,
    ephemeral: bool,
    e: E,
    text: &str,
) -> Result<(), Error> {
    let _ = ctx
        .send(
            CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .thumbnail(
                            "https://upload.wikimedia.org/wikipedia/commons/5/56/Bsodwindows10.png",
                        )
                        .title("AN ERROR OCCURRED")
                        .description(text),
                )
                .ephemeral(ephemeral),
        )
        .await;

    Err(Box::new(e))
}

#[allow(unused)]
pub async fn error_text(ctx: &Context<'_>, ephemeral: bool, text: &str) {
    let _ = ctx
        .send(
            CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .thumbnail(
                            "https://upload.wikimedia.org/wikipedia/commons/5/56/Bsodwindows10.png",
                        )
                        .title("AN ERROR OCCURRED")
                        .description(text),
                )
                .ephemeral(ephemeral),
        )
        .await;
}
