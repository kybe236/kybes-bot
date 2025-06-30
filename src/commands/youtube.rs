use once_cell::sync::Lazy;
use poise::CreateReply;
use regex::Regex;
use serde::Deserialize;
use serenity::all::{Color, CreateEmbed};

use crate::{
    Context, Error,
    utils::bot::{self, error_and_return, error_text, is_youtube},
};

// Compile regex once
static YT_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:v=|\/)([0-9A-Za-z_-]{11})").expect("Invalid regex"));

#[poise::command(slash_command)]
pub async fn yt_vid(
    ctx: Context<'_>,
    #[description = "YouTube video URL"] url: String,
    #[description = "Show video description?"] show_description: Option<bool>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    if !is_youtube(ctx).await? {
        error_text(
            &ctx,
            ephemeral,
            "You are not allowed to use the YouTube API!",
        )
        .await;
        return Ok(());
    }

    let video_id = match YT_ID_REGEX.captures(&url).and_then(|caps| caps.get(1)) {
        Some(m) => m.as_str(),
        None => {
            error_text(&ctx, ephemeral, "Invalid YouTube URL provided").await;
            return Ok(());
        }
    };

    let config = ctx.data().config.read().await;

    let api_key = match config.youtube_token.as_deref() {
        Some(key) if !key.is_empty() => key.to_owned(), // Clone key to own the string
        _ => {
            error_text(&ctx, ephemeral, "No YouTube API key configured").await;
            return Ok(());
        }
    };

    let api_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=snippet,statistics&id={video_id}&key={api_key}"
    );

    let response = match reqwest::get(&api_url).await {
        Ok(resp) => resp,
        Err(e) => {
            error_and_return(&ctx, ephemeral, e).await?;
            return Ok(());
        }
    };

    let yt_response: YouTubeResponse = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            error_and_return(&ctx, ephemeral, e).await?;
            return Ok(());
        }
    };

    let video = match yt_response.items.first() {
        Some(video) => video,
        None => {
            error_text(&ctx, ephemeral, "Video not found").await;
            return Ok(());
        }
    };

    let link = format!("https://youtu.be/{}", video.id);

    let views = video.statistics.view_count.parse::<f64>().unwrap_or(0.0);
    let likes = video
        .statistics
        .like_count
        .as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let like_ratio = if views > 0.0 {
        (likes / views) * 100.0
    } else {
        0.0
    };

    let mut embed = CreateEmbed::default()
        .title(&video.snippet.title)
        .url(&link)
        .thumbnail(&video.snippet.thumbnails.high.url)
        .field("Channel", &video.snippet.channel_title, true)
        .field("Published", &video.snippet.published_at[..10], true)
        .field("Views", &video.statistics.view_count, true)
        .field(
            "Likes",
            video.statistics.like_count.as_deref().unwrap_or("N/A"),
            true,
        )
        .field(
            "Comments",
            video.statistics.comment_count.as_deref().unwrap_or("N/A"),
            true,
        )
        .field("Like View Ratio", format!("{:.2}%", like_ratio), true)
        .color(Color::RED);

    if show_description.unwrap_or(false) {
        embed = embed.description(&video.snippet.description);
    }

    ctx.send(CreateReply::default().ephemeral(ephemeral).embed(embed))
        .await?;

    Ok(())
}

#[derive(Deserialize)]
struct YouTubeResponse {
    items: Vec<YouTubeItem>,
}

#[derive(Deserialize)]
struct YouTubeItem {
    id: String,
    snippet: Snippet,
    statistics: Statistics,
}

#[derive(Deserialize)]
struct Snippet {
    title: String,
    #[serde(rename = "channelTitle")]
    channel_title: String,
    #[serde(rename = "publishedAt")]
    published_at: String,
    description: String,
    thumbnails: Thumbnails,
}

#[derive(Deserialize)]
struct Thumbnails {
    #[serde(rename = "high")]
    high: Thumbnail,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
}

#[derive(Deserialize)]
struct Statistics {
    #[serde(rename = "viewCount")]
    view_count: String,
    #[serde(rename = "likeCount")]
    like_count: Option<String>,
    #[serde(rename = "commentCount")]
    comment_count: Option<String>,
}
