use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use poise::CreateReply;
use reqwest::Client;
use serde_json::Value;

use crate::{
    Context, Error,
    utils::bot::{self, error_and_return, error_text, is_admin},
};

#[poise::command(slash_command)]
pub async fn time(
    ctx: Context<'_>,
    #[description = "What timezone to use?"] timezone: Option<String>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let utc_now: DateTime<Utc> = Utc::now();

    let (formatted_time, tz_display) = match timezone {
        Some(ref tz_string) => match tz_string.parse::<Tz>() {
            Ok(tz) => {
                let time_in_tz = utc_now.with_timezone(&tz);
                (
                    time_in_tz.format("%d.%m.%Y %H:%M:%S").to_string(),
                    tz.name().to_string(),
                )
            }
            Err(_) => {
                let local = chrono::Local::now();
                (
                    local.format("%d.%m.%Y %H:%M:%S").to_string(),
                    local.offset().to_string(),
                )
            }
        },
        None => {
            let local = chrono::Local::now();
            (
                local.format("%d.%m.%Y %H:%M:%S").to_string(),
                local.offset().to_string(),
            )
        }
    };

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Current time: {} in {}",
                formatted_time, tz_display
            ))
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn test(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    ctx.send(
        CreateReply::default()
            .content("HELLO WORLD FROM 2kybe3!")
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn reload_settings(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    if !is_admin(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to run the /reload_settings command",
        )
        .await;
        return Ok(());
    }

    ctx.data()
        .config
        .write()
        .await
        .reload("config.json")
        .await?;

    ctx.send(
        CreateReply::default()
            .content("reloaded config!")
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn print(
    ctx: Context<'_>,
    print: String,
    #[description = "Auto Delete"] auto_delete: Option<bool>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;
    let auto_delete = auto_delete.unwrap_or(false);

    let hide = ctx
        .send(CreateReply::default().content(".").ephemeral(true))
        .await?;
    let msg = ctx
        .send(CreateReply::default().content(print).ephemeral(ephemeral))
        .await?;
    hide.delete(ctx).await?;
    if auto_delete {
        msg.delete(ctx).await?;
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "Text to translate"] text: String,
    #[description = "Language code (e.g., 'en', 'fr')"] lang: Option<String>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        lang.unwrap_or("en".to_string()),
        urlencoding::encode(&text)
    );

    let client = Client::new();
    let res = match client.get(&url).send().await {
        Ok(res) => res,
        Err(e) => {
            return error_and_return(&ctx, ephemeral, e).await;
        }
    };
    let res: Value = match res.json().await {
        Ok(res) => res,
        Err(e) => {
            return error_and_return(&ctx, ephemeral, e).await;
        }
    };

    if let Some(translated) = res[0][0][0].as_str() {
        ctx.send(
            CreateReply::default()
                .ephemeral(ephemeral)
                .content(translated),
        )
        .await?;
    } else {
        error_text(&ctx, ephemeral, "Empty Answer").await;
    }

    Ok(())
}
