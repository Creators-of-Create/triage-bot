use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Url;

lazy_static! {
    // Used to extract the username/org name and repo name
    static ref GITHUB_REPO_URL_REGEX: Regex = Regex::new(r"https://api\.github\.com/repos/([\w,\-_]+)/([\w,\-_]+)").unwrap();
}

pub fn extract_owner_and_repo(url: Url) -> Option<(String, String)> {
    if let Some(captures) = GITHUB_REPO_URL_REGEX.captures(url.as_str()) {
        let owner = captures.get(1).map(|m| m.as_str().to_string());
        let repo = captures.get(2).map(|m| m.as_str().to_string());

        if let (Some(owner), Some(repo)) = (owner, repo) {
            return Some((owner, repo));
        }
    }

    None
}
