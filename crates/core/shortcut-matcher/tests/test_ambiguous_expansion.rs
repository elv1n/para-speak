use shortcut_matcher::{parse_pattern, parse_multiple_patterns, ShortcutAction};

mod tests {
    use super::*;

    #[test]
    fn test_single_ambiguous_modifiers() {
        let patterns = parse_pattern("Control", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("Ctrl", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("Shift", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("Alt", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("Meta", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn test_explicit_left_right_no_expansion() {
        let patterns = parse_pattern("ControlLeft", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
        
        let patterns = parse_pattern("ControlRight", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
        
        let patterns = parse_pattern("ShiftLeft", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_combo_ambiguous_expansion() {
        let patterns = parse_pattern("Control+Y", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("Control+Alt+Y", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 4);
        
        let patterns = parse_pattern("Control+Shift+Alt+Y", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 8);
    }

    #[test]
    fn test_combo_mixed_explicit_and_ambiguous() {
        let patterns = parse_pattern("ControlLeft+Alt+Y", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("ControlLeft+AltRight+Y", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_double_tap_ambiguous_expansion() {
        let patterns = parse_pattern("double(Control)", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("double(Shift, 500)", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 2);
        
        let patterns = parse_pattern("double(ControlLeft)", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_multiple_patterns_expansion() {
        let patterns = parse_multiple_patterns("Control+Y;Shift+X;Alt", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 6);
    }

    #[test]
    fn test_regular_keys_no_expansion() {
        let patterns = parse_pattern("A", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
        
        let patterns = parse_pattern("Space", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
        
        let patterns = parse_pattern("F1", ShortcutAction::Start).unwrap();
        assert_eq!(patterns.len(), 1);
    }
}