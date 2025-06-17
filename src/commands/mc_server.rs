use base64::{Engine, prelude::BASE64_STANDARD};
use poise::CreateReply;
use serenity::all::{Colour, CreateAttachment, CreateEmbed};

use tracing::warn;

use crate::{
    Context, Error,
    utils::{
        bot::{self, error_and_return_text, error_text, is_ping},
        server::{self, ping::ServerStatus},
    },
};

const DEFAULT_SERVER: &str = "2b2t.org";
const DEFAULT_PORT: u16 = 25565;
const DEFAULT_PROTOCOL_VERSION: i32 = 770;

fn default_server_info(
    server: Option<String>,
    port: Option<u16>,
    protocol_version: Option<i32>,
) -> (String, u16, i32) {
    (
        server.unwrap_or_else(|| DEFAULT_SERVER.to_string()),
        port.unwrap_or(DEFAULT_PORT),
        protocol_version.unwrap_or(DEFAULT_PROTOCOL_VERSION),
    )
}

async fn extract_ping_params(
    ctx: &Context<'_>,
    ephemeral: Option<bool>,
    server: Option<String>,
    port: Option<u16>,
    protocol_version: Option<i32>,
) -> Result<(bool, String, u16, i32), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(*ctx, ephemeral).await?;
    let (server, port, protocol_version) = default_server_info(server, port, protocol_version);
    Ok((ephemeral, server, port, protocol_version))
}

#[poise::command(slash_command)]
pub async fn ping(
    ctx: Context<'_>,
    #[description = "Server hostname or IP"] server: Option<String>,
    #[description = "Server port"] port: Option<u16>,
    #[description = "Minecraft protocol version"] protocol_version: Option<i32>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let (ephemeral, server, port, protocol_version) =
        extract_ping_params(&ctx, ephemeral, server, port, protocol_version).await?;

    if !is_ping(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to use ping functionality!",
        )
        .await;
        return Ok(());
    }

    let status = match server::ping::ping(&server, port, protocol_version).await {
        Ok(status) => status,
        Err(e) => return error_and_return_text(&ctx, ephemeral, e, "Failed to ping server").await,
    };

    let embed = create_server_embed(&status);

    let attachment = status.favicon.as_ref().and_then(|favicon| {
        let base64_str = favicon
            .strip_prefix("data:image/png;base64,")
            .unwrap_or(favicon);
        match BASE64_STANDARD.decode(base64_str) {
            Ok(image_bytes) => Some(CreateAttachment::bytes(image_bytes, "favicon.png")),
            Err(e) => {
                warn!("Failed to decode favicon base64: {}", e);
                None
            }
        }
    });

    send_with_embed(&ctx, embed, attachment, ephemeral).await
}

#[poise::command(slash_command)]
pub async fn dump_ping(
    ctx: Context<'_>,
    #[description = "Server hostname or IP"] server: Option<String>,
    #[description = "Server port"] port: Option<u16>,
    #[description = "Minecraft protocol version"] protocol_version: Option<i32>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let (ephemeral, server, port, protocol_version) =
        extract_ping_params(&ctx, ephemeral, server, port, protocol_version).await?;

    if !is_ping(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to use ping functionality!",
        )
        .await;
        return Ok(());
    }

    let status = match server::ping::ping(&server, port, protocol_version).await {
        Ok(status) => status,
        Err(e) => return error_and_return_text(&ctx, ephemeral, e, "Failed to ping server").await,
    };

    let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
        warn!("Failed to serialize ping status: {}", e);
        e
    })?;

    let attachment = CreateAttachment::bytes(json_string.into_bytes(), "ping_dump.json");

    let embed = CreateEmbed::default()
        .title("Ping Dump")
        .description(format!("Ping data for server: `{}`", server))
        .color(0x00FF00);

    ctx.send(
        CreateReply::default()
            .embed(embed)
            .attachment(attachment)
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

pub fn create_server_embed(server_status: &ServerStatus) -> CreateEmbed {
    CreateEmbed::default()
        .title("Server Status")
        .field(
            "Version",
            format!(
                "{} (protocol {})",
                server_status.version.name, server_status.version.protocol
            ),
            false,
        )
        .field(
            "Players",
            format!(
                "{}/{}",
                server_status.players.online, server_status.players.max
            ),
            false,
        )
        .field(
            "MOTD (ANSI)",
            "```ansi\n".to_string() + &parse_motd_to_ansi(&server_status.raw_description) + "\n```",
            false,
        )
        .field("RAW MOTD", server_status.raw_description.to_string(), false)
        .color(Colour::LIGHT_GREY)
}

async fn send_with_embed(
    ctx: &Context<'_>,
    embed: CreateEmbed,
    attachment: Option<CreateAttachment>,
    ephemeral: bool,
) -> Result<(), Error> {
    if let Some(att) = attachment {
        let reply = CreateReply::default()
            .embed(embed.attachment(att.filename.clone()))
            .ephemeral(ephemeral)
            .attachment(att);
        ctx.send(reply).await?;
        return Ok(());
    }

    let reply = CreateReply::default().embed(embed).ephemeral(ephemeral);
    ctx.send(reply).await?;
    Ok(())
}

fn parse_motd_to_ansi(json: &serde_json::Value) -> String {
    use std::collections::HashMap;

    let mut ansi_colors = HashMap::new();
    ansi_colors.insert("white", "\x1b[97m");
    ansi_colors.insert("black", "\x1b[30m");
    ansi_colors.insert("dark_blue", "\x1b[34m");
    ansi_colors.insert("dark_green", "\x1b[32m");
    ansi_colors.insert("dark_aqua", "\x1b[36m");
    ansi_colors.insert("dark_red", "\x1b[31m");
    ansi_colors.insert("dark_purple", "\x1b[35m");
    ansi_colors.insert("gold", "\x1b[33m");
    ansi_colors.insert("gray", "\x1b[37m");
    ansi_colors.insert("dark_gray", "\x1b[90m");
    ansi_colors.insert("blue", "\x1b[94m");
    ansi_colors.insert("green", "\x1b[92m");
    ansi_colors.insert("aqua", "\x1b[96m");
    ansi_colors.insert("red", "\x1b[91m");
    ansi_colors.insert("light_purple", "\x1b[95m");
    ansi_colors.insert("yellow", "\x1b[93m");
    ansi_colors.insert("reset", "\x1b[0m");

    let mut out = String::new();
    let extra = json.get("extra");

    if let Some(parts) = extra.and_then(|v| v.as_array()) {
        for part in parts {
            match part {
                serde_json::Value::Object(obj) => {
                    let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or_default();
                    let color = obj.get("color").and_then(|c| c.as_str()).unwrap_or("reset");
                    let ansi = ansi_colors.get(color).unwrap_or(&ansi_colors["reset"]);
                    out.push_str("\x1b[0m");
                    out.push_str(ansi);
                    out.push_str(text);
                }
                serde_json::Value::String(s) => {
                    out.push_str(s);
                }
                _ => {}
            }
        }
        out.push_str("\x1b[0m");
    } else if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
        out.push_str(text);
    }

    out
}
