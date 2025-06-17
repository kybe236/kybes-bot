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
