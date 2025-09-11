mod mocks;

use mocks::MockController;
use shortcut_matcher::{MatcherEngine, ShortcutAction, ShortcutHandler, KeyEvent, ShortcutEngineState};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::thread;
use rdev::Key;

#[cfg(test)]
mod tests {
    use super::*;

    fn simulate_key_press(engine: &mut MatcherEngine, key: Key) -> Option<ShortcutAction> {
        engine.process_event(KeyEvent::Press(key))
    }

    #[test]
    fn test_shortcut_triggers_controller() {
        // Setup mock controller
        let controller = Arc::new(MockController::new());
        
        // Create shortcut engine with test patterns
        let mut engine = MatcherEngine::new_with_patterns(
            Some(50),
            vec![
                ("F1", ShortcutAction::Start),
                ("F2", ShortcutAction::Stop),
                ("F3", ShortcutAction::Pause),
                ("F4", ShortcutAction::Cancel),
            ],
        );

        // Initially, no methods should have been called
        assert_eq!(controller.handle_start_count.load(Ordering::SeqCst), 0);
        assert_eq!(controller.handle_stop_count.load(Ordering::SeqCst), 0);
        
        // Simulate F1 key press (Start)
        if let Some(action) = simulate_key_press(&mut engine, Key::F1) {
            controller.handle_action(action);
        }
        
        // Verify start was called
        assert_eq!(controller.handle_start_count.load(Ordering::SeqCst), 1);
        assert!(controller.audio.lock().unwrap().is_recording());
        
        // Update engine state to Active (since recording started)
        engine.set_state(ShortcutEngineState::Active);
        
        // Simulate F3 key press (Pause)
        if let Some(action) = simulate_key_press(&mut engine, Key::F3) {
            controller.handle_action(action);
        }
        
        // Verify pause was called
        assert_eq!(controller.handle_pause_count.load(Ordering::SeqCst), 1);
        assert!(controller.audio.lock().unwrap().is_paused());
        
        // Update engine state to Paused
        engine.set_state(ShortcutEngineState::Paused);
        
        // Simulate F3 key press again (Resume)
        if let Some(action) = simulate_key_press(&mut engine, Key::F3) {
            controller.handle_action(action);
        }
        
        // Verify pause was called again (which handles resume)
        assert_eq!(controller.handle_pause_count.load(Ordering::SeqCst), 2);
        assert!(!controller.audio.lock().unwrap().is_paused());
        
        // Update engine state back to Active
        engine.set_state(ShortcutEngineState::Active);
        
        // Simulate F2 key press (Stop)
        if let Some(action) = simulate_key_press(&mut engine, Key::F2) {
            controller.handle_action(action);
        }
        
        thread::sleep(Duration::from_millis(100));
        
        // Verify stop was called
        assert_eq!(controller.handle_stop_count.load(Ordering::SeqCst), 1);
        assert!(!controller.audio.lock().unwrap().is_recording());
        
        // Verify the complete flow
        assert_eq!(controller.registry.start_count.load(Ordering::SeqCst), 1);
        assert_eq!(controller.registry.pause_count.load(Ordering::SeqCst), 1);
        assert_eq!(controller.registry.resume_count.load(Ordering::SeqCst), 1);
        assert_eq!(controller.registry.stop_count.load(Ordering::SeqCst), 1);
        assert_eq!(controller.registry.processing_start_count.load(Ordering::SeqCst), 1);
        assert_eq!(controller.registry.processing_complete_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cancel_shortcut() {
        // Setup mock controller
        let controller = Arc::new(MockController::new());
        
        // Create shortcut engine
        let mut engine = MatcherEngine::new_with_patterns(
            Some(50),
            vec![
                ("F1", ShortcutAction::Start),
                ("F4", ShortcutAction::Cancel),
            ],
        );

        // Start recording
        if let Some(action) = simulate_key_press(&mut engine, Key::F1) {
            controller.handle_action(action);
        }
        
        assert_eq!(controller.handle_start_count.load(Ordering::SeqCst), 1);
        assert!(controller.audio.lock().unwrap().is_recording());
        
        // Update engine state to Active
        engine.set_state(ShortcutEngineState::Active);
        
        // Cancel recording
        if let Some(action) = simulate_key_press(&mut engine, Key::F4) {
            controller.handle_action(action);
        }
        
        // Verify cancel was called
        assert_eq!(controller.handle_cancel_count.load(Ordering::SeqCst), 1);
        assert!(!controller.audio.lock().unwrap().is_recording());
        assert_eq!(controller.registry.cancel_count.load(Ordering::SeqCst), 1);
        
        // Verify no transcription happened (since we cancelled)
        thread::sleep(Duration::from_millis(100));
        assert_eq!(controller.registry.processing_start_count.load(Ordering::SeqCst), 0);
        assert_eq!(controller.registry.processing_complete_count.load(Ordering::SeqCst), 0);
    }
}