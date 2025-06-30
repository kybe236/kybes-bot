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

/// Returns server info, filling in defaults if any parameter is None.
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

/// Extract and prepare ping parameters, handling ephemeral defer logic.
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

/// Ping command: attempts to ping a Minecraft server and reply with a summary embed.
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

    // Permissions check
    if !is_ping(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to use ping functionality!",
        )
        .await;
        return Ok(());
    }

    // Perform ping
    let status = match server::ping::ping(&server, port, protocol_version).await {
        Ok(status) => status,
        Err(e) => {
            return error_and_return_text(&ctx, ephemeral, e, "Failed to ping server").await;
        }
    };

    // Build embed message
    let embed = create_server_embed(&status);

    // Attempt to decode favicon if present, attach as image
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

/// Dump Ping command: returns raw ping data JSON as an attachment with a summary embed.
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

    // Permissions check
    if !is_ping(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to use ping functionality!",
        )
        .await;
        return Ok(());
    }

    // Perform ping
    let status = match server::ping::ping(&server, port, protocol_version).await {
        Ok(status) => status,
        Err(e) => {
            return error_and_return_text(&ctx, ephemeral, e, "Failed to ping server").await;
        }
    };

    // Serialize status as pretty JSON
    let json_string = serde_json::to_string_pretty(&status).map_err(|e| {
        warn!("Failed to serialize ping status: {}", e);
        e
    })?;

    // Prepare JSON attachment
    let attachment = CreateAttachment::bytes(json_string.into_bytes(), "ping_dump.json");

    // Create embed for dump message
    let embed = CreateEmbed::default()
        .title("Ping Dump")
        .description(format!("Ping data for server: `{}`", server))
        .color(Colour::DARK_GREEN);

    // Send reply with attachment and embed
    ctx.send(
        CreateReply::default()
            .embed(embed)
            .attachment(attachment)
            .ephemeral(ephemeral),
    )
    .await?;

    Ok(())
}

/// Builds a detailed embed summarizing the Minecraft server status.
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
            format!(
                "```ansi\n{}\n```",
                parse_motd_to_ansi(&server_status.raw_description)
            ),
            false,
        )
        .field(
            "Raw MOTD JSON",
            server_status.raw_description.to_string(),
            false,
        )
        .color(Colour::LIGHT_GREY)
}

/// Helper to send a message with optional attachment and embed.
async fn send_with_embed(
    ctx: &Context<'_>,
    embed: CreateEmbed,
    attachment: Option<CreateAttachment>,
    ephemeral: bool,
) -> Result<(), Error> {
    let mut reply = CreateReply::default().embed(embed).ephemeral(ephemeral);

    if let Some(att) = attachment {
        reply = reply.attachment(att);
    }

    ctx.send(reply).await?;
    Ok(())
}

/// Converts a Minecraft MOTD JSON value into ANSI-colored text for terminals.
/// Supports color codes defined in Minecraft chat JSON format.
fn parse_motd_to_ansi(json: &serde_json::Value) -> String {
    use std::collections::HashMap;

    // Map Minecraft color names to ANSI escape codes
    let ansi_colors: HashMap<&str, &str> = [
        ("white", "\x1b[97m"),
        ("black", "\x1b[30m"),
        ("dark_blue", "\x1b[34m"),
        ("dark_green", "\x1b[32m"),
        ("dark_aqua", "\x1b[36m"),
        ("dark_red", "\x1b[31m"),
        ("dark_purple", "\x1b[35m"),
        ("gold", "\x1b[33m"),
        ("gray", "\x1b[37m"),
        ("dark_gray", "\x1b[90m"),
        ("blue", "\x1b[94m"),
        ("green", "\x1b[92m"),
        ("aqua", "\x1b[96m"),
        ("red", "\x1b[91m"),
        ("light_purple", "\x1b[95m"),
        ("yellow", "\x1b[93m"),
        ("reset", "\x1b[0m"),
    ]
    .into_iter()
    .collect();

    let mut output = String::new();

    if let Some(parts) = json.get("extra").and_then(|v| v.as_array()) {
        for part in parts {
            match part {
                serde_json::Value::Object(obj) => {
                    let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or_default();
                    let color = obj.get("color").and_then(|c| c.as_str()).unwrap_or("reset");
                    let ansi = ansi_colors.get(color).unwrap_or(&ansi_colors["reset"]);
                    output.push_str("\x1b[0m"); // reset before each part
                    output.push_str(ansi);
                    output.push_str(text);
                }
                serde_json::Value::String(s) => {
                    output.push_str(s);
                }
                _ => {}
            }
        }
        output.push_str("\x1b[0m"); // reset at the end
    } else if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
        output.push_str(text);
    }

    output
}
