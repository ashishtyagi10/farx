#[derive(Debug, Clone)]
pub struct QuickAction {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuickActionResult {
    None,
    Close,
    Execute(String),
}

pub struct QuickActionsState {
    pub active: bool,
    pub actions: Vec<QuickAction>,
    pub cursor: usize,
    pub file_name: String,
}
