use chrono::Local;
use log::info;
use memory_stats::memory_stats;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub struct MemoryMonitor {
    shutdown_tx: Option<mpsc::Sender<()>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl MemoryMonitor {
    pub fn new() -> Option<Self> {
        if !config::Config::global().memory_monitor {
            return None;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let thread_handle = thread::spawn(move || {
            info!("Memory monitoring started");
            let mut last_report = Instant::now();
            let mut last_physical_mb: Option<f64> = None;

            loop {
                match shutdown_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(_) => {
                        info!("Memory monitoring shutting down");
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if last_report.elapsed() >= Duration::from_secs(10) {
                            if let Some((physical_mb, virtual_mb)) =
                                get_memory_stats_if_changed(last_physical_mb)
                            {
                                info!(
                                    "[{}] Memory Stats - Physical: {:.2} MB, Virtual: {:.2} MB",
                                    Local::now().format("%H:%M:%S"),
                                    physical_mb,
                                    virtual_mb
                                );
                                last_physical_mb = Some(physical_mb);
                            }
                            last_report = Instant::now();
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
        });

        Some(MemoryMonitor {
            shutdown_tx: Some(shutdown_tx),
            thread_handle: Some(thread_handle),
        })
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for MemoryMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

fn get_memory_stats_if_changed(last_physical_mb: Option<f64>) -> Option<(f64, f64)> {
    if let Some(usage) = memory_stats() {
        let physical_mb = usage.physical_mem as f64 / 1_048_576.0;
        let virtual_mb = usage.virtual_mem as f64 / 1_048_576.0;

        match last_physical_mb {
            None => Some((physical_mb, virtual_mb)),
            Some(last_phys) => {
                let physical_change_mb = (physical_mb - last_phys).abs();
                let threshold = if physical_change_mb < 30.0 { 20.0 } else { 5.0 };
                
                let physical_change_percent = if last_phys > 0.0 {
                    ((physical_mb - last_phys) / last_phys).abs() * 100.0
                } else {
                    100.0
                };

                if physical_change_percent > threshold {
                    Some((physical_mb, virtual_mb))
                } else {
                    None
                }
            }
        }
    } else {
        info!("Memory Stats - Unable to fetch memory statistics");
        None
    }
}
