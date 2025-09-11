mod fixtures;

use fixtures::TestEngine;
use rdev::Key;
use shortcut_matcher::{ShortcutAction, ShortcutEngineState};
use std::time::{Duration, Instant};

mod tests {
    use super::*;

    #[test]
    fn test_single_control_actually_triggers() {
        let mut test_engine =
            TestEngine::new_with_patterns(vec![("ControlLeft", ShortcutAction::Start)]);

        // Test that pressing ControlLeft triggers Start action
        test_engine.press_key(Key::ControlLeft);
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }

    #[test]
    fn test_single_control_wrong_key_no_trigger() {
        let mut test_engine =
            TestEngine::new_with_patterns(vec![("ControlLeft", ShortcutAction::Stop)]);

        test_engine.press_key(Key::ControlRight);
        test_engine.expect_no_event(50);
    }

    #[test]
    fn test_double_control_correct_sequence() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "double(ControlLeft, 300)",
            ShortcutAction::Start,
        )]);

        let now = Instant::now();

        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.expect_no_event(50);

        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(200));
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }

    #[test]
    fn test_double_control_timeout_exceeded() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "double(ControlLeft, 300)",
            ShortcutAction::Start,
        )]);

        let now = Instant::now();

        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.expect_no_event(50);

        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(400));
        test_engine.expect_no_event(50);
    }

    #[test]
    fn test_double_control_wrong_key_no_trigger() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "double(ControlLeft, 300)",
            ShortcutAction::Start,
        )]);

        let now = Instant::now();

        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.expect_no_event(50);

        test_engine.press_key_at_time(Key::ControlRight, now + Duration::from_millis(200));
        test_engine.expect_no_event(50);
    }

    #[test]
    fn test_combo_cmd_shift_y_correct_sequence() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "CommandLeft+ShiftLeft+KeyY",
            ShortcutAction::Start,
        )]);

        test_engine.press_key(Key::MetaLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::KeyY);
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }

    #[test]
    fn test_combo_cmd_shift_y_wrong_order() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "CommandLeft+ShiftLeft+KeyY",
            ShortcutAction::Start,
        )]);

        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::MetaLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::KeyY);
        test_engine.expect_no_event(10);
    }

    #[test]
    fn test_combo_cmd_shift_y_incomplete() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "CommandLeft+ShiftLeft+KeyY",
            ShortcutAction::Start,
        )]);

        test_engine.press_key(Key::MetaLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::KeyX);
        test_engine.expect_no_event(10);
    }

    #[test]
    fn test_multiple_patterns_parsing_and_matching() {
        let mut combo_test_engine = TestEngine::new_with_patterns(vec![
            ("CommandLeft+ShiftLeft+KeyY", ShortcutAction::Start),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
        ]);

        combo_test_engine.press_key(Key::MetaLeft);
        combo_test_engine.press_key(Key::ShiftLeft);
        combo_test_engine.press_key(Key::KeyY);
        combo_test_engine.expect_immediate_event(ShortcutAction::Start);

        let mut double_test_engine = TestEngine::new_with_patterns(vec![
            ("CommandLeft+ShiftLeft+KeyY", ShortcutAction::Start),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
        ]);

        let now = Instant::now();
        double_test_engine.press_key_at_time(Key::ControlLeft, now);
        double_test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(200));
        double_test_engine.expect_immediate_event(ShortcutAction::Start);
    }

    #[test]
    fn test_engine_state_transitions() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Start),
            ("ControlRight", ShortcutAction::Stop),
            ("ShiftLeft", ShortcutAction::Pause),
        ]);

        assert_eq!(test_engine.get_state(), ShortcutEngineState::Idle);

        test_engine.press_key(Key::ControlLeft);
        test_engine.expect_immediate_event(ShortcutAction::Start);
        assert_eq!(test_engine.get_state(), ShortcutEngineState::Active);

        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_immediate_event(ShortcutAction::Pause);
        assert_eq!(test_engine.get_state(), ShortcutEngineState::Paused);

        test_engine.press_key(Key::ControlRight);
        test_engine.expect_immediate_event(ShortcutAction::Stop);
        assert_eq!(test_engine.get_state(), ShortcutEngineState::Idle);
    }

    #[test]
    fn test_combo_activation_only_on_first_key() {
        let mut test_engine = TestEngine::new_with_patterns(vec![(
            "CommandLeft+ShiftLeft+KeyY",
            ShortcutAction::Start,
        )]);

        // Test that only the first key (CommandLeft) triggers initial activation
        // Pressing other keys in the combo should not activate if not in correct sequence
        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::KeyY);
        test_engine.expect_no_event(10);

        // But pressing the first key should start the pattern matching
        test_engine.press_key(Key::MetaLeft);
        test_engine.expect_no_event(10);

        // And now continue with the correct sequence
        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_no_event(10);

        test_engine.press_key(Key::KeyY);
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }
}
