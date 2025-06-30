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
    #[description = "Prompt"] text: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    // Authorization check
    if !is_deepseek(ctx).await? {
        error_text(&ctx, ephemeral, "You are not allowed to use deepseek!").await;
        return Ok(());
    }

    if text.trim().is_empty() {
        error_text(&ctx, ephemeral, "Please provide a prompt").await;
        return Ok(());
    }

    // Get API key safely
    let api_key = match ctx.data().config.read().await.deepseek_token.clone() {
        Some(key) => key,
        None => {
            error_text(&ctx, ephemeral, "Sorry! but there's no deepseek key!").await;
            return Ok(());
        }
    };

    // Initial reply (deferred) to user
    let reply = ctx
        .send(
            CreateReply::default()
                .content("Please wait...")
                .ephemeral(ephemeral),
        )
        .await?;

    // Setup request
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "text/event-stream")
        .json(&serde_json::json!({
            "model": "deepseek-chat",
            "messages": [{"role": "user", "content": text}],
            "stream": true
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Deepseek request error: {}", e);
            e
        });

    let response = match response {
        Ok(resp) => resp,
        Err(e) => return error_and_return(&ctx, ephemeral, e).await,
    };

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        error_text(&ctx, ephemeral, &format!("API error: {}", err_text)).await;
        reply.delete(ctx).await.ok();
        return Ok(());
    }

    // Stream the response incrementally
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

                                // Update reply every 5 characters or when big enough
                                if collected.len() % 5 == 0 && !collected.is_empty() {
                                    let content_to_send = if collected.len() > 1750 {
                                        format!("{}\n(wait for the rest...)", &collected[..1750])
                                    } else {
                                        collected.clone()
                                    };
                                    reply
                                        .edit(ctx, CreateReply::default().content(content_to_send))
                                        .await?;
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

    reply.delete(ctx).await.ok();

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
