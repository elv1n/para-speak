use aho_corasick::{AhoCorasickBuilder, MatchKind};
use config::Config;
use std::collections::HashMap;
use std::sync::Arc;

pub fn handle_transcribed_text(text: String, config: Arc<Config>) -> String {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() || trimmed.len() < 3 {
        log::debug!("[ML] Empty transcription");
        return String::new();
    }

    apply_replacements(&trimmed, config.transcription_replacements())
}

#[inline]
fn is_word_boundary(c: Option<char>) -> bool {
    c.is_none_or(|ch| !ch.is_alphanumeric() && ch != '_')
}

#[inline]
fn calculate_deletion_end(end: usize, next_char: Option<char>, text_bytes: &[u8]) -> usize {
    match next_char {
        Some(',') | Some(';') => {
            let mut new_end = end + 1;
            if new_end < text_bytes.len() && text_bytes[new_end] == b' ' {
                new_end += 1;
            }
            new_end
        }
        Some(' ') => end + 1,
        _ => end,
    }
}
fn apply_replacements(text: &str, replacements: &HashMap<String, Option<String>>) -> String {
    if text.is_empty() || replacements.is_empty() {
        return text.to_owned();
    }

    let patterns: Vec<&str> = replacements.keys().map(|s| s.as_str()).collect();

    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&patterns)
        .unwrap();
    let text_bytes = text.as_bytes();

    let mut result = Vec::with_capacity(text.len());
    let mut last_end = 0;

    for m in ac.find_iter(text) {
        let start = m.start();
        let end = m.end();

        let prev_char = if start > 0 {
            text[..start].chars().last()
        } else {
            None
        };
        let next_char = text[end..].chars().next();

        let is_word_start = is_word_boundary(prev_char);
        let is_word_end = is_word_boundary(next_char);

        if !is_word_start || !is_word_end {
            continue;
        }

        result.extend_from_slice(&text_bytes[last_end..start]);

        let pattern = &text[start..end];
        if let Some(replacement) = replacements.get(pattern) {
            match replacement {
                Some(rep) => {
                    result.extend_from_slice(rep.as_bytes());
                    last_end = end;
                }
                None => {
                    last_end = calculate_deletion_end(end, next_char, text_bytes);
                }
            }
        }
    }

    // Add remaining text
    result.extend_from_slice(&text_bytes[last_end..]);

    String::from_utf8(result).unwrap_or_else(|_| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_replacements() {
        let mut replacements = HashMap::new();
        replacements.insert("ui".to_string(), Some("UI".to_string()));
        replacements.insert("uh".to_string(), None);
        replacements.insert("um".to_string(), None);

        assert_eq!(
            apply_replacements("ui test uh something um", &replacements),
            "UI test something "
        );
    }

    #[test]
    fn test_word_boundaries() {
        let mut replacements = HashMap::new();
        replacements.insert("is".to_string(), Some("was".to_string()));

        assert_eq!(
            apply_replacements("This is a test", &replacements),
            "This was a test"
        );

        assert_eq!(
            apply_replacements("This isn't affected", &replacements),
            "This isn't affected"
        );

        assert_eq!(
            apply_replacements("His idea is good", &replacements),
            "His idea was good"
        );
    }

    #[test]
    fn test_case_sensitive() {
        let mut replacements = HashMap::new();
        replacements.insert("ui".to_string(), Some("UI".to_string()));
        replacements.insert("UI".to_string(), Some("ui".to_string()));

        assert_eq!(
            apply_replacements("ui should become UI and UI becomes ui", &replacements),
            "UI should become ui and ui becomes UI"
        );

        let mut case_specific = HashMap::new();
        case_specific.insert("So".to_string(), None);
        assert_eq!(
            apply_replacements("So what about so and SO?", &case_specific),
            "what about so and SO?"
        );
    }

    #[test]
    fn test_punctuation_preservation() {
        let mut replacements = HashMap::new();
        replacements.insert("uh".to_string(), None);
        replacements.insert("um".to_string(), None);

        assert_eq!(
            apply_replacements("Well, uh, I think, um, yes!", &replacements),
            "Well, I think, yes!"
        );

        assert_eq!(
            apply_replacements("uh... what about um?", &replacements),
            "... what about ?"
        );
    }

    #[test]
    fn test_multiple_occurrences() {
        let mut replacements = HashMap::new();
        replacements.insert("foo".to_string(), Some("bar".to_string()));

        assert_eq!(
            apply_replacements("foo is foo and foo", &replacements),
            "bar is bar and bar"
        );
    }

    #[test]
    fn test_empty_and_whitespace() {
        let mut replacements = HashMap::new();
        replacements.insert("test".to_string(), None);

        assert_eq!(apply_replacements("", &replacements), "");
        assert_eq!(apply_replacements("   ", &replacements), "   ");
    }

    #[test]
    fn test_no_replacements() {
        assert_eq!(
            apply_replacements("unchanged text", &HashMap::new()),
            "unchanged text"
        );
    }

    #[test]
    fn test_special_regex_chars() {
        let mut replacements = HashMap::new();
        replacements.insert("a.b".to_string(), Some("replaced".to_string()));
        replacements.insert("c*d".to_string(), None);
        replacements.insert("e+f".to_string(), Some("plus".to_string()));
        replacements.insert("g?h".to_string(), None);
        replacements.insert("i[j]k".to_string(), Some("brackets".to_string()));
        replacements.insert("l(m)n".to_string(), None);
        replacements.insert("o^p".to_string(), Some("caret".to_string()));
        replacements.insert("q$r".to_string(), None);
        replacements.insert("s|t".to_string(), Some("pipe".to_string()));
        replacements.insert("u\\v".to_string(), None);

        assert_eq!(
            apply_replacements("test a.b and c*d here", &replacements),
            "test replaced and here"
        );

        assert_eq!(
            apply_replacements("e+f g?h i[j]k", &replacements),
            "plus brackets"
        );

        assert_eq!(apply_replacements("l(m)n o^p q$r", &replacements), "caret ");

        assert_eq!(
            apply_replacements("s|t u\\v end", &replacements),
            "pipe end"
        );
    }

    #[test]
    fn test_beginning_and_end_of_text() {
        let mut replacements = HashMap::new();
        replacements.insert("start".to_string(), Some("begin".to_string()));
        replacements.insert("end".to_string(), Some("finish".to_string()));

        assert_eq!(
            apply_replacements("start middle end", &replacements),
            "begin middle finish"
        );

        assert_eq!(apply_replacements("start", &replacements), "begin");

        assert_eq!(apply_replacements("end", &replacements), "finish");
    }

    #[test]
    fn test_adjacent_replacements() {
        let mut replacements = HashMap::new();
        replacements.insert("uh".to_string(), None);
        replacements.insert("um".to_string(), None);

        assert_eq!(
            apply_replacements("uh um testing", &replacements),
            "testing"
        );

        assert_eq!(apply_replacements("uhum", &replacements), "uhum");
    }

    #[test]
    fn test_mixed_deletions_and_replacements() {
        let mut replacements = HashMap::new();
        replacements.insert("like".to_string(), None);
        replacements.insert("you know".to_string(), None);
        replacements.insert("basically".to_string(), None);
        replacements.insert("i".to_string(), Some("I".to_string()));
        replacements.insert("dont".to_string(), Some("don't".to_string()));

        // assert_eq!(
        //     apply_replacements("so like i dont know basically what you know", &replacements),
        //     "so  I don't know  what you know"
        // );
        log::info!("replacements: {:?}", replacements);
        assert_eq!(
            apply_replacements("i think like basically you know i dont", &replacements),
            "I think I don't"
        );
    }

    #[test]
    fn test_handle_transcribed_text_empty() {
        let config = Arc::new(Config::new_for_test(vec![], vec![], vec![], vec![]));

        assert_eq!(handle_transcribed_text("".to_string(), config.clone()), "");
        assert_eq!(
            handle_transcribed_text("  ".to_string(), config.clone()),
            ""
        );
        assert_eq!(handle_transcribed_text("ab".to_string(), config), "");
    }

    #[test]
    fn test_handle_transcribed_text_with_replacements() {
        let config = Arc::new(Config::new_for_test(vec![], vec![], vec![], vec![]));

        assert_eq!(
            handle_transcribed_text("This is a test".to_string(), config.clone()),
            "This is a test"
        );

        assert_eq!(
            handle_transcribed_text("   spaces around   ".to_string(), config),
            "spaces around"
        );
    }

    #[test]
    fn test_overlapping_patterns() {
        let mut replacements = HashMap::new();
        replacements.insert("the".to_string(), Some("a".to_string()));
        replacements.insert("them".to_string(), Some("those".to_string()));

        assert_eq!(
            apply_replacements("the them theory", &replacements),
            "a those theory"
        );
    }

    #[test]
    fn test_numbers_and_alphanumeric() {
        let mut replacements = HashMap::new();
        replacements.insert("v2".to_string(), Some("version2".to_string()));
        replacements.insert("3d".to_string(), Some("three-dimensional".to_string()));

        assert_eq!(
            apply_replacements("v2 model in 3d space", &replacements),
            "version2 model in three-dimensional space"
        );
    }
    #[test]
    fn test_its() {
        let mut replacements = HashMap::new();
        replacements.insert("it it it".to_string(), Some("it".to_string()));

        assert_eq!(
            apply_replacements("hm it it it to it it", &replacements),
            "hm it to it it"
        );
    }
    #[test]
    fn test_its_doubles() {
        let mut replacements = HashMap::new();
        replacements.insert("it it it".to_string(), Some("it".to_string()));
        replacements.insert("it it".to_string(), Some("it".to_string()));

        assert_eq!(
            apply_replacements("hm it it it to it it", &replacements),
            "hm it to it"
        );
    }
}
