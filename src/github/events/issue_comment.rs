use crate::utils::extract_owner_and_repo;
use chrono::{DateTime, Duration, Utc};
use hex_literal::hex;
use octocrab::models::webhook_events::payload::{
    IssueCommentWebhookEventAction, IssueCommentWebhookEventPayload,
};
use octocrab::Octocrab;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub async fn handle(
    payload: Box<IssueCommentWebhookEventPayload>,
    https: &Client,
    octocrab: &Octocrab,
) -> anyhow::Result<()> {
    match payload.action {
        IssueCommentWebhookEventAction::Created => {
            let user = payload.comment.user;
            let username = user.login;

            let url = format!("https://api.github.com/users/{}", username);
            let creation_date = https
                .get(url)
                .header("User-Agent", "create-triage-bot")
                .send()
                .await?
                .json::<GithubUser>()
                .await?
                .created_at;

            if Utc::now() - creation_date <= Duration::days(10) {
                let avatar_url = user.avatar_url;

                let image_bytes = https.get(avatar_url).send().await?.bytes().await?;

                let hashed =
                    tokio::task::spawn_blocking(move || Sha256::digest(&image_bytes)).await?;

                if hashed[..]
                    == hex!("dfdad7b099e4b311efca730cff45bf3ed08cce63b5358e7da587679a2beb2d83")
                {
                    let (owner, repo) =
                        extract_owner_and_repo(payload.issue.repository_url).unwrap();

                    let installation = octocrab
                        .apps()
                        .get_repository_installation(&owner, &repo)
                        .await?;
                    let installation_handler = octocrab.installation(installation.id)?;
                    let issue_handler = installation_handler.issues(&owner, repo);

                    issue_handler.delete_comment(payload.comment.id).await?;
                }
            }

            Ok(())
        },
        _ => Ok(()),
    }
}

#[derive(Deserialize)]
struct GithubUser {
    created_at: DateTime<Utc>,
}
