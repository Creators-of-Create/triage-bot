use octocrab::models::issues::IssueStateReason;
use octocrab::models::issues::IssueStateReason::Completed;

pub struct AnalyzerResult {
    pub reply: Option<String>,
    pub close: bool,
    pub close_reason: IssueStateReason,
    pub labels: Option<Box<[String]>>,
}

#[allow(dead_code)]
impl AnalyzerResult {
    pub fn new() -> Self {
        Self {
            reply: None,
            close: false,
            close_reason: Completed,
            labels: None,
        }
    }

    pub fn reply(mut self, reply: String) -> Self {
        self.reply = Some(reply);
        self
    }

    pub fn close(mut self) -> Self {
        self.close = true;
        self
    }

    pub fn close_reason(mut self, close_reason: IssueStateReason) -> Self {
        self.close_reason = close_reason;
        self
    }

    pub fn labels(mut self, labels: Box<[String]>) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn build(self) -> Self {
        self
    }
}
