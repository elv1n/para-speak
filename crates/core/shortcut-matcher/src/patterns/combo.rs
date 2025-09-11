use super::Pattern;
use crate::types::ShortcutAction;
use crate::types::{MatchResult, PatternType};
use rdev::Key;
use std::any::Any;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug)]
pub struct ComboPattern {
    sequence: Vec<Key>,
    current_index: usize,
    held_keys: HashSet<Key>,
    action: ShortcutAction,
}

impl ComboPattern {
    pub fn new(sequence: Vec<Key>, action: ShortcutAction) -> Self {
        assert!(!sequence.is_empty(), "Combo pattern cannot be empty");

        Self {
            sequence,
            current_index: 0,
            held_keys: HashSet::new(),
            action,
        }
    }
}

impl Pattern for ComboPattern {
    fn process_key_press(&mut self, key: Key, _now: Instant) -> MatchResult {
        if self.current_index < self.sequence.len() && key == self.sequence[self.current_index] {
            self.held_keys.insert(key);
            self.current_index += 1;

            if self.current_index == self.sequence.len() {
                MatchResult::Complete {
                    action: self.action,
                }
            } else {
                MatchResult::Partial {
                    next_expected: vec![self.sequence[self.current_index]],
                }
            }
        } else if self.held_keys.contains(&key) {
            if self.current_index < self.sequence.len() {
                MatchResult::Partial {
                    next_expected: vec![self.sequence[self.current_index]],
                }
            } else {
                MatchResult::NoMatch
            }
        } else {
            self.reset();
            MatchResult::NoMatch
        }
    }

    fn process_key_release(&mut self, key: Key, _now: Instant) -> MatchResult {
        if self.held_keys.remove(&key) {
            if let Some(released_index) = self.sequence.iter().position(|&k| k == key) {
                if released_index < self.current_index {
                    self.reset();
                }
            }
        }
        MatchResult::NoMatch
    }

    fn reset(&mut self) {
        self.current_index = 0;
        self.held_keys.clear();
    }

    fn is_expired(&self, _now: Instant) -> bool {
        false
    }

    fn get_trigger_key(&self) -> Key {
        self.sequence[0]
    }

    fn get_type(&self) -> PatternType {
        PatternType::Combo
    }

    fn get_timeout(&self) -> Option<u64> {
        None
    }

    fn could_match_key(&self, key: Key) -> bool {
        self.sequence.contains(&key)
    }

    fn get_activation_keys(&self) -> Vec<Key> {
        vec![self.sequence[0]]
    }

    fn has_partial_match(&self) -> bool {
        self.current_index > 0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
