use poise::CreateReply;
use serenity::{
    all::{CreateAttachment, CreateEmbed},
    futures::StreamExt,
};

use crate::{
    Context, Error,
    utils::bot::{self, error_and_return, error_text, is_deepseek},
};

#[poise::command(slash_command)]
pub async fn deepseek(
    ctx: Context<'_>,
    #[description = "prompt"] text: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    if !is_deepseek(ctx).await? {
        error_text(&ctx, ephemeral, "You are not allowed to use deepseek!").await;
        return Ok(());
    }

    if text.is_empty() {
        error_text(&ctx, ephemeral, "Please provide a prompt").await;
        return Ok(());
    }

    let api_key = ctx.data().config.read().await.deepseek_token.clone();
    let api_key = match api_key {
        Some(key) => key,
        None => {
            error_text(&ctx, ephemeral, "Sorry! but theres no deepseek key!").await;
            return Ok(());
        }
    };

    let reply = ctx
        .send(
            CreateReply::default()
                .content("please wait...")
                .ephemeral(ephemeral),
        )
        .await?;

    let url = "https://api.deepseek.com/v1/chat/completions";
    let client = reqwest::Client::new();

    let response = match client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "text/event-stream")
        .json(&serde_json::json!({
            "model": "deepseek-chat",
            "messages": [{"role": "user", "content": text}],
            "stream": true
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return error_and_return(&ctx, ephemeral, e).await;
        }
    };

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        error_text(&ctx, ephemeral, &format!("API error: {}", err_text)).await;
        reply.delete(ctx).await?;
        return Ok(());
    }

    let mut stream = response.bytes_stream();
    let mut collected = String::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                let text_chunk = String::from_utf8_lossy(&chunk);

                for line in text_chunk.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            break;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                collected.push_str(content);

                                if collected.len() % 5 == 0 && !collected.is_empty() {
                                    if collected.len() > 1750 {
                                        reply
                                            .edit(
                                                ctx,
                                                CreateReply::default().content(
                                                    collected[..1750].to_string()
                                                        + "\nwait for the rest...",
                                                ),
                                            )
                                            .await?;
                                    } else {
                                        reply
                                            .edit(ctx, CreateReply::default().content(&collected))
                                            .await?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                error_text(&ctx, ephemeral, &format!("Stream error: {}", e)).await;
                break;
            }
        }
    }

    reply.delete(ctx).await?;
    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Deepseek response for prompt:")
                    .description(text),
            )
            .attachment(CreateAttachment::bytes(
                collected.as_bytes(),
                "deepseek_response.txt",
            )),
    )
    .await?;

    Ok(())
}
