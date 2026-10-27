//! The wake phrase: "hey claude" at the start of what was heard. The rest of the sentence is the request.

/// Words of `text` with their byte spans, lower-cased and stripped of punctuation.
fn words(text: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        let part = c.is_alphanumeric() || c == '\'';
        match (part, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((text[s..i].to_lowercase(), s, i));
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// If `text` begins with one of the wake phrases (within the first few words), what follows it; otherwise None.
/// "Hey Claude, what time is it?" with "hey claude" gives "what time is it?"; "hey claude" alone gives "".
pub fn strip_wake<'t>(text: &'t str, phrases: &[&str]) -> Option<&'t str> {
    let ws = words(text);
    for phrase in phrases {
        let want: Vec<String> = phrase.split_whitespace().map(|w| w.to_lowercase()).collect();
        if want.is_empty() || ws.len() < want.len() {
            continue;
        }
        // allow a filler word or two before the phrase ("um hey claude")
        for skip in 0..=2usize.min(ws.len() - want.len()) {
            if ws[skip..skip + want.len()].iter().map(|w| &w.0).eq(want.iter()) {
                let end = ws[skip + want.len() - 1].2;
                return Some(text[end..].trim_start_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAKE: [&str; 3] = ["hey claude", "okay claude", "claude"];

    #[test]
    fn the_request_follows_the_wake_phrase() {
        assert_eq!(strip_wake("Hey Claude, what time is it?", &WAKE), Some("what time is it?"));
        assert_eq!(strip_wake("hey claude", &WAKE), Some(""));
        assert_eq!(strip_wake("Okay Claude... set a timer", &WAKE), Some("set a timer"));
        assert_eq!(strip_wake("um hey claude turn it off", &WAKE), Some("turn it off"));
        assert_eq!(strip_wake("Claude, hi", &WAKE), Some("hi"));
    }

    #[test]
    fn talk_that_is_not_for_it_is_ignored() {
        assert_eq!(strip_wake("what time is it", &WAKE), None);
        assert_eq!(strip_wake("I told the claude story yesterday", &WAKE), None);
        assert_eq!(strip_wake("", &WAKE), None);
    }
}
