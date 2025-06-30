use std::{io::Write, path::Path};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub discord_token: String,
    pub deepseek_token: Option<String>,
    pub youtube_token: Option<String>,
    pub youtube_whitelist_active: bool,
    pub youtube_whitelist: Vec<String>,
    pub admin_list: Vec<String>,
    pub deepseek_whitelist_active: bool,
    pub deepseek_whitelist: Vec<String>,
    pub ping_whitelist_active: bool,
    pub ping_whitelist: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discord_token: String::new(),
            youtube_token: None,
            deepseek_token: None,
            youtube_whitelist_active: false,
            deepseek_whitelist_active: true,
            ping_whitelist_active: false,
            admin_list: vec!["921066050009833572".into()],
            youtube_whitelist: vec!["921066050009833572".into()],
            deepseek_whitelist: vec!["921066050009833572".into()],
            ping_whitelist: vec!["921066050009833572".into()],
        }
    }
}

impl Config {
    pub async fn load_or_create(path: &str) -> tokio::io::Result<Self> {
        if Path::new(path).exists() {
            let data = tokio::fs::read_to_string(path).await?;
            Ok(serde_json::from_str(&data)?)
        } else {
            let discord_token = Self::ask("DISCORD TOKEN").await?;
            let youtube_token = Self::ask_optional(
                "YOUTUBE TOKEN",
                Some("https://console.cloud.google.com/apis/library/youtube.googleapis.com"),
            )
            .await?;
            let deepseek_token = Self::ask_optional(
                "DEEPSEEK TOKEN",
                Some("https://platform.deepseek.com/api_keys"),
            )
            .await?;

            let config = Self {
                discord_token,
                youtube_token,
                deepseek_token,
                ..Default::default()
            };

            config.save(path).await?;
            Ok(config)
        }
    }

    async fn ask(prompt: &str) -> tokio::io::Result<String> {
        print!("{prompt} => ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        BufReader::new(tokio::io::stdin())
            .read_line(&mut input)
            .await?;
        Ok(input.trim().to_owned())
    }

    async fn ask_optional(
        prompt: &str,
        help_url: Option<&str>,
    ) -> tokio::io::Result<Option<String>> {
        if let Some(url) = help_url {
            println!("{url}");
            println!("Keep empty to not set.");
        }

        let value = Self::ask(prompt).await?;
        Ok(if value.is_empty() { None } else { Some(value) })
    }

    pub async fn save(&self, path: &str) -> tokio::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, json).await
    }

    pub async fn reload(&mut self, path: &str) -> tokio::io::Result<()> {
        let data = tokio::fs::read_to_string(path).await?;
        *self = serde_json::from_str(&data)?;
        Ok(())
    }
}
