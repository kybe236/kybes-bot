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

#[derive(Deserialize)]
struct GitHubLicense {
    name: Option<String>,
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
    let res = match client
        .get(&url)
        .header("User-Agent", "poise-bot")
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            return error_and_return(&ctx, ephemeral, e).await;
        }
    };

    if res.status().is_success() {
        let embed = if is_repo {
            let repo: GitHubRepo = match res.json().await {
                Ok(repo) => repo,
                Err(e) => {
                    return error_and_return(&ctx, ephemeral, e).await;
                }
            };
            let mut embed = CreateEmbed::default()
                .title(repo.full_name.clone().unwrap_or_default())
                .url(repo.html_url.clone().unwrap_or_default());

            if let Some(desc) = &repo.description {
                embed = embed.description(desc);
            }

            if let Some(stars) = repo.stargazers_count {
                embed = embed.field("Stars", stars.to_string(), true);
            }
            if let Some(watchers) = repo.watchers_count {
                embed = embed.field("Watchers", watchers.to_string(), true);
            }
            if let Some(forks) = repo.forks_count {
                embed = embed.field("Forks", forks.to_string(), true);
            }
            if let Some(open_issues) = repo.open_issues_count {
                embed = embed.field("Open Issues", open_issues.to_string(), true);
            }
            if let Some(language) = &repo.language {
                embed = embed.field("Language", language, true);
            }
            if let Some(private) = repo.private {
                embed = embed.field("Private", private.to_string(), true);
            }
            if let Some(fork) = repo.fork {
                embed = embed.field("Forked Repo", fork.to_string(), true);
            }
            if let Some(homepage) = &repo.homepage {
                if !homepage.is_empty() {
                    embed = embed.field("Homepage", homepage, false);
                }
            }
            if let Some(created) = &repo.created_at {
                embed = embed.field("Created At", created, true);
            }
            if let Some(updated) = &repo.updated_at {
                embed = embed.field("Last Updated", updated, true);
            }
            if let Some(pushed) = &repo.pushed_at {
                embed = embed.field("Last Push", pushed, true);
            }
            if let Some(license) = &repo.license {
                if let Some(name) = &license.name {
                    embed = embed.field("📄 License", name, true);
                }
            }
            if let Some(owner) = &repo.owner {
                if let Some(avatar) = &owner.avatar_url {
                    embed = embed.thumbnail(avatar.clone());
                }
            }
            embed
        } else {
            let user: GitHubUser = match res.json().await {
                Ok(user) => user,
                Err(e) => {
                    return error_and_return(&ctx, ephemeral, e).await;
                }
            };

            let mut embed = CreateEmbed::default()
                .title(user.login.clone().unwrap_or_default())
                .url(user.html_url.clone().unwrap_or_default());

            if let Some(avatar) = &user.avatar_url {
                embed = embed.thumbnail(avatar.clone());
            }
            if let Some(public_repos) = user.public_repos {
                embed = embed.field("Public Repos", public_repos.to_string(), true);
            }
            if let Some(followers) = user.followers {
                embed = embed.field("Followers", followers.to_string(), true);
            }
            if let Some(name) = &user.name {
                embed = embed.field("Name", name.clone(), true);
            }
            if let Some(company) = &user.company {
                embed = embed.field("Company", company.clone(), true);
            }
            if let Some(blog) = &user.blog {
                if !blog.is_empty() {
                    embed = embed.field("Blog", blog.clone(), false);
                }
            }
            if let Some(location) = &user.location {
                embed = embed.field("Location", location.clone(), true);
            }
            if let Some(email) = &user.email {
                embed = embed.field("Email", email.clone(), true);
            }
            if let Some(bio) = &user.bio {
                embed = embed.description(bio.clone());
            }
            if let Some(twitter) = &user.twitter_username {
                embed = embed.field("Twitter", format!("@{}", twitter), true);
            }
            if let Some(created) = &user.created_at {
                embed = embed.field("Account Created", created.clone(), true);
            }
            if let Some(updated) = &user.updated_at {
                embed = embed.field("Last Updated", updated.clone(), true);
            }

            embed
        };

        ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral))
            .await?;
    } else {
        error_text(&ctx, ephemeral, "GitHub user or repository not found.").await;
    }
    Ok(())
}
