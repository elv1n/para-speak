pub mod combo;
pub mod double;
pub mod single;

use crate::types::{MatchResult, PatternType};
use rdev::Key;
use std::any::Any;
use std::fmt::Debug;
use std::time::Instant;

pub trait Pattern: Send + Sync + Debug {
    fn process_key_press(&mut self, key: Key, now: Instant) -> MatchResult;
    fn process_key_release(&mut self, key: Key, now: Instant) -> MatchResult;
    fn reset(&mut self);
    fn is_expired(&self, now: Instant) -> bool;

    fn get_trigger_key(&self) -> Key;
    fn get_type(&self) -> PatternType;
    fn get_timeout(&self) -> Option<u64>;

    fn could_match_key(&self, key: Key) -> bool {
        self.get_trigger_key() == key
    }

    fn get_activation_keys(&self) -> Vec<Key>;

    fn has_partial_match(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
