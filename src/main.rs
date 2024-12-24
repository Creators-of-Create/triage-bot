mod app;
mod github;
mod log;

use crate::app::App;
use crate::github::events::issues;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::Router;
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use octocrab::models::webhook_events::{WebhookEvent, WebhookEventPayload};
use std::env;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{error, info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .init();

    dotenvy::dotenv().ok();

    let github_secret = env::var("GITHUB_WEBHOOK_SECRET")
        .expect("Missing GITHUB_WEBHOOK_SECRET Environment Variable");

    let router = Router::new()
        .route("/webhook/github", post(github_webhook))
        .layer(TraceLayer::new_for_http())
        .with_state(App::new(GithubToken(Arc::new(github_secret))));

    let ip = env::var("APP_IP").unwrap_or("0.0.0.0".to_string());
    let port = env::var("APP_PORT").unwrap_or("3000".to_string());
    let address = format!("{}:{}", ip, port);

    info!("Listening on {}", address);

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn github_webhook(
    State(app): State<App>,
    headers: HeaderMap,
    payload: GithubEvent<serde_json::Value>,
) {
    let Some(Ok(event_type)) = headers.get("X-Github-Event").map(|s| s.to_str()) else {
        error!("Missing X-GitHub-Event header");
        return;
    };

    let event = match WebhookEvent::try_from_header_and_body(event_type, &payload.0.to_string()) {
        Ok(event) => event,
        Err(e) => {
            error!(%e, "Failed to parse event, octocrab might be outdated or github's api updated");
            return;
        },
    };

    #[allow(clippy::single_match)]
    match event.specific {
        WebhookEventPayload::Issues(p) => issues::handle(p, &app.https, &app.octocrab).await,
        _ => {},
    }
}
