mod conflict_resolver;
mod error;
mod handler;
mod matcher;
mod patterns;
mod types;

pub mod engine;
pub mod parser;

pub use engine::MatcherEngine;
pub use error::MatcherError;
pub use handler::ShortcutHandler;
pub use parser::{parse_multiple_patterns, parse_pattern, ParseError};
pub use types::{KeyEvent, MatchResult, ShortcutAction, ShortcutEngineState};
