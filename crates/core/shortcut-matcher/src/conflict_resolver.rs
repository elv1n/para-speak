use crate::patterns::{single::SingleKeyPattern, Pattern};
use crate::types::{ConflictStrategy, PatternType};
use rdev::Key;
use std::collections::HashMap;

pub struct ConflictResolver {
    buffer_ms: u64,
}

impl ConflictResolver {
    pub fn new(buffer_ms: u64) -> Self {
        Self { buffer_ms }
    }

    pub fn analyze_and_mark_conflicts(&self, patterns: &mut [Box<dyn Pattern>]) {
        let patterns_by_key = self.group_patterns_by_trigger_key(patterns);

        for (_key, pattern_indices) in patterns_by_key {
            self.apply_type_based_strategies(pattern_indices, patterns);
        }
    }

    fn group_patterns_by_trigger_key(
        &self,
        patterns: &[Box<dyn Pattern>],
    ) -> HashMap<Key, Vec<usize>> {
        let mut groups: HashMap<Key, Vec<usize>> = HashMap::new();

        for (idx, pattern) in patterns.iter().enumerate() {
            let key = pattern.get_trigger_key();
            groups.entry(key).or_default().push(idx);
        }

        groups.retain(|_, indices| indices.len() > 1);
        groups
    }

    fn apply_type_based_strategies(
        &self,
        pattern_indices: Vec<usize>,
        patterns: &mut [Box<dyn Pattern>],
    ) {
        let mut has_combo = false;
        let mut has_double = false;
        let mut max_double_timeout = 0u64;
        let mut single_indices = Vec::new();

        for &idx in &pattern_indices {
            match patterns[idx].get_type() {
                PatternType::Single => {
                    single_indices.push(idx);
                }
                PatternType::Combo => {
                    has_combo = true;
                }
                PatternType::DoubleTap => {
                    has_double = true;
                    if let Some(timeout) = patterns[idx].get_timeout() {
                        max_double_timeout = max_double_timeout.max(timeout);
                    }
                }
            }
        }

        for idx in single_indices {
            let strategy = if has_combo {
                ConflictStrategy::FireOnRelease
            } else if has_double {
                ConflictStrategy::DelayedFire {
                    delay_ms: max_double_timeout + self.buffer_ms,
                }
            } else {
                ConflictStrategy::Immediate
            };

            if let Some(single) = patterns[idx]
                .as_any_mut()
                .downcast_mut::<SingleKeyPattern>()
            {
                single.set_conflict_strategy(strategy);
            }
        }
    }
}
