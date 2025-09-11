use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum MatcherError {
    ConflictingActions {
        has_immediate: bool,
        delayed_count: usize,
        event_debug: String,
    },
    InvalidState {
        message: String,
    },
    ProcessingError {
        message: String,
    },
}

impl fmt::Display for MatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingActions {
                has_immediate,
                delayed_count,
                event_debug,
            } => write!(
                f,
                "Conflicting actions detected: immediate={}, delayed_count={}, event={}",
                has_immediate, delayed_count, event_debug
            ),
            Self::InvalidState { message } => write!(f, "Invalid state: {}", message),
            Self::ProcessingError { message } => write!(f, "Processing error: {}", message),
        }
    }
}

impl Error for MatcherError {}
