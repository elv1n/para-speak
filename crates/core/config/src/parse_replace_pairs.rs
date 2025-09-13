use std::collections::HashMap;

pub fn parse_replace_pairs(replace_str: &Option<String>) -> HashMap<String, Option<String>> {
    let mut replacements = default_replacements();

    if let Some(s) = replace_str {
        if !s.is_empty() {
            for pair in s.split(';').filter(|pair| !pair.trim().is_empty()) {
                let trimmed = pair.trim();
                if let Some(colon_idx) = trimmed.find(':') {
                    let (from, to) = trimmed.split_at(colon_idx);
                    let to_str = to[1..].trim();
                    replacements.insert(
                        from.trim().to_string(),
                        if to_str.is_empty() {
                            None
                        } else {
                            Some(to_str.to_string())
                        },
                    );
                } else {
                    replacements.insert(trimmed.to_string(), None);
                }
            }
        }
    }

    replacements
}

fn default_replacements() -> HashMap<String, Option<String>> {
    let mut defaults = HashMap::new();

    defaults.insert("Uh".to_string(), None);
    defaults.insert("uh".to_string(), None);
    defaults.insert("ah".to_string(), None);
    defaults.insert("oh".to_string(), None);
    defaults.insert("um".to_string(), None);
    defaults.insert("Um".to_string(), None);
    defaults.insert("Oh".to_string(), None);
    defaults.insert("so.".to_string(), None);

    defaults
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_replace_pairs() {
        let result = parse_replace_pairs(&Some("ui:UI;uh:;um".to_string()));
        assert_eq!(result.get("ui"), Some(&Some("UI".to_string())));
        assert_eq!(result.get("uh"), Some(&None));
        assert_eq!(result.get("um"), Some(&None));

        let result = parse_replace_pairs(&Some("foo:bar".to_string()));
        assert_eq!(result.get("foo"), Some(&Some("bar".to_string())));

        let result = parse_replace_pairs(&Some("remove_me:".to_string()));
        assert_eq!(result.get("remove_me"), Some(&None));

        let result = parse_replace_pairs(&Some("remove_me".to_string()));
        assert_eq!(result.get("remove_me"), Some(&None));

        let result = parse_replace_pairs(&Some("a:b;c:d;e:f".to_string()));
        assert_eq!(result.get("a"), Some(&Some("b".to_string())));
        assert_eq!(result.get("c"), Some(&Some("d".to_string())));
        assert_eq!(result.get("e"), Some(&Some("f".to_string())));

        let result = parse_replace_pairs(&Some("  spaces : trimmed  ; next:pair ".to_string()));
        assert_eq!(result.get("spaces"), Some(&Some("trimmed".to_string())));
        assert_eq!(result.get("next"), Some(&Some("pair".to_string())));
    }

    #[test]
    fn test_empty_string_uses_defaults() {
        let result = parse_replace_pairs(&Some("".to_string()));

        assert_eq!(result.get("Uh"), Some(&None));
        assert_eq!(result.get("uh"), Some(&None));
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_custom_overrides_defaults() {
        let result = parse_replace_pairs(&Some("uh:actually;Um:Indeed".to_string()));

        assert_eq!(result.get("uh"), Some(&Some("actually".to_string())));
        assert_eq!(result.get("Um"), Some(&Some("Indeed".to_string())));

        assert_eq!(result.get("Uh"), Some(&None));
        assert_eq!(result.get("oh"), Some(&None));
    }
}
