use lazy_static::lazy_static;
use octocrab::models::issues::{Issue, IssueStateReason};
use octocrab::models::IssueState;
use octocrab::Octocrab;
use regex::Regex;
use reqwest::Client;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tracing::error;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"https?://((www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b)([-a-zA-Z0-9()@:%_+.~#?&/=]*)").unwrap();
    // Used to extract the username/org name and repo name
    static ref GITHUB_REPO_URL_REGEX: Regex = Regex::new(r"https://api\.github\.com/repos/([\w,\-_]+)/([\w,\-_]+)").unwrap();

    static ref NO_AUTH_OCTOCRAB: Octocrab = Octocrab::builder().build().unwrap();
}

#[derive(EnumIter)]
enum PasteSites {
    Gist,
    Haste,
    Mclogs,
    Pastebin,
}

impl PasteSites {
    fn hostname(&self) -> &'static str {
        match self {
            PasteSites::Gist => "gist.github.com",
            PasteSites::Haste => "hst.sh",
            PasteSites::Mclogs => "mclo.gs",
            PasteSites::Pastebin => "pastebin.com",
        }
    }

    async fn get_raw_url(&self, text: &str) -> Option<String> {
        return match self {
            PasteSites::Gist => {
                let regex =
                    Regex::new(r"https://gist\.github\.com/[A-Za-z\d-]{0,38}/(\w*)").unwrap();

                let id = regex.captures(text).and_then(|captures| captures.get(1));

                if let Some(id) = id {
                    match NO_AUTH_OCTOCRAB.gists().get(id.as_str()).await {
                        Ok(gist) => {
                            return gist
                                .files
                                .iter()
                                .next()
                                .map(|file| file.1.raw_url.to_string());
                        },
                        Err(e) => error!(%e, "Error occurred while fetching raw url for gist"),
                    }
                }

                None
            },
            PasteSites::Haste => r(text, r"https://hst\.sh/\w*", "hst.sh", "hst.sh/raw"),
            PasteSites::Mclogs => r(
                text,
                r"https://mclo\.gs/\w*",
                "mclo.gs",
                "api.mclo.gs/1/raw",
            ),
            PasteSites::Pastebin => r(
                text,
                r"https://pastebin\.com/\w*",
                "pastebin.com",
                "pastebin.com/raw",
            ),
        };

        fn r(s: &str, regex: &str, from: &str, to: &str) -> Option<String> {
            let regex = Regex::new(regex).unwrap();

            regex
                .captures(s)
                .and_then(|cap| cap.get(0))
                .map(|cap| cap.as_str())
                .map(|url| url.replace(from, to))
        }
    }
}

#[allow(dead_code)]
pub enum AnalyzerResult<'a> {
    Reply(&'a str),
    CloseAsNotPlanned(Option<&'a str>),
    Close(Option<&'a str>),
    None,
}

#[derive(EnumIter)]
pub enum Analyzers {
    Test,
}

impl Analyzers {
    fn get_result(&self, text: &str) -> AnalyzerResult {
        match self {
            Analyzers::Test => {
                if text.contains("Hello") {
                    return AnalyzerResult::CloseAsNotPlanned(Some("ABC"));
                }

                AnalyzerResult::None
            },
        }
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

    let Some(site) = URL_REGEX
        .captures(&body)
        .and_then(|captures| captures.get(1))
        .and_then(|hostname| PasteSites::iter().find(|site| site.hostname() == hostname.as_str()))
    else {
        return;
    };

    let Some(url) = site.get_raw_url(&body).await else {
        return;
    };

    #[rustfmt::skip]
    let text = https.get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    for analyzer in Analyzers::iter() {
        let result = analyzer.get_result(&text);

        match result {
            AnalyzerResult::Reply(message) => {
                let result = issue_handler.create_comment(issue.number, message).await;

                if let Err(err) = result {
                    error!(%err, "Error while commenting on github issue");
                }

                break;
            },
            AnalyzerResult::Close(message) | AnalyzerResult::CloseAsNotPlanned(message) => {
                if let Some(message) = message {
                    let result = issue_handler.create_comment(issue.number, message).await;

                    if let Err(err) = result {
                        error!(%err, "Error while commenting on github issue");
                    }
                }

                let reason = if matches!(result, AnalyzerResult::CloseAsNotPlanned(_)) {
                    IssueStateReason::NotPlanned
                } else {
                    IssueStateReason::Completed
                };

                let result = issue_handler
                    .update(issue.number)
                    .state(IssueState::Closed)
                    .state_reason(reason)
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
