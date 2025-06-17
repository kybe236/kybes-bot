use std::time::{Duration, SystemTime};

use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::all::{CreateMessage, UserId};
use tokio::fs;

use crate::{
    Context, Error,
    utils::bot::{self, error_text},
};

#[poise::command(slash_command)]
pub async fn reminder(
    ctx: Context<'_>,
    #[description = "When?"] when: String,
    #[description = "What?"] what: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let duration = match humantime::parse_duration(&when) {
        Ok(d) => d,
        Err(_) => {
            error_text(
                &ctx,
                ephemeral,
                "Invalid time format. Use formats like 1h1m1s, 1h10m, 10h, 1d, 1w, or 1y.",
            )
            .await;
            return Ok(());
        }
    };

    let reminder = Reminder {
        time: SystemTime::now() + duration,
        message: what.clone(),
        user_id: ctx.author().id.get(),
        direct: ephemeral,
    };

    let mut reminders = load_reminders().await;
    reminders.push(reminder);
    save_reminders(&reminders).await;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Reminder set for {} from now!",
                humantime::format_duration(duration)
            ))
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn reminders(
    ctx: crate::Context<'_>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let reminders = load_reminders().await;
    let user_id = ctx.author().id.get();

    let user_reminders: Vec<_> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| r.user_id == user_id)
        .collect();

    if user_reminders.is_empty() {
        ctx.send(
            CreateReply::default()
                .content("You have no reminders.")
                .ephemeral(ephemeral),
        )
        .await?;
        return Ok(());
    }

    let mut reply = String::from("Your reminders:\n");
    for (i, reminder) in user_reminders {
        let remaining = reminder
            .time
            .duration_since(SystemTime::now())
            .unwrap_or_default();
        reply.push_str(&format!(
            "`{}`: {} (in {})\n",
            i,
            reminder.message,
            humantime::format_duration(remaining)
        ));
    }

    ctx.send(CreateReply::default().content(reply).ephemeral(ephemeral))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_reminder(
    ctx: crate::Context<'_>,
    #[description = "Reminder index from /list-reminders"] index: usize,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let mut reminders = load_reminders().await;
    let user_id = ctx.author().id.get();

    let user_indices: Vec<_> = reminders
        .iter()
        .enumerate()
        .filter(|(_, r)| r.user_id == user_id)
        .map(|(i, _)| i)
        .collect();

    if index >= user_indices.len() {
        ctx.send(
            CreateReply::default()
                .content("Invalid reminder index.")
                .ephemeral(ephemeral),
        )
        .await?;
        return Ok(());
    }

    let global_index = user_indices[index];
    reminders.remove(global_index);
    save_reminders(&reminders).await;

    ctx.send(
        CreateReply::default()
            .content("Reminder deleted.")
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub time: SystemTime,
    pub message: String,
    pub user_id: u64,
    pub direct: bool,
}

const REMINDERS_PATH: &str = "reminders.json";

async fn load_reminders() -> Vec<Reminder> {
    if let Ok(data) = fs::read_to_string(REMINDERS_PATH).await {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        vec![]
    }
}

async fn save_reminders(reminders: &[Reminder]) {
    if let Ok(data) = serde_json::to_string_pretty(reminders) {
        let _ = fs::write(REMINDERS_PATH, data).await;
    }
}

pub async fn start_reminder_loop(ctx: serenity::all::Context) {
    tokio::spawn(async move {
        loop {
            let reminders = load_reminders().await;
            let now = SystemTime::now();

            let (due, future): (Vec<_>, Vec<_>) =
                reminders.into_iter().partition(|r| r.time <= now);

            for reminder in due {
                let user = ctx.http.get_user(UserId::new(reminder.user_id)).await;
                if let Ok(user) = user {
                    let _ = user
                        .dm(
                            &ctx.http,
                            CreateMessage::default().content(reminder.message.clone()),
                        )
                        .await;
                }
            }

            save_reminders(&future).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
