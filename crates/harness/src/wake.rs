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

/// A word's consonant skeleton ("Claude", "Eclord", "cloud", "clawed" all give "cld"): how a recogniser that hears a name badly still gets its shape. Only used
/// for words whose skeleton keeps at least three letters, so short names are matched exactly ("Jean" does not wake on "John").
fn skeleton(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars().filter(|c| !"aeiouhwyr'".contains(*c)) {
        if out.chars().last() != Some(c) {
            out.push(c);
        }
    }
    out
}

/// Whether a heard word is the wanted one: the same, or (for a name long enough to tell apart) the same consonant skeleton. A skeleton match is only
/// trusted when something else vouches for it: the greeting before it matched exactly ("hey" + a name-shaped word), or it is a longish word that begins
/// the utterance ("Eclord, ..."); "cold" has Claude's skeleton too, and "The cold wind" must not wake anything.
fn same_word(heard: &str, want: &str, after_greeting: bool, starts_utterance: bool) -> bool {
    if heard == want {
        return true;
    }
    let (sh, sw) = (skeleton(heard), skeleton(want));
    sw.len() >= 3 && sh == sw && (after_greeting || (starts_utterance && heard.chars().count() >= 5))
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
            if ws[skip..skip + want.len()].iter().zip(&want).enumerate().all(|(k, (h, w))| same_word(&h.0, w, k > 0, skip == 0)) {
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
    fn a_name_the_recogniser_heard_badly_still_wakes_it() {
        // what whisper's base model really made of "Hey Claude, what is the weather like?" in one run
        assert_eq!(strip_wake("Eclord, what is the weather like?", &WAKE), Some("what is the weather like?"));
        assert_eq!(strip_wake("Hey Cloud, what time is it", &WAKE), Some("what time is it"));
        assert_eq!(strip_wake("Okay clawed turn it off", &WAKE), Some("turn it off"));
        assert_eq!(strip_wake("Hey, clod. Lights please", &WAKE), Some("Lights please"));
    }

    #[test]
    fn short_names_and_other_words_do_not_wake_it() {
        // "Jean" has a two-letter skeleton, so it is matched exactly: John is not Jean
        assert_eq!(strip_wake("Hey John, what time is it", &["hey jean"]), None);
        assert_eq!(strip_wake("Hey Jean, what time is it", &["hey jean"]), Some("what time is it"));
        assert_eq!(strip_wake("The cold wind outside", &WAKE), None);
        assert_eq!(strip_wake("Hey kid, come here", &WAKE), None);
    }

    #[test]
    fn talk_that_is_not_for_it_is_ignored() {
        assert_eq!(strip_wake("what time is it", &WAKE), None);
        assert_eq!(strip_wake("I told the claude story yesterday", &WAKE), None);
        assert_eq!(strip_wake("", &WAKE), None);
    }
}
