use crate::controller::Controllers;
use crate::memory_monitor::MemoryMonitor;
use crate::permissions::PermissionManager;
use anyhow::Result;
use log::{error, info};
use shortcut_listener::{ListenerControl, ListenerHandle, ShortcutListener};
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

enum MainEvent {
    Shutdown,
}

pub struct ParaSpeakApp;

impl Default for ParaSpeakApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ParaSpeakApp {
    pub fn new() -> Self {
        Self
    }

    pub fn run(self) -> Result<()> {
        info!("Starting para-speak ...");

        let mut memory_monitor = if config::Config::global().memory_monitor {
            MemoryMonitor::new()
        } else {
            None
        };

        let manager = PermissionManager::new();
        manager.ensure_permissions()?;

        let controllers = Arc::new(Controllers::new()?);
        let listener = ShortcutListener::new(controllers.clone());

        let (listener_handle, control) = listener.spawn();

        let (event_tx, event_rx) = mpsc::channel();
        let event_tx_clone = event_tx.clone();

        thread::spawn(move || {
            if let Err(e) = Self::setup_signal_handlers(event_tx_clone) {
                error!("Failed to setup signal handlers: {}", e);
            }
        });

        info!("para-speak is running");
        info!("Press Ctrl+C to exit");

        self.run_main_event_loop(event_rx, control, listener_handle, controllers)?;

        if let Some(ref mut monitor) = memory_monitor {
            monitor.stop();
        }

        info!("Shutdown complete");
        Ok(())
    }

    fn run_main_event_loop(
        &self,
        event_rx: mpsc::Receiver<MainEvent>,
        control: ListenerControl,
        listener_handle: ListenerHandle,
        controllers: Arc<Controllers>,
    ) -> Result<()> {
        loop {
            match event_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(MainEvent::Shutdown) => {
                    info!("Received shutdown request - rdev grab cannot exit cleanly, forcing process exit");

                    if let Err(e) = control.stop() {
                        error!("Failed to send stop signal to listener: {}", e);
                    } else {
                        info!("Stop signal sent to listener");
                    }

                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if listener_handle.is_finished() {
                        error!("Listener thread died unexpectedly");
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Event channel disconnected unexpectedly");
                    if let Err(e) = control.stop() {
                        error!("Failed to stop listener after channel disconnect: {}", e);
                    }
                    break;
                }
            }
        }

        info!("Shutting down controllers...");
        if let Err(e) = controllers.shutdown() {
            error!("Failed to shutdown controllers: {}", e);
        } else {
            info!("Controllers shutdown complete");
        }

        thread::sleep(Duration::from_millis(100));

        info!("Process termination");
        // Return cleanly to allow destructors and shutdown hooks to run
        Ok(())
    }

    fn setup_signal_handlers(event_tx: mpsc::Sender<MainEvent>) -> Result<()> {
        let mut signals = Signals::new([SIGTERM, SIGINT, SIGQUIT, SIGHUP])?;
        info!("Signal handlers installed for SIGTERM, SIGINT, SIGQUIT, SIGHUP");

        for sig in signals.forever() {
            match sig {
                SIGTERM => info!("Received SIGTERM signal"),
                SIGINT => info!("Received SIGINT signal (Ctrl+C)"),
                SIGQUIT => info!("Received SIGQUIT signal"),
                SIGHUP => info!("Received SIGHUP signal"),
                _ => continue,
            }

            match event_tx.send(MainEvent::Shutdown) {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    error!("CRITICAL: Failed to send shutdown event: {}", e);
                    error!("Application may not shut down cleanly");
                    break;
                }
            }
        }
        info!("Signal handler thread exiting");
        Ok(())
    }
}
