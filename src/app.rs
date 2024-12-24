use axum::extract::FromRef;
use axum_github_webhook_extract::GithubToken;
use jsonwebtoken::EncodingKey;
use octocrab::models::AppId;
use octocrab::Octocrab;
use reqwest::Client;
use std::env;

#[derive(Clone)]
pub struct App {
    pub https: Client,
    pub octocrab: Octocrab,
    pub github_token: GithubToken,
}

impl App {
    pub fn new(github_token: GithubToken) -> Self {
        Self {
            https: Client::new(),
            octocrab: {
                let client_id = env::var("GITHUB_CLIENT_ID")
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
                let rsa_pem = env::var("GITHUB_PRIVATE_KEY").unwrap();

                Octocrab::builder()
                    .app(
                        AppId(client_id),
                        EncodingKey::from_rsa_pem(rsa_pem.as_bytes()).unwrap(),
                    )
                    .build()
                    .unwrap()
            },
            github_token,
        }
    }
}

impl FromRef<App> for GithubToken {
    fn from_ref(state: &App) -> Self {
        state.github_token.clone()
    }
}
