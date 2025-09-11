use super::Pattern;
use crate::types::ShortcutAction;
use crate::types::{ConflictStrategy, MatchResult, PatternType};
use rdev::Key;
use std::any::Any;
use std::time::Instant;

#[derive(Debug)]
pub struct SingleKeyPattern {
    key: Key,
    action: ShortcutAction,
    conflict_strategy: ConflictStrategy,
    press_time: Option<Instant>,
}

impl SingleKeyPattern {
    pub fn new(key: Key, action: ShortcutAction) -> Self {
        Self {
            key,
            action,
            conflict_strategy: ConflictStrategy::Immediate,
            press_time: None,
        }
    }

    pub fn set_conflict_strategy(&mut self, strategy: ConflictStrategy) {
        self.conflict_strategy = strategy;
    }
}

impl Pattern for SingleKeyPattern {
    fn process_key_press(&mut self, key: Key, now: Instant) -> MatchResult {
        if key == self.key {
            self.press_time = Some(now);

            match &self.conflict_strategy {
                ConflictStrategy::Immediate => MatchResult::Complete {
                    action: self.action,
                },
                ConflictStrategy::FireOnRelease => MatchResult::Partial {
                    next_expected: vec![],
                },
                ConflictStrategy::DelayedFire { delay_ms } => MatchResult::Delayed {
                    action: self.action,
                    wait_ms: *delay_ms,
                },
            }
        } else {
            // When a different key is pressed:
            // - If we're waiting for release, stay active (return Partial)
            // - Otherwise, this pattern doesn't match
            if self.press_time.is_some()
                && matches!(self.conflict_strategy, ConflictStrategy::FireOnRelease)
            {
                MatchResult::Partial {
                    next_expected: vec![],
                }
            } else {
                MatchResult::NoMatch
            }
        }
    }

    fn process_key_release(&mut self, key: Key, now: Instant) -> MatchResult {
        if key != self.key {
            return MatchResult::NoMatch;
        }

        let result = match &self.conflict_strategy {
            ConflictStrategy::FireOnRelease => {
                if let Some(press_time) = self.press_time {
                    let hold_duration = now.duration_since(press_time).as_millis();

                    if hold_duration < 100 {
                        MatchResult::Complete {
                            action: self.action,
                        }
                    } else {
                        MatchResult::NoMatch
                    }
                } else {
                    MatchResult::NoMatch
                }
            }
            _ => MatchResult::NoMatch,
        };

        self.reset();
        result
    }

    fn reset(&mut self) {
        self.press_time = None;
    }

    fn is_expired(&self, _now: Instant) -> bool {
        false
    }

    fn get_trigger_key(&self) -> Key {
        self.key
    }

    fn get_type(&self) -> PatternType {
        PatternType::Single
    }

    fn get_timeout(&self) -> Option<u64> {
        None
    }

    fn get_activation_keys(&self) -> Vec<Key> {
        vec![self.key]
    }

    fn has_partial_match(&self) -> bool {
        self.press_time.is_some()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
