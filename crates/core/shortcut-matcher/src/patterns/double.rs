use super::Pattern;
use crate::types::ShortcutAction;
use crate::types::{MatchResult, PatternType};
use rdev::Key;
use std::any::Any;
use std::time::Instant;

#[derive(Debug)]
pub struct DoubleTapPattern {
    target_key: Key,
    timeout_ms: u64,
    action: ShortcutAction,
    first_press: Option<Instant>,
}

impl DoubleTapPattern {
    pub fn new(target_key: Key, timeout_ms: u64, action: ShortcutAction) -> Self {
        Self {
            target_key,
            timeout_ms,
            action,
            first_press: None,
        }
    }
}

impl Pattern for DoubleTapPattern {
    fn process_key_press(&mut self, key: Key, now: Instant) -> MatchResult {
        if key != self.target_key {
            return MatchResult::NoMatch;
        }

        match self.first_press {
            None => {
                self.first_press = Some(now);
                MatchResult::Partial {
                    next_expected: vec![self.target_key],
                }
            }
            Some(first) => {
                let elapsed_ms = now.duration_since(first).as_millis() as u64;

                if elapsed_ms <= self.timeout_ms {
                    self.reset();
                    MatchResult::Complete {
                        action: self.action,
                    }
                } else {
                    self.first_press = Some(now);
                    MatchResult::Partial {
                        next_expected: vec![self.target_key],
                    }
                }
            }
        }
    }

    fn process_key_release(&mut self, _key: Key, _now: Instant) -> MatchResult {
        MatchResult::NoMatch
    }

    fn reset(&mut self) {
        self.first_press = None;
    }

    fn is_expired(&self, now: Instant) -> bool {
        self.first_press
            .map(|t| now.duration_since(t).as_millis() as u64 > self.timeout_ms)
            .unwrap_or(false)
    }

    fn get_trigger_key(&self) -> Key {
        self.target_key
    }

    fn get_type(&self) -> PatternType {
        PatternType::DoubleTap
    }

    fn get_timeout(&self) -> Option<u64> {
        Some(self.timeout_ms)
    }

    fn get_activation_keys(&self) -> Vec<Key> {
        vec![self.target_key]
    }

    fn has_partial_match(&self) -> bool {
        self.first_press.is_some()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
