mod fixtures;

#[cfg(test)]
mod tests {
    use super::fixtures::TestEngine;
    use rdev::Key;
    use shortcut_matcher::ShortcutAction;
    use std::time::{Duration, Instant};

    #[test]
    fn test_single_control_delayed_when_double_exists() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
        ]);

        test_engine.press_key(Key::ControlLeft);

        // Should not fire immediately
        test_engine.expect_no_event(50);

        // Should fire after delay (300ms + 50ms buffer = 350ms)
        test_engine.expect_delayed_event(ShortcutAction::Stop, 400);
    }

    #[test]
    fn test_control_presses_at_0_and_350ms() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Start),
            ("double(ControlLeft, 300)", ShortcutAction::Stop),
        ]);

        let now = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(350));

        test_engine.expect_events(vec![ShortcutAction::Start]);

        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(100));
        test_engine.expect_events(vec![ShortcutAction::Stop]);
    }

    #[test]
    fn test_double_control_wins_over_single() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
        ]);

        let now = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(200));

        // Double tap should fire immediately
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // No delayed action should fire
        test_engine.expect_no_event(400);
    }

    #[test]
    fn test_single_fires_on_release_when_combo_exists() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("ControlLeft+ShiftLeft", ShortcutAction::Start),
        ]);

        test_engine.press_key(Key::ControlLeft);

        // Should not fire on press when combo exists
        test_engine.expect_no_event(50);

        test_engine.release_key(Key::ControlLeft);

        // Should fire on release since combo didn't complete
        test_engine.expect_immediate_event(ShortcutAction::Stop);
    }

    #[test]
    fn test_combo_wins_over_single_on_release() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("ControlLeft+ShiftLeft", ShortcutAction::Start),
        ]);

        test_engine.press_key(Key::ControlLeft);
        test_engine.press_key(Key::ShiftLeft);

        // Combo should fire immediately
        test_engine.expect_immediate_event(ShortcutAction::Start);

        test_engine.release_key(Key::ControlLeft);
        test_engine.release_key(Key::ShiftLeft);

        // Single key's release action should be cancelled
        test_engine.expect_no_event(100);
    }

    #[test]
    fn test_single_fires_on_release_with_combo_and_double() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("ControlLeft+ShiftLeft", ShortcutAction::Start),
            ("double(ControlLeft, 300)", ShortcutAction::Pause),
        ]);

        test_engine.press_key(Key::ControlLeft);

        // Should not fire immediately (FireOnRelease strategy with both combo and double)
        test_engine.expect_no_event(50);

        test_engine.release_key(Key::ControlLeft);

        // Should fire on release
        test_engine.expect_immediate_event(ShortcutAction::Stop);
    }

    #[test]
    fn test_single_delayed_by_longest_double_timeout() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("double(ControlLeft, 200)", ShortcutAction::Start),
            ("double(ControlLeft, 500)", ShortcutAction::Pause),
        ]);

        test_engine.press_key(Key::ControlLeft);

        // Should not fire immediately
        test_engine.expect_no_event(50);

        // Should not fire after shorter timeout
        test_engine.expect_no_event(250);

        // Should fire after longest timeout (500ms) + buffer (50ms)
        test_engine.expect_delayed_event(ShortcutAction::Stop, 350);
    }

    #[test]
    fn test_multiple_double_taps_different_actions() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("double(ControlLeft, 200)", ShortcutAction::Start),
            ("double(ControlLeft, 500)", ShortcutAction::Pause),
        ]);

        let now = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, now);
        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(150));

        // Faster double-tap should fire
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // No other events
        test_engine.expect_no_event(500);
    }

    #[test]
    fn test_interleaved_patterns_different_keys() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
            ("ShiftLeft", ShortcutAction::Pause),
            ("double(ShiftLeft, 300)", ShortcutAction::Cancel),
        ]);

        let now = Instant::now();

        test_engine.press_key_at_time(Key::ControlLeft, now);

        test_engine.press_key_at_time(Key::ControlLeft, now + Duration::from_millis(100));

        test_engine.expect_immediate_event(ShortcutAction::Start);

        let now = Instant::now();

        test_engine.press_key_at_time(Key::ShiftLeft, now);

        test_engine.expect_delayed_event(ShortcutAction::Pause, 400);
    }

    #[test]
    fn test_start_to_cancel_sequence() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("double(ControlLeft, 300)", ShortcutAction::Start),
            ("ShiftLeft", ShortcutAction::Pause),
            ("double(ShiftLeft, 300)", ShortcutAction::Cancel),
        ]);

        // Start: Idle -> Active
        let t0 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t0);
        test_engine.press_key_at_time(Key::ControlLeft, t0 + Duration::from_millis(100));
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Pause: Active -> Paused
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_delayed_event(ShortcutAction::Pause, 400);

        // Cancel: Paused -> Idle
        std::thread::sleep(Duration::from_millis(100));
        let t1 = Instant::now();
        test_engine.press_key_at_time(Key::ShiftLeft, t1);
        test_engine.press_key_at_time(Key::ShiftLeft, t1 + Duration::from_millis(150));
        test_engine.expect_immediate_event(ShortcutAction::Cancel);

        // Verify we're back in Idle - can Start again
        std::thread::sleep(Duration::from_millis(100));
        let t2 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t2);
        test_engine.press_key_at_time(Key::ControlLeft, t2 + Duration::from_millis(100));
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }

    #[test]
    fn test_comprehensive_state_transitions() {
        let mut test_engine = TestEngine::new_with_patterns(vec![
            ("ControlLeft", ShortcutAction::Stop),
            ("double(ControlLeft, 300)", ShortcutAction::Start),
            ("ShiftLeft", ShortcutAction::Pause),
            ("double(ShiftLeft, 300)", ShortcutAction::Cancel),
        ]);

        // Phase 1: Start the engine with double ControlLeft
        let t0 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t0);
        test_engine.press_key_at_time(Key::ControlLeft, t0 + Duration::from_millis(150));

        // Should fire Start immediately (double tap detected)
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Phase 2: Pause with single ShiftLeft (state is now Active)
        // Use current time instead of future time
        std::thread::sleep(Duration::from_millis(100)); // Small delay to ensure clean state

        test_engine.press_key(Key::ShiftLeft);

        // Single ShiftLeft should fire Pause after delay (300ms timeout + 50ms buffer = 350ms)
        test_engine.expect_delayed_event(ShortcutAction::Pause, 400);

        // Phase 3: Cancel from Paused state with double ShiftLeft
        // State is now Paused, only Cancel or Stop actions are allowed
        std::thread::sleep(Duration::from_millis(100));
        let t3 = Instant::now();
        test_engine.press_key_at_time(Key::ShiftLeft, t3);
        test_engine.press_key_at_time(Key::ShiftLeft, t3 + Duration::from_millis(150));

        // Should fire Cancel immediately (double-tap in Paused state)
        test_engine.expect_immediate_event(ShortcutAction::Cancel);

        // Phase 4: Start again with double ControlLeft (state is now Idle after Cancel)
        std::thread::sleep(Duration::from_millis(100));
        let t4 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t4);
        test_engine.press_key_at_time(Key::ControlLeft, t4 + Duration::from_millis(150));

        // Should fire Start immediately
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Phase 5: Stop with single ControlLeft (state is now Active)
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ControlLeft);

        // Single ControlLeft should fire Stop after delay
        test_engine.expect_delayed_event(ShortcutAction::Stop, 400);

        // Phase 6: Start again with double ControlLeft (state is now Idle after Stop)
        std::thread::sleep(Duration::from_millis(100));
        let t6 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t6);
        test_engine.press_key_at_time(Key::ControlLeft, t6 + Duration::from_millis(200));

        // Should fire Start immediately
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Phase 7: Pause then Stop from Paused (state is Active)
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_delayed_event(ShortcutAction::Pause, 400);

        // Now in Paused state, Stop with single ControlLeft
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ControlLeft);
        test_engine.expect_delayed_event(ShortcutAction::Stop, 400);

        // Phase 8: Start, Pause, then Cancel sequence (state is now Idle after Stop)
        std::thread::sleep(Duration::from_millis(100));

        // Start with double ControlLeft
        let t8a = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t8a);
        test_engine.press_key_at_time(Key::ControlLeft, t8a + Duration::from_millis(100));
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Pause with single ShiftLeft
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ShiftLeft);
        test_engine.expect_delayed_event(ShortcutAction::Pause, 400);

        // Cancel with double ShiftLeft from Paused state
        std::thread::sleep(Duration::from_millis(100));
        let t8b = Instant::now();
        test_engine.press_key_at_time(Key::ShiftLeft, t8b);
        test_engine.press_key_at_time(Key::ShiftLeft, t8b + Duration::from_millis(150));
        test_engine.expect_immediate_event(ShortcutAction::Cancel);

        // Phase 9: Test timing precision - single key with exact delay measurement
        std::thread::sleep(Duration::from_millis(100));
        let t9 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t9);
        test_engine.press_key_at_time(Key::ControlLeft, t9 + Duration::from_millis(90));

        // Double tap within 300ms window - should fire Start immediately
        test_engine.expect_immediate_event(ShortcutAction::Start);

        // Phase 10: Test timeout boundary - just outside double-tap window
        std::thread::sleep(Duration::from_millis(100));
        test_engine.press_key(Key::ControlLeft);

        // Wait for single Stop to fire (should be exactly 350ms)
        test_engine.expect_delayed_event(ShortcutAction::Stop, 400);

        // Now press again after the timeout - should be treated as new single press
        std::thread::sleep(Duration::from_millis(100));
        let t10 = Instant::now();
        test_engine.press_key_at_time(Key::ControlLeft, t10);
        test_engine.press_key_at_time(Key::ControlLeft, t10 + Duration::from_millis(295));

        // Just inside the 300ms window - should fire Start
        test_engine.expect_immediate_event(ShortcutAction::Start);
    }
}
