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
    Ok(data.config.admin_list.contains(&author_id))
}
