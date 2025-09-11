pub mod audio;
pub mod focus_detector;
pub mod overlay;
pub mod spotify;
pub mod transcription_handler;

pub use audio::AudioComponent;
pub use focus_detector::FocusDetectorComponent;
pub use overlay::OverlayComponent;
pub use spotify::SpotifyComponent;
pub use transcription_handler::TranscriptionHandler;

use anyhow::Result;
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Parallel,
    Sequential,
}

pub trait Component: Send + Sync + Debug {
    fn name(&self) -> &str;

    fn type_id(&self) -> TypeId;

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }

    fn preload(&self) -> Result<()> {
        Ok(())
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn on_start(&self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> Result<()> {
        Ok(())
    }

    fn on_pause(&self) -> Result<()> {
        Ok(())
    }

    fn on_resume(&self) -> Result<()> {
        Ok(())
    }

    fn on_cancel(&self) -> Result<()> {
        Ok(())
    }

    fn on_processing_start(&self) -> Result<()> {
        Ok(())
    }

    fn on_processing_complete(&self, _result: &str) -> Result<()> {
        Ok(())
    }

    fn on_error(&self, _error: &str) -> Result<()> {
        Ok(())
    }

    fn on_partial_processing_start(&self) -> Result<()> {
        Ok(())
    }

    fn on_partial_processing_complete(&self, _result: &str) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any;
}

pub type ComponentRef = Arc<dyn Component>;

#[derive(Debug)]
pub struct ComponentMetadata {
    pub name: String,
    pub type_id: TypeId,
    pub execution_mode: ExecutionMode,
    pub dependencies: Vec<TypeId>,
}

impl ComponentMetadata {
    pub fn new(name: impl Into<String>, type_id: TypeId) -> Self {
        Self {
            name: name.into(),
            type_id,
            execution_mode: ExecutionMode::Parallel,
            dependencies: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<TypeId>) -> Self {
        self.dependencies = deps;
        self
    }
}