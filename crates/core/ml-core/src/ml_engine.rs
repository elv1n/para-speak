use crate::ml_error::{Result, TranscriptionError};
use ml_utils::{get_model_cache_path, model_exists};
use config::Config;
use log::{debug, info};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::atomic::{AtomicU8, Ordering};

const PYTHON_PATH: &str = "/python";
const VENV_SITE_PACKAGES: &str = "/python/venv/lib";

const AUDIO_SAMPLE_RATE: u32 = 48000;
const AUDIO_CHANNELS: u32 = 1;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
enum ModelState {
    NotInitialized = 0,
    Initialized = 1,
    ModelLoaded = 2,
}

pub struct MLEngine {
    python_env: PythonEnvironment,
    model_state: AtomicU8,
    current_model: Option<String>,
}

impl MLEngine {
    pub fn new() -> Self {
        Self {
            python_env: PythonEnvironment::new(),
            model_state: AtomicU8::new(ModelState::NotInitialized as u8),
            current_model: None,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        if self.is_initialized() {
            return Ok(());
        }

        let config = Config::global();
        let model_name = config.model_name();

        if !model_exists(model_name) {
            return Err(TranscriptionError::ModelLoadingError(
                format!("Model '{}' not found. Please download it first by running:\n  cargo run -p verify-cli", model_name)
            ));
        }

        let model_path = get_model_cache_path(model_name);
        log::debug!("[MLEngine] Model '{}' available at: {:?}", model_name, model_path);

        self.python_env.initialize()?;
        self.model_state
            .store(ModelState::Initialized as u8, Ordering::Release);
        Ok(())
    }

    pub fn load_model(&mut self, model_type: &str) -> Result<()> {
        if !self.is_initialized() {
            return Err(TranscriptionError::NotInitialized);
        }

        // check if it's the same model
        if self.is_model_loaded() && self.current_model.as_deref() == Some(model_type) {
            debug!("[MLEngine] Model {} already loaded", model_type);
            return Ok(());
        }

        if self.is_model_loaded() {
            //unload old model
            self.unload_model()?;
        }

        let loaded_model = self.python_env.load_model(model_type)?;
        self.current_model = Some(loaded_model.clone());
        self.model_state
            .store(ModelState::ModelLoaded as u8, Ordering::Release);
        Ok(())
    }

    pub fn transcribe(&self, audio_data: &[u8]) -> Result<String> {
        if !self.is_model_loaded() {
            return Err(TranscriptionError::ModelNotLoaded);
        }

        self.python_env
            .transcribe_raw(audio_data, AUDIO_SAMPLE_RATE, AUDIO_CHANNELS)
    }

    pub fn unload_model(&mut self) -> Result<()> {
        if !self.is_model_loaded() {
            return Ok(());
        }

        self.python_env.unload_model()?;
        self.current_model = None;
        self.model_state
            .store(ModelState::Initialized as u8, Ordering::Release);
        info!("[MLEngine] Model unloaded");
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.model_state.load(Ordering::Acquire) >= ModelState::Initialized as u8
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_state.load(Ordering::Acquire) == ModelState::ModelLoaded as u8
    }
}

struct PythonEnvironment {
    daemon: Option<PyObject>,
}

impl PythonEnvironment {
    fn new() -> Self {
        Self { daemon: None }
    }

    fn initialize(&mut self) -> Result<()> {
        Python::with_gil(|py| {
            self.setup_python_paths(py)?;
            self.create_daemon(py)?;
            Ok(())
        })
    }

    fn setup_python_paths(&self, py: Python) -> PyResult<()> {
        let sys = py.import("sys")?;
        let path_list = sys.getattr("path")?;

        let os = py.import("os")?;
        let cwd = os.call_method0("getcwd")?;
        let cwd_str = cwd.extract::<String>()?;

        let python_path = format!("{}{}", cwd_str, PYTHON_PATH);
        path_list.call_method1("append", (python_path,))?;
        
        // Add project root so "python" module can be found
        path_list.call_method1("append", (cwd_str.clone(),))?;

        let glob_module = py.import("glob")?;
        let venv_pattern = format!("{}{}/*/site-packages", cwd_str, VENV_SITE_PACKAGES);
        let site_packages_dirs = glob_module.call_method1("glob", (venv_pattern,))?;

        if let Ok(iter) = site_packages_dirs.extract::<Vec<String>>() {
            for dir_path in iter {
                path_list.call_method1("insert", (0, dir_path))?;
            }
        }

        let locale = py.import("locale")?;
        locale.call_method1("setlocale", (locale.getattr("LC_ALL")?, "en_US.UTF-8"))?;

        Ok(())
    }

    fn create_daemon(&mut self, py: Python) -> PyResult<()> {
        let ml_daemon_module = py.import("ml_daemon")?;
        let ml_daemon_class = ml_daemon_module.getattr("MLDaemon")?;
        let daemon_instance = ml_daemon_class.call0()?;
        self.daemon = Some(daemon_instance.into());
        Ok(())
    }

    fn load_model(&self, model_type: &str) -> Result<String> {
        Python::with_gil(|py| {
            let daemon = self
                .daemon
                .as_ref()
                .ok_or(TranscriptionError::NotInitialized)?;

            daemon
                .call_method1(py, "load_model", (model_type,))?
                .extract::<String>(py)
                .map_err(|e| TranscriptionError::ModelLoadingError(e.to_string()))
        })
    }

    fn transcribe_raw(&self, audio_data: &[u8], sample_rate: u32, channels: u32) -> Result<String> {
        Python::with_gil(|py| {
            let daemon = self
                .daemon
                .as_ref()
                .ok_or(TranscriptionError::NotInitialized)?;

            let py_bytes = PyBytes::new(py, audio_data);
            daemon
                .call_method1(py, "transcribe_raw", (py_bytes, sample_rate, channels))?
                .extract::<String>(py)
                .map_err(|e| TranscriptionError::TranscriptionFailed(e.to_string()))
        })
    }

    fn unload_model(&mut self) -> Result<()> {
        if let Some(daemon) = &self.daemon {
            Python::with_gil(|py| {
                // Try to unload the model - best effort during shutdown
                let unload_result = daemon.call_method0(py, "unload_model");
                match unload_result {
                    Ok(result) => {
                        if let Ok(msg) = result.extract::<String>(py) {
                            if msg.contains("interrupted during shutdown") {
                                debug!("[PythonEnvironment] Model unload interrupted (expected during shutdown)");
                            } else {
                                debug!("[PythonEnvironment] {}", msg);
                            }
                        }
                    }
                    Err(e) => {
                        // Check if this is a keyboard interrupt during shutdown
                        let error_str = e.to_string();
                        if error_str.contains("KeyboardInterrupt") {
                            debug!("[PythonEnvironment] Model unload interrupted by signal (expected during shutdown)");
                        } else {
                            // Log but don't fail on unload errors during shutdown
                            debug!("[PythonEnvironment] Model unload error (continuing): {}", e);
                        }
                    }
                }

                // Try cleanup - also best effort
                let cleanup_result = daemon.call_method0(py, "cleanup");
                match cleanup_result {
                    Ok(result) => {
                        if let Ok(msg) = result.extract::<String>(py) {
                            debug!("[PythonEnvironment] {}", msg);
                        }
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        if !error_str.contains("KeyboardInterrupt") {
                            debug!("[PythonEnvironment] Cleanup error (continuing): {}", e);
                        }
                    }
                }

                Ok::<(), TranscriptionError>(())
            })?;
        }
        Ok(())
    }
}

impl Drop for MLEngine {
    fn drop(&mut self) {
        if self.is_model_loaded() {
            let _ = self.unload_model();
        }
    }
}
