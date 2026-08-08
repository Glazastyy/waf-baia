use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityRule {
    pub name: String,
    pub priority: u32,
    pub path_prefix: Option<String>,
    pub action: RuleAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    Allow,
    Block,
    Challenge,
    Captcha,
    RateLimit,
    Redirect,
    AddHeader,
    RemoveHeader,
    Log,
    ApplyRule,
}
