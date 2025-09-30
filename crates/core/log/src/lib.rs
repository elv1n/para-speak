mod init;

pub use init::init;
pub use tracing::{debug, error, info, trace, warn};
pub use tracing::Level;

use tracing_log::log;

pub struct SuppressLogs {
    previous_level: log::LevelFilter,
}

impl SuppressLogs {
    pub fn new() -> Self {
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Error);
        Self { previous_level }
    }
}

impl Drop for SuppressLogs {
    fn drop(&mut self) {
        log::set_max_level(self.previous_level);
    }
}

impl Default for SuppressLogs {
    fn default() -> Self {
        Self::new()
    }
}
