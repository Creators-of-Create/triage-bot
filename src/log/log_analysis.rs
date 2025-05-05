use crate::log::analyzer_result::AnalyzerResult;
use lazy_static::lazy_static;
use octocrab::models::issues::Issue;
use octocrab::models::issues::IssueStateReason::NotPlanned;
use octocrab::models::IssueState;
use octocrab::Octocrab;
use fancy_regex::Regex;
use reqwest::Client;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tracing::{debug, error};

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"https?://((www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b)([-a-zA-Z0-9()@:%_+.~#?&/=]*)").unwrap();
    // Used to extract the username/org name and repo name
    static ref GITHUB_REPO_URL_REGEX: Regex = Regex::new(r"https://api\.github\.com/repos/([\w,\-_]+)/([\w,\-_]+)").unwrap();

    static ref NO_AUTH_OCTOCRAB: Octocrab = Octocrab::builder().build().unwrap();
    
    // ---
    
    static ref MISSING_CREATE_CLASS_REGEX: Regex = Regex::new(r"java\.lang\.NoClassDefFoundError: com/simibubi/create/.*\n.*(?:TRANSFORMER/([a-z][a-z0-9_]{1,63})@|at .*~\[(?!javafmllanguage)([a-zA-Z0-9_]*)-.*jar)").unwrap();
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

                let id = regex.captures(text).ok()
                    .flatten()
                    .and_then(|captures| captures.get(1));

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
                .captures(s).ok()
                .flatten()
                .and_then(|cap| cap.get(0))
                .map(|cap| cap.as_str())
                .map(|url| url.replace(from, to))
        }
    }
}

#[derive(EnumIter)]
pub enum Analyzers {
    MissingCreateClass,
}

impl Analyzers {
    fn get_result(&self, text: &str) -> Option<AnalyzerResult> {
        match self {
            Analyzers::MissingCreateClass => {
                MISSING_CREATE_CLASS_REGEX
                    .captures(text).ok()
                    .flatten()
                    .and_then(|captures| captures.get(1))
                    .map(|mod_id| {
                        let mod_id = mod_id.as_str();
                        let r = format!("The mod `{}` is trying to use Create classes that no longer exist, the developer for `{}` will have to update their mod to fix this.", mod_id, mod_id);
                        
                        AnalyzerResult::new()
                            .close()
                            .close_reason(NotPlanned)
                            .labels(vec!["wrong repo: other mod"])
                            .reply(r)
                            .build()
                    })
            }
        }
    }
}

pub async fn run_analyzer(issue: Issue, https: &Client, octocrab: &Octocrab) -> anyhow::Result<()> {
    debug!("Running analyzer for issue: {:?}", issue.id);
    
    let repo_url = &issue.repository_url.to_string();
    let captures = GITHUB_REPO_URL_REGEX.captures(repo_url)?.unwrap();

    let owner = captures.get(1).unwrap().as_str();
    let repo = captures.get(2).unwrap().as_str();

    let installation = octocrab
        .apps()
        .get_repository_installation(owner, repo)
        .await?;

    let installation_handler = octocrab.installation(installation.id)?;

    let issue_handler = installation_handler.issues(owner, repo);

    let Some(body) = issue.body else {
        return Ok(());
    };

    let Some(site) = URL_REGEX
        .captures(&body).ok()
        .flatten()
        .and_then(|captures| captures.get(1))
        .and_then(|hostname| PasteSites::iter().find(|site| site.hostname() == hostname.as_str()))
    else {
        return Ok(());
    };

    let Some(url) = site.get_raw_url(&body).await else {
        return Ok(());
    };

    debug!("Found url: {}", url);

    #[rustfmt::skip]
    let text = https.get(url)
        .send()
        .await?
        .text()
        .await?;

    for analyzer in Analyzers::iter() {
        let result = analyzer.get_result(&text);
        
        if let Some(result) = result {
            if let Some(labels) = result.labels {
                let mut final_labels: Vec<String> = Vec::new();

                for label in &labels {
                    final_labels.push(label.to_string());
                }

                for label in &issue.labels {
                    final_labels.push(label.clone().name);
                }

                issue_handler
                    .update(issue.number)
                    .labels(&final_labels)
                    .send()
                    .await?;
            }

            if let Some(message) = result.reply {
                issue_handler.create_comment(issue.number, message).await?;
            }

            if result.close {
                issue_handler
                    .update(issue.number)
                    .state(IssueState::Closed)
                    .state_reason(result.close_reason)
                    .send()
                    .await?;
            }
            
            debug!("Ran analyzer that matched this issue!")
        }
    }

    Ok(())
}
