use poise::CreateReply;

use crate::{
    Context, Error,
    utils::bot::{self},
};

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
