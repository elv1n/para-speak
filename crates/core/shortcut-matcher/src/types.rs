use rdev::Key;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutAction {
    Start,
    Stop,
    Cancel,
    Pause,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutEngineState {
    Idle,
    Active,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    Press(Key),
    Release(Key),
}

impl KeyEvent {
    pub fn key(&self) -> Key {
        match self {
            KeyEvent::Press(k) | KeyEvent::Release(k) => *k,
        }
    }

    pub fn state(&self) -> KeyState {
        match self {
            KeyEvent::Press(_) => KeyState::Pressed,
            KeyEvent::Release(_) => KeyState::Released,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchResult {
    NoMatch,
    Partial {
        next_expected: Vec<Key>,
    },
    Complete {
        action: ShortcutAction,
    },
    Delayed {
        action: ShortcutAction,
        wait_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternType {
    Single,
    Combo,
    DoubleTap,
}

#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    Immediate,
    FireOnRelease,
    DelayedFire { delay_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DelayedActionId(pub u64);

#[derive(Debug, Clone)]
pub struct DelayedAction {
    pub id: DelayedActionId,
    pub action: ShortcutAction,
    pub trigger_at: Instant,
}
