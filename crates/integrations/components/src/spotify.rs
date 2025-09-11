use crate::{Component, ExecutionMode};
use anyhow::Result;
use config::Config;
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum VolumeLevel {
    Normal,
    Reduced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum VolumeState {
    Normal = 0,
    Reducing = 1,
    Reduced = 2,
    Restoring = 3,
}

impl From<u8> for VolumeState {
    fn from(val: u8) -> Self {
        match val {
            0 => VolumeState::Normal,
            1 => VolumeState::Reducing,
            2 => VolumeState::Reduced,
            3 => VolumeState::Restoring,
            _ => VolumeState::Normal,
        }
    }
}

#[derive(Error, Debug)]
pub enum SpotifyError {
    #[error("Configuration conflict: {0}")]
    Config(String),
    #[error("Mutex lock error")]
    Lock,
    #[error("Spotify operation failed: {0}")]
    Operation(String),
}

#[derive(Debug)]
pub struct SpotifyComponent {
    name: String,
    original_volume: Arc<Mutex<Option<u8>>>,
    volume_state: AtomicU8,
    operation_counter: AtomicU64,
}

impl SpotifyComponent {
    pub fn new() -> Self {
        Self {
            name: "Spotify".to_string(),
            original_volume: Arc::new(Mutex::new(None)),
            volume_state: AtomicU8::new(VolumeState::Normal as u8),
            operation_counter: AtomicU64::new(0),
        }
    }

    fn get_current_state(&self) -> VolumeState {
        VolumeState::from(self.volume_state.load(Ordering::Acquire))
    }

    fn try_transition_state(&self, from: VolumeState, to: VolumeState) -> bool {
        self.volume_state
            .compare_exchange(from as u8, to as u8, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn next_operation_id(&self) -> u64 {
        self.operation_counter.fetch_add(1, Ordering::AcqRel)
    }

    fn handle_volume_change(&self, level: VolumeLevel) -> Result<()> {
        let operation_id = self.next_operation_id();

        let config = Config::global();
        let recording_volume = config.spotify_recording_volume.map(|v| v as u8);
        let reduce_by = config.spotify_reduce_by.map(|v| v as u8);

        if recording_volume.is_some() && reduce_by.is_some() {
            return Err(anyhow::anyhow!(SpotifyError::Config(
                "spotify_recording_volume and spotify_reduce_by are mutually exclusive".to_string()
            )));
        }

        if recording_volume.is_none() && reduce_by.is_none() {
            log::debug!(
                "No Spotify volume configuration, skipping operation {}",
                operation_id
            );
            return Ok(());
        }

        match level {
            VolumeLevel::Reduced => {
                self.handle_reduce_volume(operation_id, recording_volume, reduce_by)
            }
            VolumeLevel::Normal => self.handle_restore_volume(operation_id),
        }
    }

    fn handle_reduce_volume(
        &self,
        operation_id: u64,
        recording_volume: Option<u8>,
        reduce_by: Option<u8>,
    ) -> Result<()> {
        let current_state = self.get_current_state();

        match current_state {
            VolumeState::Reduced => {
                log::debug!(
                    "Volume already reduced, skipping operation {}",
                    operation_id
                );
                return Ok(());
            }
            VolumeState::Reducing => {
                log::debug!(
                    "Volume currently reducing, skipping operation {}",
                    operation_id
                );
                return Ok(());
            }
            VolumeState::Restoring => {
                log::debug!(
                    "Volume currently restoring, waiting then retrying operation {}",
                    operation_id
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
                return self.handle_reduce_volume(operation_id, recording_volume, reduce_by);
            }
            VolumeState::Normal => {}
        }

        if !self.try_transition_state(VolumeState::Normal, VolumeState::Reducing) {
            log::debug!(
                "Failed to transition to reducing state for operation {}",
                operation_id
            );
            return Ok(());
        }

        let result = self.perform_volume_reduction(operation_id, recording_volume, reduce_by);

        match &result {
            Ok(_) => {
                if !self.try_transition_state(VolumeState::Reducing, VolumeState::Reduced) {
                    log::warn!(
                        "Failed to transition to reduced state after successful operation {}",
                        operation_id
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Volume reduction failed for operation {}: {}",
                    operation_id,
                    e
                );
                if !self.try_transition_state(VolumeState::Reducing, VolumeState::Normal) {
                    log::error!(
                        "Failed to restore normal state after failed operation {}",
                        operation_id
                    );
                }
            }
        }

        result
    }

    fn perform_volume_reduction(
        &self,
        operation_id: u64,
        recording_volume: Option<u8>,
        reduce_by: Option<u8>,
    ) -> Result<()> {
        let current_volume = self.get_current_spotify_volume()?;
        log::debug!(
            "Current Spotify volume: {} (operation: {})",
            current_volume,
            operation_id
        );

        {
            let mut original = self
                .original_volume
                .lock()
                .map_err(|_| anyhow::anyhow!(SpotifyError::Lock))?;
            if original.is_none() {
                *original = Some(current_volume);
                log::debug!(
                    "Stored original volume: {} (operation: {})",
                    current_volume,
                    operation_id
                );
            }
        }

        let target_volume = if let Some(exact_volume) = recording_volume {
            exact_volume
        } else if let Some(reduction) = reduce_by {
            current_volume.saturating_sub(reduction)
        } else {
            return Err(anyhow::anyhow!(SpotifyError::Config(
                "No Spotify volume configuration provided".to_string()
            )));
        };

        log::debug!(
            "Reducing Spotify volume from {} to {} (operation: {})",
            current_volume,
            target_volume,
            operation_id
        );

        self.set_spotify_volume(target_volume)
    }

    fn handle_restore_volume(&self, operation_id: u64) -> Result<()> {
        let current_state = self.get_current_state();

        match current_state {
            VolumeState::Normal => {
                log::debug!("Volume already normal, skipping operation {}", operation_id);
                return Ok(());
            }
            VolumeState::Restoring => {
                log::debug!(
                    "Volume currently restoring, skipping operation {}",
                    operation_id
                );
                return Ok(());
            }
            VolumeState::Reducing => {
                log::debug!(
                    "Volume currently reducing, waiting then retrying operation {}",
                    operation_id
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
                return self.handle_restore_volume(operation_id);
            }
            VolumeState::Reduced => {}
        }

        if !self.try_transition_state(VolumeState::Reduced, VolumeState::Restoring) {
            log::debug!(
                "Failed to transition to restoring state for operation {}",
                operation_id
            );
            return Ok(());
        }

        let result = self.perform_volume_restoration(operation_id);

        match &result {
            Ok(_) => {
                if !self.try_transition_state(VolumeState::Restoring, VolumeState::Normal) {
                    log::warn!(
                        "Failed to transition to normal state after successful operation {}",
                        operation_id
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Volume restoration failed for operation {}: {}",
                    operation_id,
                    e
                );
                if !self.try_transition_state(VolumeState::Restoring, VolumeState::Reduced) {
                    log::error!(
                        "Failed to restore reduced state after failed operation {}",
                        operation_id
                    );
                }
            }
        }

        result
    }

    fn perform_volume_restoration(&self, operation_id: u64) -> Result<()> {
        let volume_to_restore = {
            let mut original = self
                .original_volume
                .lock()
                .map_err(|_| anyhow::anyhow!(SpotifyError::Lock))?;
            original.take()
        };

        if let Some(volume) = volume_to_restore {
            log::debug!(
                "Restoring Spotify volume to {} (operation: {})",
                volume,
                operation_id
            );
            self.set_spotify_volume(volume)
        } else {
            log::debug!(
                "No original volume to restore (operation: {})",
                operation_id
            );
            Ok(())
        }
    }

    fn get_current_spotify_volume(&self) -> Result<u8> {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("osascript")
                .arg("-e")
                .arg("tell application \"Spotify\" to get sound volume")
                .output()?;

            if output.status.success() {
                let volume_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                volume_str.parse::<u8>().map_err(|_| {
                    anyhow::anyhow!(SpotifyError::Operation(format!(
                        "Failed to parse volume: {}",
                        volume_str
                    )))
                })
            } else {
                Err(anyhow::anyhow!(SpotifyError::Operation(
                    "Failed to get Spotify volume".to_string()
                )))
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(50)
        }
    }

    fn set_spotify_volume(&self, volume: u8) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"Spotify\" to set sound volume to {}",
                    volume
                ))
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(SpotifyError::Operation(
                    "Failed to set Spotify volume".to_string()
                )));
            }
        }

        Ok(())
    }
}

impl Default for SpotifyComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SpotifyComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }

    fn on_start(&self) -> Result<()> {
        let _ = self.handle_volume_change(VolumeLevel::Reduced);
        Ok(())
    }

    fn on_pause(&self) -> Result<()> {
        let _ = self.handle_volume_change(VolumeLevel::Normal);
        Ok(())
    }

    fn on_resume(&self) -> Result<()> {
        let _ = self.handle_volume_change(VolumeLevel::Reduced);
        Ok(())
    }

    fn on_cancel(&self) -> Result<()> {
        let _ = self.handle_volume_change(VolumeLevel::Normal);
        Ok(())
    }

    fn on_processing_complete(&self, _result: &str) -> Result<()> {
        let _ = self.handle_volume_change(VolumeLevel::Normal);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
