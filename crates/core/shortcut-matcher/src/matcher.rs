use crate::patterns::Pattern;
use crate::types::ShortcutAction;
use crate::types::{DelayedAction, DelayedActionId, KeyEvent, MatchResult};
use rdev::Key;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static DELAYED_ACTION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ShortcutMatcher {
    patterns: Vec<Box<dyn Pattern>>,
    patterns_by_key: HashMap<Key, Vec<usize>>,
    active_patterns: HashSet<usize>,
    pending_delayed_ids: Vec<DelayedActionId>,

    activation_keys: HashSet<Key>,
}

impl ShortcutMatcher {
    pub fn new(patterns: Vec<Box<dyn Pattern>>) -> Self {
        let patterns_by_key = Self::build_index(&patterns);

        let mut activation_keys = HashSet::new();

        for pattern in &patterns {
            for key in pattern.get_activation_keys() {
                activation_keys.insert(key);
            }
        }
        
        log::debug!("Matcher created with activation_keys: {:?}", activation_keys);

        Self {
            patterns,
            patterns_by_key,
            active_patterns: HashSet::new(),
            pending_delayed_ids: Vec::new(),
            activation_keys,
        }
    }

    fn build_index(patterns: &[Box<dyn Pattern>]) -> HashMap<Key, Vec<usize>> {
        let mut index: HashMap<Key, Vec<usize>> = HashMap::new();

        for (idx, pattern) in patterns.iter().enumerate() {
            let key = pattern.get_trigger_key();
            index.entry(key).or_default().push(idx);
        }

        // Sort patterns by priority: double-tap patterns with shorter timeouts first
        for pattern_indices in index.values_mut() {
            pattern_indices.sort_by(|&a, &b| {
                let pattern_a = &patterns[a];
                let pattern_b = &patterns[b];
                
                match (pattern_a.get_timeout(), pattern_b.get_timeout()) {
                    (Some(timeout_a), Some(timeout_b)) => timeout_a.cmp(&timeout_b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }

        index
    }

    pub fn process_event(
        &mut self,
        event: KeyEvent,
        now: Instant,
    ) -> (
        Option<ShortcutAction>,
        Vec<DelayedAction>,
        Vec<DelayedActionId>,
    ) {
        self.cleanup_expired_patterns(now);

        match event {
            KeyEvent::Press(key) => self.process_press(key, now),
            KeyEvent::Release(key) => self.process_release(key, now),
        }
    }

    fn process_press(
        &mut self,
        key: Key,
        now: Instant,
    ) -> (
        Option<ShortcutAction>,
        Vec<DelayedAction>,
        Vec<DelayedActionId>,
    ) {
        let mut delayed_actions = Vec::new();
        let mut cancelled_ids = Vec::new();

        // Check active patterns first, in priority order (shortest timeout first)
        let mut active_patterns: Vec<usize> = self.active_patterns.iter().copied().collect();
        active_patterns.sort_by(|&a, &b| {
            let pattern_a = &self.patterns[a];
            let pattern_b = &self.patterns[b];
            
            match (pattern_a.get_timeout(), pattern_b.get_timeout()) {
                (Some(timeout_a), Some(timeout_b)) => timeout_a.cmp(&timeout_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        for idx in active_patterns {
            match self.patterns[idx].process_key_press(key, now) {
                MatchResult::Complete { action } => {
                    // Cancel all pending delayed actions
                    cancelled_ids.append(&mut self.pending_delayed_ids);
                    self.reset_all_patterns();
                    // Clear any delayed actions collected so far
                    delayed_actions.clear();
                    return (Some(action), delayed_actions, cancelled_ids);
                }
                MatchResult::Delayed { action, wait_ms } => {
                    let id = DelayedActionId(DELAYED_ACTION_COUNTER.fetch_add(1, Ordering::SeqCst));
                    self.pending_delayed_ids.push(id);
                    delayed_actions.push(DelayedAction {
                        id,
                        action,
                        trigger_at: now + Duration::from_millis(wait_ms),
                    });
                }
                MatchResult::Partial { .. } => {
                    // Pattern still active
                }
                MatchResult::NoMatch => {
                    self.active_patterns.remove(&idx);
                }
            }
        }

        // Check new patterns that could start with this key
        if let Some(pattern_indices) = self.patterns_by_key.get(&key) {
            for &idx in pattern_indices {
                if self.active_patterns.contains(&idx) {
                    continue;
                }
                match self.patterns[idx].process_key_press(key, now) {
                    MatchResult::Partial { .. } => {
                        self.active_patterns.insert(idx);
                    }
                    MatchResult::Complete { action } => {
                        // Cancel all pending delayed actions
                        cancelled_ids.append(&mut self.pending_delayed_ids);
                        self.reset_all_patterns();
                        // Clear any delayed actions collected so far
                        delayed_actions.clear();
                        return (Some(action), delayed_actions, cancelled_ids);
                    }
                    MatchResult::Delayed { action, wait_ms } => {
                        let id =
                            DelayedActionId(DELAYED_ACTION_COUNTER.fetch_add(1, Ordering::SeqCst));
                        self.pending_delayed_ids.push(id);
                        delayed_actions.push(DelayedAction {
                            id,
                            action,
                            trigger_at: now + Duration::from_millis(wait_ms),
                        });
                    }
                    MatchResult::NoMatch => {}
                }
            }
        }

        (None, delayed_actions, cancelled_ids)
    }

    fn process_release(
        &mut self,
        key: Key,
        now: Instant,
    ) -> (
        Option<ShortcutAction>,
        Vec<DelayedAction>,
        Vec<DelayedActionId>,
    ) {
        let active_patterns: Vec<usize> = self.active_patterns.iter().copied().collect();
        let mut cancelled_ids = Vec::new();

        for idx in active_patterns {
            match self.patterns[idx].process_key_release(key, now) {
                MatchResult::Complete { action } => {
                    // Cancel all pending delayed actions
                    cancelled_ids.append(&mut self.pending_delayed_ids);
                    self.reset_all_patterns();
                    return (Some(action), Vec::new(), cancelled_ids);
                }
                MatchResult::NoMatch => {}
                _ => {}
            }
        }

        (None, Vec::new(), Vec::new())
    }

    fn cleanup_expired_patterns(&mut self, now: Instant) {
        let expired: Vec<usize> = self
            .active_patterns
            .iter()
            .copied()
            .filter(|&idx| self.patterns[idx].is_expired(now))
            .collect();

        for idx in expired {
            self.patterns[idx].reset();
            self.active_patterns.remove(&idx);
        }
    }

    fn reset_all_patterns(&mut self) {
        for idx in &self.active_patterns {
            self.patterns[*idx].reset();
        }
        self.active_patterns.clear();
    }

    pub fn reset(&mut self) {
        self.reset_all_patterns();
    }

    pub fn get_expected_keys(&self) -> HashSet<Key> {
        let mut keys = HashSet::new();

        if self.active_patterns.is_empty() {
            keys.extend(self.patterns_by_key.keys().copied());
        }

        keys
    }

    pub fn can_activate_fast(&self, key: Key) -> bool {
        self.activation_keys.contains(&key)
    }

    pub fn has_partial_matches(&self) -> bool {
        self.patterns.iter().any(|p| p.has_partial_match())
    }
}
