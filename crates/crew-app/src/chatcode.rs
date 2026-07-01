//! Fenced-code detection for message bodies: splits a reply into prose and
//! ```code``` segments so the card renderer can give code a bordered,
//! dimmed-background treatment with a language tag.

/// One segment of a message body: prose lines, or a fenced code block.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Seg {
    /// Prose: the logical (newline-split) lines outside any fence.
    Text(Vec<String>),
    /// A fenced block: the language tag from the opening fence ("" if none)
    /// and the verbatim lines inside it.
    Code { lang: String, lines: Vec<String> },
}

/// Split `text` into prose/code segments on ``` fences. The fence lines
/// themselves are consumed; an unclosed fence runs to the end of the message.
pub(crate) fn parse_fences(text: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut prose: Vec<String> = Vec::new();
    let mut code: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("```") {
            match code.take() {
                Some((lang, lines)) => segs.push(Seg::Code { lang, lines }),
                None => {
                    if !prose.is_empty() {
                        segs.push(Seg::Text(std::mem::take(&mut prose)));
                    }
                    code = Some((rest.trim().to_string(), Vec::new()));
                }
            }
        } else {
            match &mut code {
                Some((_, lines)) => lines.push(line.to_string()),
                None => prose.push(line.to_string()),
            }
        }
    }
    if let Some((lang, lines)) = code {
        segs.push(Seg::Code { lang, lines }); // unclosed fence
    }
    if !prose.is_empty() {
        segs.push(Seg::Text(prose));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_prose_segment() {
        let segs = parse_fences("hello\nworld");
        assert_eq!(segs, vec![Seg::Text(vec!["hello".into(), "world".into()])]);
    }

    #[test]
    fn fence_with_language_tag_is_extracted() {
        let segs = parse_fences("look:\n```rust\nfn main() {}\n```\ndone");
        assert_eq!(
            segs,
            vec![
                Seg::Text(vec!["look:".into()]),
                Seg::Code {
                    lang: "rust".into(),
                    lines: vec!["fn main() {}".into()],
                },
                Seg::Text(vec!["done".into()]),
            ]
        );
    }

    #[test]
    fn unclosed_fence_runs_to_message_end() {
        let segs = parse_fences("```py\nx = 1");
        assert_eq!(
            segs,
            vec![Seg::Code {
                lang: "py".into(),
                lines: vec!["x = 1".into()],
            }]
        );
    }

    #[test]
    fn indented_fence_and_empty_lang_are_tolerated() {
        let segs = parse_fences("  ```\ncode\n  ```");
        assert_eq!(
            segs,
            vec![Seg::Code {
                lang: String::new(),
                lines: vec!["code".into()],
            }]
        );
    }
}
