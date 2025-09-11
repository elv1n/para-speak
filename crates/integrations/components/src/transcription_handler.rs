use crate::{Component, ExecutionMode};
use anyhow::Result;
use clipboard::{insert_text_at_cursor, set_clipboard};
use config::Config;
use std::any::{Any, TypeId};

#[derive(Debug)]
pub struct TranscriptionHandler;

impl TranscriptionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranscriptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TranscriptionHandler {
    fn name(&self) -> &str {
        "TranscriptionHandler"
    }
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn on_processing_complete(&self, result: &str) -> Result<()> {
        if result.is_empty() {
            log::warn!("Empty transcription received");
            return Ok(());
        }

        let config = Config::global();

        if config.paste {
            insert_text_at_cursor(result);
        } else {
            set_clipboard(result);
        }
        Ok(())
    }

    fn on_partial_processing_complete(&self, result: &str) -> Result<()> {
        if result.is_empty() {
            log::warn!("Empty transcription received");
            return Ok(());
        }
        log::info!("Partial transcription received: {}", result);
        Ok(())
    }

    fn on_error(&self, error: &str) -> Result<()> {
        log::error!("Transcription failed: {}", error);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
