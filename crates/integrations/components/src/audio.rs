use crate::{Component, ExecutionMode};
use anyhow::Result;
use std::any::{Any, TypeId};

#[derive(Debug)]
pub struct AudioComponent {
    name: String,
}

impl AudioComponent {
    pub fn new() -> Self {
        Self {
            name: "Audio".to_string(),
        }
    }
}

impl Default for AudioComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for AudioComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }

    fn preload(&self) -> Result<()> {
        audio::preload_sounds();
        Ok(())
    }

    fn on_start(&self) -> Result<()> {
        audio::play_start_sound();
        Ok(())
    }

    fn on_stop(&self) -> Result<()> {
        audio::play_stop_sound();
        Ok(())
    }

    fn on_pause(&self) -> Result<()> {
        audio::play_stop_sound();
        Ok(())
    }

    fn on_resume(&self) -> Result<()> {
        audio::play_start_sound();
        Ok(())
    }

    fn on_cancel(&self) -> Result<()> {
        audio::play_stop_sound();
        Ok(())
    }

    fn on_processing_complete(&self, _result: &str) -> Result<()> {
        audio::play_complete_sound();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}