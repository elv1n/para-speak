use parking_lot::RwLock;
use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use shortcut_matcher::{KeyEvent, MatcherEngine, ShortcutHandler};

pub struct ShortcutListener {
    engine: Arc<RwLock<MatcherEngine>>,
    handler: Arc<dyn ShortcutHandler>,
}

pub struct ListenerHandle {
    handle: Option<JoinHandle<Result<(), String>>>,
}

impl ListenerHandle {
    pub fn join(mut self) -> Result<(), String> {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .unwrap_or_else(|_| Err("Listener thread panicked".into()))
        } else {
            Ok(())
        }
    }

    pub fn join_with_timeout(mut self, timeout: Duration) -> Result<(), String> {
        if let Some(handle) = self.handle.take() {
            let start = std::time::Instant::now();
            while !handle.is_finished() {
                if start.elapsed() > timeout {
                    log::warn!("Listener thread did not stop within timeout, abandoning");
                    return Err("Listener thread timeout".into());
                }
                thread::sleep(Duration::from_millis(10));
            }
            handle
                .join()
                .unwrap_or_else(|_| Err("Listener thread panicked".into()))
        } else {
            Ok(())
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| h.is_finished())
            .unwrap_or(true)
    }
}

#[derive(Clone)]
pub struct ListenerControl {
    shutdown_tx: mpsc::Sender<()>,
    should_stop: Arc<AtomicBool>,
}

impl ShortcutListener {
    pub fn new(handler: Arc<dyn ShortcutHandler>) -> Self {
        let engine = MatcherEngine::default();

        Self {
            engine: Arc::new(RwLock::new(engine)),
            handler,
        }
    }

    pub fn spawn(self) -> (ListenerHandle, ListenerControl) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();

        let handle = thread::spawn(move || self.run_listen_loop(shutdown_rx, should_stop_clone));

        let control = ListenerControl {
            shutdown_tx,
            should_stop,
        };

        (
            ListenerHandle {
                handle: Some(handle),
            },
            control,
        )
    }

    fn run_listen_loop(
        self,
        shutdown_rx: mpsc::Receiver<()>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let engine = self.engine.clone();
        let handler = self.handler.clone();
        let should_stop_clone = should_stop.clone();

        thread::spawn(move || {
            if shutdown_rx.recv().is_ok() {
                should_stop_clone.store(true, Ordering::SeqCst);
                log::info!(
                    "Shutdown signal received - listen will exit on next user input or timeout"
                );
            }
        });

        let should_stop_listen = should_stop.clone();
        let listen_callback = move |event: Event| {
            if should_stop_listen.load(Ordering::SeqCst) {
                return;
            }

            let (key, is_press) = match event.event_type {
                EventType::KeyPress(k) => (k, true),
                EventType::KeyRelease(k) => (k, false),
                _ => return,
            };

            let mut engine = engine.write();

            let key_event = if is_press {
                KeyEvent::Press(key)
            } else {
                KeyEvent::Release(key)
            };

            let action = engine.process_event(key_event);

            if let Some(action) = action {
                log::debug!("Action triggered: {:?}", action);
                handler.handle_action(action);

                // return None; // listen cannot consume events
            }

            // Some(event) // listen passes all events through
        };

        let engine_delayed = self.engine.clone();
        let handler_delayed = self.handler.clone();
        let should_stop_delayed = should_stop.clone();

        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(10));

            if should_stop_delayed.load(Ordering::SeqCst) {
                break;
            }

            let mut engine = engine_delayed.write();
            if let Some(action) = engine.poll_delayed_action() {
                log::debug!("Delayed action triggered: {:?}", action);
                handler_delayed.handle_action(action);
            }
        });

        if let Err(e) = listen(listen_callback) {
            return Err(format!("Event listen failed: {:?}", e));
        }

        log::info!("Event listen stopped cleanly");
        Ok(())
    }
}

impl ListenerControl {
    pub fn stop(&self) -> Result<(), String> {
        log::info!("Initiating listener shutdown");

        self.should_stop.store(true, Ordering::SeqCst);

        self.shutdown_tx
            .send(())
            .map_err(|_| "Listener already stopped".to_string())?;

        log::info!("Shutdown signal sent to listener thread");
        Ok(())
    }

    pub fn is_stopping(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }
}
