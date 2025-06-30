use std::time::Duration;

use poise::CreateReply;

use crate::{
    Context, Error,
    utils::{
        bot::{self, error_text, is_admin},
        git::get_git_hash,
    },
};

/// Replies with the current git hash of the bot.
#[poise::command(slash_command)]
pub async fn version(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    // Send git hash or "INVALID" if unavailable.
    let version = get_git_hash()
        .await
        .unwrap_or_else(|| "INVALID".to_string());

    ctx.send(CreateReply::default().content(version).ephemeral(ephemeral))
        .await?;

    Ok(())
}

/// Stops the bot after a short delay, only usable by admins.
#[poise::command(slash_command)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    // Permission check
    if !is_admin(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to run the /stop command",
        )
        .await;
        return Ok(());
    }

    // Confirm shutdown to user
    ctx.send(
        CreateReply::default()
            .content("bye bye...")
            .ephemeral(ephemeral),
    )
    .await?;

    // Clone shard manager to move into async task
    let shard_manager = ctx.framework().shard_manager().clone();

    // Spawn a task to delay shutdown by 2 seconds to allow message delivery
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        shard_manager.shutdown_all().await;
    });

    Ok(())
}
