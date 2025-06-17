use std::time::Duration;

use poise::CreateReply;

use crate::{
    Context, Error,
    utils::{
        bot::{self, error_text, is_admin},
        git::get_git_hash,
    },
};

#[poise::command(slash_command)]
pub async fn version(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    ctx.send(
        CreateReply::default()
            .content(get_git_hash().await.unwrap_or("INVALID".to_string()))
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    if !is_admin(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to run the /stop command",
        )
        .await;
        return Ok(());
    }

    ctx.send(
        CreateReply::default()
            .content("bye bye...")
            .ephemeral(ephemeral),
    )
    .await?;

    let shard_manager = ctx.framework().shard_manager().clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        shard_manager.shutdown_all().await;
    });

    Ok(())
}
