mod app;
mod github;
mod log;
mod utils;

use crate::app::App;
use crate::github::events::issue_comment;
use crate::github::events::issues;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use axum_github_webhook_extract::{GithubEvent, GithubToken};
use octocrab::models::webhook_events::{WebhookEvent, WebhookEventPayload};
use std::env;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let github_secret = env::var("GITHUB_WEBHOOK_SECRET")
        .expect("Missing GITHUB_WEBHOOK_SECRET Environment Variable");

    let router = Router::new()
        .route("/status", get(|| async { StatusCode::OK }))
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

    debug!("Got event: {:?}", event);

    #[allow(clippy::single_match)]
    let result = match event.specific {
        WebhookEventPayload::Issues(p) => issues::handle(p, &app.https, &app.octocrab).await,
        WebhookEventPayload::IssueComment(p) => {
            issue_comment::handle(p, &app.https, &app.octocrab).await
        },
        _ => Ok(()),
    };

    if let Err(e) = result {
        error!(%e, "Error occurred while handling event");
    }
}
