use crate::log::log_analysis::run_analyzer;
use octocrab::models::webhook_events::payload::{
    IssuesWebhookEventAction, IssuesWebhookEventPayload,
};
use octocrab::Octocrab;
use reqwest::Client;

pub async fn handle(payload: Box<IssuesWebhookEventPayload>, https: &Client, octocrab: &Octocrab) {
    match payload.action {
        IssuesWebhookEventAction::Opened | IssuesWebhookEventAction::Edited => {
            run_analyzer(payload.issue, https, octocrab).await
        },
        _ => {},
    }
}
