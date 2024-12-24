use crate::java_like_enum;
use lazy_static::lazy_static;
use octocrab::models::issues::{Issue, IssueStateReason};
use octocrab::models::IssueState;
use octocrab::Octocrab;
use regex::{Captures, Regex};
use reqwest::Client;
use strum::IntoEnumIterator;
use tracing::error;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"https?://((www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b)([-a-zA-Z0-9()@:%_+.~#?&/=]*)").unwrap();
    // Used to extract the username/org name and repo name
    static ref GITHUB_REPO_URL_REGEX: Regex = Regex::new(r"https://api\.github\.com/repos/([\w,\-_]+)/([\w,\-_]+)").unwrap();
}

// Replace regex's with groups to get rid of the replace's
java_like_enum! {
    pub enum PasteSites(hostname: &'static str, func: Box<fn(&'_ str) -> Option<String>>) {
        Haste("hst.sh", Box::new(|text| {
            let regex = Regex::new(r"https://hst\.sh/\w*").unwrap();

            regex.captures(text).map(|captures| get_url(captures).replace("hst.sh", "hst.sh/raw"))
        }));
        Mclogs("mclo.gs", Box::new(|text| {
            let regex = Regex::new(r"https://mclo\.gs/\w*").unwrap();
            
            regex.captures(text).map(|captures| get_url(captures).replace("mclo.gs", "api.mclo.gs/1/raw"))
        }));
        Pastebin("pastebin.com", Box::new(|text| {
            let regex = Regex::new(r"https://pastebin\.com/\w*").unwrap();
            
            regex.captures(text).map(|captures| get_url(captures).replace("pastebin.com", "pastebin.com/raw"))
        }));
        //Pastegg("paste.gg", Box::new(|text, https| { Some(text) }));
    }
}

pub enum AnalyzerResult<'a> {
    #[allow(dead_code)]
    Reply(&'a str),
    Close(Option<&'a str>),
    None,
}

java_like_enum! {
    pub enum Analyzers(func: Box<fn(&'_ str) -> AnalyzerResult<'static>>) {
        Test(Box::new(|text| {
            if text.contains("Hello") {
                return AnalyzerResult::Close(Some("ABC"));
            }

            AnalyzerResult::None
        }));
    }
}

pub async fn run_analyzer(issue: Issue, https: &Client, octocrab: &Octocrab) {
    let repo_url = &issue.repository_url.to_string();
    let captures = GITHUB_REPO_URL_REGEX.captures(repo_url).unwrap();

    let owner = captures.get(1).unwrap().as_str();
    let repo = captures.get(2).unwrap().as_str();

    let installation = octocrab
        .apps()
        .get_repository_installation(owner, repo)
        .await
        .unwrap();

    let installation_handler = octocrab.installation(installation.id).unwrap();

    let issue_handler = installation_handler.issues(owner, repo);

    let Some(body) = issue.body else { return };

    #[rustfmt::skip]
    let Some(url) = URL_REGEX.captures(&body)
        .and_then(|captures| captures.get(1))
        .and_then(|hostname| {
            for site in PasteSites::iter() {
                if site.hostname() == hostname.as_str() {
                    return Some(site.func());
                }
            }

            None
        })
        .and_then(|func| func(&body)) else { return };

    #[rustfmt::skip]
    let text = https.get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    for analyzer in Analyzers::iter() {
        let result = analyzer.func()(&text);

        match result {
            AnalyzerResult::Reply(message) => {
                let result = issue_handler.create_comment(issue.number, message).await;

                if let Err(err) = result {
                    error!(%err, "Error while commenting on github issue");
                }

                break;
            },
            AnalyzerResult::Close(message) => {
                if let Some(message) = message {
                    let result = issue_handler.create_comment(issue.number, message).await;

                    if let Err(err) = result {
                        error!(%err, "Error while commenting on github issue");
                    }
                }

                let result = issue_handler
                    .update(issue.number)
                    .state(IssueState::Closed)
                    .state_reason(IssueStateReason::NotPlanned)
                    .send()
                    .await;

                if let Err(err) = result {
                    error!(%err, "Error while closing github issue")
                }

                break;
            },
            AnalyzerResult::None => {},
        }
    }
}

fn get_url(captures: Captures) -> String {
    captures.get(0).unwrap().as_str().to_string()
}
