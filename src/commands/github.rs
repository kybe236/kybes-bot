use poise::CreateReply;
use serde::Deserialize;
use serenity::all::CreateEmbed;

use crate::{
    Context, Error,
    utils::bot::{self, error_and_return, error_text},
};

#[derive(Deserialize)]
struct GitHubUser {
    login: Option<String>,
    public_repos: Option<u32>,
    followers: Option<u32>,
    avatar_url: Option<String>,
    html_url: Option<String>,
    name: Option<String>,
    company: Option<String>,
    blog: Option<String>,
    location: Option<String>,
    email: Option<String>,
    bio: Option<String>,
    twitter_username: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct GitHubLicense {
    name: Option<String>,
}

#[derive(Deserialize)]
struct GitHubRepo {
    #[allow(unused)]
    name: Option<String>,
    full_name: Option<String>,
    private: Option<bool>,
    html_url: Option<String>,
    description: Option<String>,
    fork: Option<bool>,
    language: Option<String>,
    stargazers_count: Option<u32>,
    watchers_count: Option<u32>,
    forks_count: Option<u32>,
    open_issues_count: Option<u32>,
    homepage: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    pushed_at: Option<String>,
    license: Option<GitHubLicense>,
    owner: Option<GitHubUser>,
}

// Updated to return the updated embed, because .field() consumes and returns new CreateEmbed
fn add_field_if_some(
    embed: CreateEmbed,
    name: &str,
    value: Option<impl ToString>,
    inline: bool,
) -> CreateEmbed {
    if let Some(v) = value {
        let v_str = v.to_string();
        if !v_str.is_empty() {
            return embed.field(name, v_str, inline);
        }
    }
    embed
}

#[poise::command(slash_command)]
pub async fn github(
    ctx: Context<'_>,
    #[description = "Username or username/repo"] query: String,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let is_repo = query.contains('/');
    let url = if is_repo {
        format!("https://api.github.com/repos/{}", query)
    } else {
        format!("https://api.github.com/users/{}", query)
    };

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("User-Agent", "poise-bot")
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Request failed: {}", e);
            e
        })?;

    if !res.status().is_success() {
        error_text(&ctx, ephemeral, "GitHub user or repository not found.").await;
        return Ok(());
    }

    if is_repo {
        let repo: GitHubRepo = match res.json().await {
            Ok(repo) => repo,
            Err(e) => return error_and_return(&ctx, ephemeral, e).await,
        };

        let mut embed = CreateEmbed::default()
            .title(repo.full_name.clone().unwrap_or_default())
            .url(repo.html_url.clone().unwrap_or_default());

        if let Some(owner) = &repo.owner {
            if let Some(avatar) = &owner.avatar_url {
                embed = embed.thumbnail(avatar.clone());
            }
        }

        if let Some(desc) = &repo.description {
            embed = embed.description(desc);
        }

        embed = add_field_if_some(embed, "Stars", repo.stargazers_count, true);
        embed = add_field_if_some(embed, "Watchers", repo.watchers_count, true);
        embed = add_field_if_some(embed, "Forks", repo.forks_count, true);
        embed = add_field_if_some(embed, "Open Issues", repo.open_issues_count, true);
        embed = add_field_if_some(embed, "Language", repo.language.clone(), true);
        embed = add_field_if_some(embed, "Private", repo.private.map(|b| b.to_string()), true);
        embed = add_field_if_some(embed, "Forked Repo", repo.fork.map(|b| b.to_string()), true);

        if let Some(homepage) = &repo.homepage {
            if !homepage.is_empty() {
                embed = embed.field("Homepage", homepage, false);
            }
        }

        embed = add_field_if_some(embed, "Created At", repo.created_at.clone(), true);
        embed = add_field_if_some(embed, "Last Updated", repo.updated_at.clone(), true);
        embed = add_field_if_some(embed, "Last Push", repo.pushed_at.clone(), true);

        if let Some(license) = &repo.license {
            embed = add_field_if_some(embed, "📄 License", license.name.clone(), true);
        }

        ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
            .await?;
    } else {
        let user: GitHubUser = match res.json().await {
            Ok(user) => user,
            Err(e) => return error_and_return(&ctx, ephemeral, e).await,
        };

        let mut embed = CreateEmbed::default()
            .title(user.login.clone().unwrap_or_default())
            .url(user.html_url.clone().unwrap_or_default());

        if let Some(avatar) = &user.avatar_url {
            embed = embed.thumbnail(avatar.clone());
        }

        embed = add_field_if_some(embed, "Public Repos", user.public_repos, true);
        embed = add_field_if_some(embed, "Followers", user.followers, true);
        embed = add_field_if_some(embed, "Name", user.name.clone(), true);
        embed = add_field_if_some(embed, "Company", user.company.clone(), true);

        if let Some(blog) = &user.blog {
            if !blog.is_empty() {
                embed = embed.field("Blog", blog.clone(), false);
            }
        }

        embed = add_field_if_some(embed, "Location", user.location.clone(), true);
        embed = add_field_if_some(embed, "Email", user.email.clone(), true);

        if let Some(bio) = &user.bio {
            embed = embed.description(bio.clone());
        }

        if let Some(twitter) = &user.twitter_username {
            embed = embed.field("Twitter", format!("@{}", twitter), true);
        }

        embed = add_field_if_some(embed, "Account Created", user.created_at.clone(), true);
        embed = add_field_if_some(embed, "Last Updated", user.updated_at.clone(), true);

        ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
            .await?;
    }

    Ok(())
}
