use super::*;
use crate::{CliAdapter, Normalize};

fn reg(names: &[&str]) -> Registry {
    Registry::new(
        names
            .iter()
            .map(|n| {
                Box::new(CliAdapter {
                    name: (*n).into(),
                    program: "true".into(),
                    args: vec![],
                    normalize: Normalize::Raw,
                }) as Box<dyn crate::Adapter>
            })
            .collect(),
    )
}

#[test]
fn roster_lists_or_explains() {
    assert!(roster(&reg(&["claude", "codex"])).contains("claude, codex"));
    assert!(roster(&reg(&[])).contains("ANTHROPIC_API_KEY"));
}
