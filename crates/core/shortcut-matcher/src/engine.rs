use crate::conflict_resolver::ConflictResolver;
use crate::error::MatcherError;
use crate::matcher::ShortcutMatcher;
use crate::parser::parse_multiple_patterns;
use crate::patterns::Pattern;
use crate::types::{DelayedAction, KeyEvent};
use crate::types::{ShortcutAction, ShortcutEngineState};
use config::Config;
use rdev::Key;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

pub struct MatcherEngine {
    matcher: ShortcutMatcher,
    pub current_state: ShortcutEngineState,
    buffer_ms: u64,
    pending_delayed: Vec<DelayedAction>,
    is_actively_listening: AtomicBool,
    last_activity: AtomicU64,
    pattern_configs: Option<Vec<(String, ShortcutAction)>>,
}

impl Default for MatcherEngine {
    fn default() -> Self {
        let config: std::sync::Arc<Config> = config::Config::global();

        let buffer_ms = config.shortcut_resolution_delay_ms.unwrap_or(50);
        let patterns = Self::build_patterns_for_state(ShortcutEngineState::Idle, buffer_ms);

        Self {
            matcher: ShortcutMatcher::new(patterns),
            current_state: ShortcutEngineState::Idle,
            buffer_ms,
            pending_delayed: Vec::new(),
            is_actively_listening: AtomicBool::new(false),
            last_activity: AtomicU64::new(0),
            pattern_configs: None,
        }
    }
}

impl MatcherEngine {
    pub fn new_with_patterns(
        buffer_ms: Option<u64>,
        pattern_configs: Vec<(&str, ShortcutAction)>,
    ) -> Self {
        let buffer_ms = buffer_ms.unwrap_or(50);
        let stored_configs: Vec<(String, ShortcutAction)> = pattern_configs
            .iter()
            .map(|(pattern, action)| (pattern.to_string(), *action))
            .collect();

        let mut conflict_resolver = ConflictResolver::new(buffer_ms);
        let patterns = Self::parse_patterns_internal(pattern_configs, &mut conflict_resolver);

        Self {
            matcher: ShortcutMatcher::new(patterns),
            current_state: ShortcutEngineState::Idle,
            buffer_ms,
            pending_delayed: Vec::new(),
            is_actively_listening: AtomicBool::new(false),
            last_activity: AtomicU64::new(0),
            pattern_configs: Some(stored_configs),
        }
    }

    fn get_available_actions(state: ShortcutEngineState) -> &'static [ShortcutAction] {
        match state {
            ShortcutEngineState::Idle => &[ShortcutAction::Start],
            ShortcutEngineState::Active => &[
                ShortcutAction::Stop,
                ShortcutAction::Cancel,
                ShortcutAction::Pause,
            ],
            ShortcutEngineState::Paused => &[
                ShortcutAction::Pause,
                ShortcutAction::Cancel,
                ShortcutAction::Stop,
            ],
        }
    }

    fn get_keys_for_action(config: &Config, action: ShortcutAction) -> &[String] {
        match action {
            ShortcutAction::Start => &config.start_keys,
            ShortcutAction::Stop => &config.stop_keys,
            ShortcutAction::Cancel => &config.cancel_keys,
            ShortcutAction::Pause => &config.pause_keys,
        }
    }

    fn build_patterns_for_state(
        state: ShortcutEngineState,
        buffer_ms: u64,
    ) -> Vec<Box<dyn Pattern>> {
        let config = Config::global();
        log::debug!("Building patterns for state {:?}", state);

        let pattern_configs: Vec<(&str, ShortcutAction)> = Self::get_available_actions(state)
            .iter()
            .filter_map(|&action| {
                let keys = Self::get_keys_for_action(&config, action);
                if action == ShortcutAction::Pause && keys.is_empty() {
                    None
                } else {
                    Some(action)
                }
            })
            .flat_map(|action| {
                Self::get_keys_for_action(&config, action)
                    .iter()
                    .map(move |key| (key.as_str(), action))
            })
            .collect();

        log::debug!(
            "Pattern configs for state {:?}: {:?}",
            state,
            pattern_configs
        );

        let mut conflict_resolver = ConflictResolver::new(buffer_ms);
        Self::parse_patterns_internal(pattern_configs, &mut conflict_resolver)
    }

    fn parse_patterns_internal(
        pattern_configs: Vec<(&str, ShortcutAction)>,
        conflict_resolver: &mut ConflictResolver,
    ) -> Vec<Box<dyn Pattern>> {
        let mut all_patterns = Vec::new();

        for (pattern_str, action) in pattern_configs {
            match parse_multiple_patterns(pattern_str, action) {
                Ok(patterns) => {
                    all_patterns.extend(patterns);
                }
                Err(e) => {
                    log::error!("Failed to parse pattern '{}': {}", pattern_str, e);
                }
            }
        }

        conflict_resolver.analyze_and_mark_conflicts(&mut all_patterns);

        all_patterns
    }

    pub fn set_state(&mut self, state: ShortcutEngineState) {
        if self.current_state != state {
            log::debug!("State transition: {:?} -> {:?}", self.current_state, state);
            self.current_state = state;

            let patterns = if let Some(stored_configs) = &self.pattern_configs {
                let available_actions = Self::get_available_actions(state);
                let filtered_configs: Vec<(&str, ShortcutAction)> = stored_configs
                    .iter()
                    .filter(|(_, action)| available_actions.contains(action))
                    .map(|(pattern, action)| (pattern.as_str(), *action))
                    .collect();

                let mut conflict_resolver = ConflictResolver::new(self.buffer_ms);
                Self::parse_patterns_internal(filtered_configs, &mut conflict_resolver)
            } else {
                Self::build_patterns_for_state(state, self.buffer_ms)
            };

            for pattern in &patterns {
                log::debug!(
                    "  Pattern: {:?} -> trigger key: {:?}",
                    pattern.get_type(),
                    pattern.get_trigger_key()
                );
            }
            self.matcher = ShortcutMatcher::new(patterns);
        }
    }

    pub fn process_event(&mut self, event: KeyEvent) -> Option<ShortcutAction> {
        self.process_event_with_time(event, Instant::now())
    }

    pub fn process_event_with_time(
        &mut self,
        event: KeyEvent,
        now: Instant,
    ) -> Option<ShortcutAction> {
        let (immediate_action, delayed_actions, cancelled_ids) =
            self.matcher.process_event(event, now);

        if immediate_action.is_some() && !delayed_actions.is_empty() {
            let error = MatcherError::ConflictingActions {
                has_immediate: immediate_action.is_some(),
                delayed_count: delayed_actions.len(),
                event_debug: format!("{:?}", event),
            };

            log::error!("Matcher error: {}", error);
            return None;
        }

        if !cancelled_ids.is_empty() {
            self.pending_delayed
                .retain(|d| !cancelled_ids.contains(&d.id));
        }

        if !delayed_actions.is_empty() {
            self.pending_delayed.extend(delayed_actions);
        }

        let triggered_action = self.check_and_trigger_delayed(now);

        let action = immediate_action.or(triggered_action);

        if let Some(action) = action {
            let new_state = Self::determine_next_state(action, self.current_state);
            self.set_state(new_state);
        }

        action
    }

    fn check_and_trigger_delayed(&mut self, now: Instant) -> Option<ShortcutAction> {
        let trigger_index = self
            .pending_delayed
            .iter()
            .position(|d| d.trigger_at <= now);

        if let Some(index) = trigger_index {
            let delayed_action = self.pending_delayed.get(index).map(|d| {
                log::debug!("Triggering delayed action: {:?}", d);
                d.action
            });

            if delayed_action.is_some() {
                self.pending_delayed.remove(index);
            }

            return delayed_action;
        }

        None
    }

    pub fn poll_delayed_action(&mut self) -> Option<ShortcutAction> {
        let action = self.check_and_trigger_delayed(Instant::now());

        if let Some(action) = action {
            let new_state = Self::determine_next_state(action, self.current_state);
            self.set_state(new_state);
        }

        action
    }

    pub fn get_expected_keys(&self) -> HashSet<Key> {
        self.matcher.get_expected_keys()
    }

    pub fn reset(&mut self) {
        self.matcher.reset();
    }

    pub fn check_delayed_actions(&mut self) {
        // This method is now redundant but kept for compatibility
        let _ = self.poll_delayed_action();
    }

    #[inline(always)]
    pub fn should_activate(&self, key: Key, is_press: bool) -> bool {
        if !is_press {
            return false;
        }

        self.matcher.can_activate_fast(key)
    }

    #[inline(always)]
    pub fn is_actively_listening(&self) -> bool {
        self.is_actively_listening.load(Ordering::Relaxed)
    }

    pub fn activate(&self) {
        log::debug!("Engine activated");
        self.is_actively_listening.store(true, Ordering::Relaxed);
        self.update_activity();
    }

    pub fn deactivate(&mut self) {
        log::debug!("Engine deactivated");
        self.is_actively_listening.store(false, Ordering::Relaxed);
        self.matcher.reset();
    }

    pub fn should_deactivate(&self) -> bool {
        if self.matcher.has_partial_matches() {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let last = self.last_activity.load(Ordering::Relaxed);
        (now - last) > 2000
    }

    #[inline(always)]
    pub fn update_activity(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_activity.store(now, Ordering::Relaxed);
    }

    fn determine_next_state(
        action: ShortcutAction,
        current_state: ShortcutEngineState,
    ) -> ShortcutEngineState {
        use crate::types::ShortcutAction::*;
        use crate::types::ShortcutEngineState::*;

        match action {
            Start => Active,
            Stop | Cancel => Idle,
            Pause => match current_state {
                Paused => Active,
                _ => Paused,
            },
        }
    }
}
