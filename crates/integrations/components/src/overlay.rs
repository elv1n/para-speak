use crate::{Component, ExecutionMode};
use anyhow::Result;
use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

pub struct OverlayManager;

impl OverlayManager {
    pub fn global() -> Arc<Mutex<Option<Self>>> {
        Arc::new(Mutex::new(Some(Self)))
    }

    pub fn show(&self, _message: &str) -> Result<()> {
        log::info!("Overlay: {}", _message);
        Ok(())
    }

    pub fn show_with_timeout(&self, _message: &str, _timeout: u32) -> Result<()> {
        log::info!("Overlay ({}ms): {}", _timeout, _message);
        Ok(())
    }
}

pub mod overlay_manager {
    pub use super::OverlayManager;
}

pub struct Manager;

impl Manager {
    pub fn init_global() -> Result<()> {
        Ok(())
    }
}

pub struct OverlayComponent {
    name: String,
    manager: Arc<Mutex<Option<OverlayManager>>>,
}

impl std::fmt::Debug for OverlayComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayComponent")
            .field("name", &self.name)
            .finish()
    }
}

impl OverlayComponent {
    pub fn new() -> Self {
        Self {
            name: "Overlay".to_string(),
            manager: OverlayManager::global(),
        }
    }
}

impl Default for OverlayComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for OverlayComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }

    fn initialize(&self) -> Result<()> {
        Manager::init_global()
            .map_err(|e| anyhow::anyhow!("Failed to initialize overlay: {}", e))?;
        Ok(())
    }

    fn on_start(&self) -> Result<()> {
        self.show_message("🎤 Recording...", None);
        Ok(())
    }

    fn on_stop(&self) -> Result<()> {
        self.show_message("⏳ Processing...", None);
        Ok(())
    }

    fn on_pause(&self) -> Result<()> {
        self.show_message("⏸️ Paused", Some(1500));
        Ok(())
    }

    fn on_resume(&self) -> Result<()> {
        self.show_message("🎤 Recording...", None);
        Ok(())
    }

    fn on_cancel(&self) -> Result<()> {
        self.show_message("⏸️ Recording cancelled", Some(1500));
        Ok(())
    }

    fn on_processing_complete(&self, result: &str) -> Result<()> {
        let display = if result.len() > 40 {
            format!("✅ {}", &result[..37].trim())
        } else {
            format!("✅ {}", result)
        };
        self.show_message(&display, Some(2500));
        Ok(())
    }

    fn on_error(&self, error: &str) -> Result<()> {
        self.show_message(&format!("❌ {}", error), Some(2000));
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl OverlayComponent {
    fn show_message(&self, message: &str, timeout_ms: Option<u32>) {
        if let Ok(manager_guard) = self.manager.lock() {
            if let Some(manager) = manager_guard.as_ref() {
                if let Some(timeout) = timeout_ms {
                    let _ = manager.show_with_timeout(message, timeout);
                } else {
                    let _ = manager.show(message);
                }
            }
        }
    }
}