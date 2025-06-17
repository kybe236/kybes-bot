use reqwest::Client;
use serenity::all::CreateAttachment;

use crate::{
    Context, Error,
    utils::bot::{self, error_text},
};

#[poise::command(slash_command)]
pub async fn cat(
    ctx: Context<'_>,
    #[description = "How many cats? (1-6)"] count: Option<u8>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let count = count.unwrap_or(1);

    if !(1..=6).contains(&count) {
        error_text(&ctx, ephemeral, "Count must be between 1 and 6.").await;
        return Ok(());
    }

    for _ in 0..count {
        fetch_and_send_cat_image(&ctx, ephemeral).await?;
    }

    Ok(())
}

async fn fetch_and_send_cat_image(ctx: &Context<'_>, ephemeral: bool) -> Result<(), Error> {
    let client = Client::new();
    let response = client.get("https://cataas.com/cat").send().await?;

    if response.status().is_success() {
        let image_bytes = response.bytes().await?;
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(ephemeral)
                .attachment(CreateAttachment::bytes(image_bytes, "cat.jpg")),
        )
        .await?;
    } else {
        error_text(ctx, ephemeral, "Failed to fetch a cat image.").await;
    }
    Ok(())
}
