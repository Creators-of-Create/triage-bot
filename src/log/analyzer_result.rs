use octocrab::models::issues::IssueStateReason;
use octocrab::models::issues::IssueStateReason::Completed;

pub struct AnalyzerResult<'a, 'b> {
    pub reply: Option<&'a str>,
    pub close: bool,
    pub close_reason: IssueStateReason,
    pub labels: Option<&'b [String]>,
}

#[allow(dead_code)]
impl<'a, 'b> AnalyzerResult<'a, 'b> {
    pub fn new() -> Self {
        Self {
            reply: None,
            close: false,
            close_reason: Completed,
            labels: None,
        }
    }

    pub fn reply(mut self, reply: &'a str) -> Self {
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

    pub fn labels(mut self, labels: &'b [String]) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn build(self) -> Self {
        self
    }
}
