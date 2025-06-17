use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use poise::CreateReply;

use crate::{
    Context, Error,
    utils::bot::{self},
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
