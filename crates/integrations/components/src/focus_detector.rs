use crate::Component;
use anyhow::Result;
use config::Config;
use focus_detector::{current_input_target, FocusInfo};
use std::any::{Any, TypeId};

#[derive(Debug)]
pub struct FocusDetectorComponent {
    name: String,
}

impl Default for FocusDetectorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusDetectorComponent {
    pub fn new() -> Self {
        Self {
            name: "FocusDetectorComponent".to_string(),
        }
    }

    fn print_focus_details(&self, event: &str) {
        let config = Config::global();
        if !config.debug {
            return;
        }

        let focus_result = std::panic::catch_unwind(|| {
            current_input_target()
        });

        match focus_result {
            Ok(focus_info) => {
                self.log_focus_info(event, &focus_info);
            }
            Err(_) => {
                println!("[FOCUS-DEBUG] {} - Failed to get focus information", event);
            }
        }
    }

    fn log_focus_info(&self, event: &str, info: &FocusInfo) {
        println!(
            "[FOCUS-DEBUG] {} - App: '{}' | Window: '{}'",
            event, info.app_name, info.window_title
        );
    }
}

impl Component for FocusDetectorComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn on_start(&self) -> Result<()> {
        self.print_focus_details("RECORDING_START");
        Ok(())
    }

    fn on_pause(&self) -> Result<()> {
        self.print_focus_details("RECORDING_PAUSE");
        Ok(())
    }

    fn on_resume(&self) -> Result<()> {
        self.print_focus_details("RECORDING_RESUME");
        Ok(())
    }

    fn on_stop(&self) -> Result<()> {
        self.print_focus_details("RECORDING_STOP");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
