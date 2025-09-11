use crossbeam_channel::{unbounded, Receiver, Sender};
use rdev::Key;
use shortcut_matcher::*;
use shortcut_matcher::{ShortcutAction, ShortcutEngineState};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct TestHandler {
    sender: Sender<ShortcutAction>,
}

impl TestHandler {
    fn new(sender: Sender<ShortcutAction>) -> Self {
        Self { sender }
    }
}

impl ShortcutHandler for TestHandler {
    fn handle_action(&self, action: ShortcutAction) {
        let _ = self.sender.send(action);
    }

    fn handle_error(&self, error: String) {
        log::debug!("Test handler error: {}", error);
    }
}

pub struct TestEngine {
    engine: MatcherEngine,
    handler: Arc<TestHandler>,
    event_receiver: Receiver<ShortcutAction>,
}

impl TestEngine {
    pub fn new_with_patterns(patterns: Vec<(&str, ShortcutAction)>) -> Self {
        let (sender, receiver) = unbounded();
        let handler = Arc::new(TestHandler::new(sender));
        
        let mut engine = MatcherEngine::new_with_patterns(Some(50), patterns);
        
        engine.reset();
        engine.activate();
        
        // Drain any stray events from the channel
        while receiver.try_recv().is_ok() {}
        
        Self {
            engine,
            handler: handler.clone(),
            event_receiver: receiver,
        }
    }


    pub fn press_key(&mut self, key: Key) {
        if let Some(action) = self.engine.process_event(KeyEvent::Press(key)) {
            self.handler.handle_action(action);
        }
    }

    pub fn press_key_at_time(&mut self, key: Key, time: Instant) {
        if let Some(action) = self.engine.process_event_with_time(KeyEvent::Press(key), time) {
            self.handler.handle_action(action);
        }
    }

    #[allow(dead_code)]
    pub fn release_key(&mut self, key: Key) {
        if let Some(action) = self.engine.process_event(KeyEvent::Release(key)) {
            self.handler.handle_action(action);
        }
    }

    #[allow(dead_code)]
    pub fn release_key_at_time(&mut self, key: Key, time: Instant) {
        if let Some(action) = self.engine.process_event_with_time(KeyEvent::Release(key), time) {
            self.handler.handle_action(action);
        }
    }

    #[allow(dead_code)]
    pub fn wait_for_event(&mut self, timeout_ms: u64) -> Option<ShortcutAction> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if let Some(action) = self.engine.poll_delayed_action() {
                self.handler.handle_action(action);
            }
            if let Ok(action) = self.event_receiver.try_recv() {
                return Some(action);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    pub fn expect_immediate_event(&self, expected_action: ShortcutAction) {
        let action = self
            .event_receiver
            .recv_timeout(Duration::from_millis(100))
            .unwrap_or_else(|_| panic!("Expected immediate {:?} action", expected_action));

        if action != expected_action {
            panic!("Expected {:?}, got {:?}", expected_action, action);
        }
    }

    #[allow(dead_code)]
    pub fn expect_delayed_event(&mut self, expected_action: ShortcutAction, timeout_ms: u64) {
        let action = self.wait_for_event(timeout_ms).unwrap_or_else(|| {
            panic!(
                "Expected to receive delayed {:?} within {}ms",
                expected_action, timeout_ms
            )
        });

        if action != expected_action {
            panic!("Expected delayed {:?}, got {:?}", expected_action, action);
        }
    }

    #[allow(dead_code)]
    pub fn expect_delayed_event_with_timing(
        &mut self,
        expected_action: ShortcutAction,
        timeout_ms: u64,
    ) -> Instant {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if let Some(action) = self.engine.poll_delayed_action() {
                self.handler.handle_action(action);
            }
            if let Ok(action) = self.event_receiver.try_recv() {
                if action == expected_action {
                    return Instant::now();
                } else {
                    panic!("Expected delayed {:?}, got {:?}", expected_action, action);
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!(
            "Expected to receive delayed {:?} within {}ms",
            expected_action, timeout_ms
        );
    }

    pub fn expect_no_event(&mut self, wait_ms: u64) {
        thread::sleep(Duration::from_millis(wait_ms));
        if let Some(action) = self.engine.poll_delayed_action() {
            self.handler.handle_action(action);
        }
        if let Ok(action) = self.event_receiver.try_recv() {
            panic!("Expected no event, but got {:?}", action);
        }
    }

    #[allow(dead_code)]
    pub fn expect_events(&mut self, expected_actions: Vec<ShortcutAction>) {
        for expected_action in expected_actions {
            if let Some(action) = self.engine.poll_delayed_action() {
                self.handler.handle_action(action);
            }
            let action = self
                .event_receiver
                .recv_timeout(Duration::from_millis(1000))
                .unwrap_or_else(|_| panic!("Expected action {:?}", expected_action));

            if action != expected_action {
                panic!("Expected {:?}, got {:?}", expected_action, action);
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_state(&self) -> ShortcutEngineState {
        self.engine.current_state
    }
}
