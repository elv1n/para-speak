use ml_utils::{get_default_model, is_model_supported, AVAILABLE_MODELS};
use crate::parse_replace_pairs::parse_replace_pairs;
use clap::Parser;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

static CONFIG: OnceLock<Arc<Config>> = OnceLock::new();

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
thread_local! {
    static TEST_CONFIG: Mutex<Option<Arc<Config>>> = const { Mutex::new(None) };
}

pub trait ShortcutConfigProvider {
    fn start_keys(&self) -> &[String];
    fn stop_keys(&self) -> &[String];
    fn cancel_keys(&self) -> &[String];
    fn pause_keys(&self) -> &[String];
}

#[derive(Parser, Debug, Clone)]
#[command(name = "para-speak")]
#[command(about = "Speech-to-text utility for macOS", long_about = None)]
#[command(version)]
pub struct Config {
    #[arg(short = 'd', long, env = "PARA_DEBUG", help = "Enable debug mode")]
    pub debug: bool,

    #[arg(
        long,
        env = "PARA_PASTE",
        help = "Enable automatic pasting of transcribed text"
    )]
    pub paste: bool,

    #[arg(
        long,
        env = "PARA_REALTIME",
        help = "Enable real-time transcription mode with streaming output"
    )]
    pub realtime: bool,

    #[arg(
        long = "spotify-recording-volume",
        env = "PARA_SPOTIFY_RECORDING_VOLUME",
        help = "Set Spotify volume to this level during recording (0-100)"
    )]
    pub spotify_recording_volume: Option<u32>,

    #[arg(
        long = "spotify-reduce-by",
        env = "PARA_SPOTIFY_REDUCE_BY",
        help = "Reduce Spotify volume by this amount during recording (0-100)"
    )]
    pub spotify_reduce_by: Option<u32>,

    #[arg(
        long = "start-keys",
        env = "PARA_START_KEYS",
        value_delimiter = ';',
        required = false,
        help = "Comma-separated list of key combinations to start recording"
    )]
    pub start_keys: Vec<String>,

    #[arg(
        long = "stop-keys",
        env = "PARA_STOP_KEYS",
        value_delimiter = ';',
        required = false,
        help = "Comma-separated list of key combinations to stop recording"
    )]
    pub stop_keys: Vec<String>,

    #[arg(
        long = "cancel-keys",
        env = "PARA_CANCEL_KEYS",
        value_delimiter = ';',
        required = false,
        help = "Comma-separated list of key combinations to cancel recording"
    )]
    pub cancel_keys: Vec<String>,

    #[arg(
        long = "pause-keys",
        env = "PARA_PAUSE_KEYS",
        value_delimiter = ';',
        required = false,
        help = "Comma-separated list of key combinations to pause recording"
    )]
    pub pause_keys: Vec<String>,

    #[arg(
        long = "transcribe-on-pause",
        env = "PARA_TRANSCRIBE_ON_PAUSE",
        help = "Enable transcription on pause"
    )]
    pub transcribe_on_pause: bool,



    #[arg(
        long = "shortcut-resolution-delay-ms",
        env = "PARA_SHORTCUT_RESOLUTION_DELAY_MS",
        help = "Delay for resolving conflicts between shortcuts sharing the same key (default: 50ms)"
    )]
    pub shortcut_resolution_delay_ms: Option<u64>,

    #[arg(
        long = "memory-monitor",
        env = "PARA_MEMORY_MONITOR",
        help = "Enable memory monitoring (reports every 10 seconds)"
    )]
    pub memory_monitor: bool,

    #[arg(skip)]
    pub sample_rate: u32,

    #[arg(skip)]
    pub initial_buffer_seconds: u32,

    #[arg(skip)]
    pub transcription_replace_text: HashMap<String, Option<String>>,

    #[arg(
        long = "model",
        env = "PARA_MODEL",
        help = "ML model to use for transcription"
    )]
    pub model: Option<String>,

    #[arg(
        long = "force",
        env = "PARA_FORCE",
        help = "Force using an unsupported model (use at your own risk)"
    )]
    pub force: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        let _ = dotenv::dotenv();
        let _ = dotenv::from_filename(".env.local");

        let mut config = Config::parse();
        config.sample_rate = 48000;
        config.initial_buffer_seconds = 15;

        if config.shortcut_resolution_delay_ms.is_none() {
            config.shortcut_resolution_delay_ms = Some(50);
        }

        if config.start_keys.is_empty() {
            config.start_keys = vec!["double(ControlLeft, 300)".to_string()];
        }

        if config.stop_keys.is_empty() {
            config.stop_keys = vec!["ControlLeft".to_string()];
        }

        if config.cancel_keys.is_empty() {
            config.cancel_keys = vec!["double(Escape, 300)".to_string()];
        }

        let replace_str = std::env::var("PARA_REPLACE").ok();
        config.transcription_replace_text = parse_replace_pairs(&replace_str);

        if config.model.is_none() {
            config.model = Some(get_default_model());
        }

        config.validate();
        config
    }

    pub fn new_for_test(
        start_keys: Vec<String>,
        stop_keys: Vec<String>,
        cancel_keys: Vec<String>,
        pause_keys: Vec<String>,
    ) -> Self {
        Config {
            debug: false,
            paste: false,
            realtime: false,
            spotify_recording_volume: None,
            spotify_reduce_by: None,
            start_keys,
            stop_keys,
            cancel_keys,
            pause_keys,
            transcribe_on_pause: false,
            shortcut_resolution_delay_ms: Some(50),
            memory_monitor: false,
            sample_rate: 48000,
            initial_buffer_seconds: 15,
            transcription_replace_text: HashMap::new(),
            model: Some(get_default_model()),
            force: false,
        }
    }

    pub fn initialize() -> Arc<Config> {
        let config = Arc::new(Config::new());
        CONFIG
            .set(config.clone())
            .unwrap_or_else(|_| {
                log::error!("Config already initialized - this should not happen");
            });
        CONFIG.get().unwrap().clone()
    }

    pub fn global() -> Arc<Config> {
        #[cfg(test)]
        {
            TEST_CONFIG.with(|test_config| {
                if let Ok(config_lock) = test_config.lock() {
                    if let Some(config) = config_lock.as_ref() {
                        return config.clone();
                    }
                }

                match CONFIG.get() {
                    Some(config) => config.clone(),
                    None => Arc::new(Config::new_for_test(
                        vec!["F1".to_string()],
                        vec!["F2".to_string()],
                        vec!["F3".to_string()],
                        vec!["F4".to_string()],
                    )),
                }
            })
        }

        #[cfg(not(test))]
        {
            CONFIG.get().cloned().unwrap_or_else(|| {
                log::error!("Config not initialized, using default");
                Arc::new(Config::new())
            })
        }
    }

    pub fn set_global_for_test(config: Arc<Config>) -> Result<(), Arc<Config>> {
        #[cfg(test)]
        {
            TEST_CONFIG.with(|test_config| {
                if let Ok(mut config_lock) = test_config.lock() {
                    *config_lock = Some(config);
                    Ok(())
                } else {
                    Err(config)
                }
            })
        }

        #[cfg(not(test))]
        {
            CONFIG.set(config)
        }
    }

    fn validate(&self) {
        if let Some(ref model) = self.model {
            if !is_model_supported(model) && !self.force {
                eprintln!("Error: Model '{}' is not in the list of supported models.", model);
                eprintln!("Supported models:");
                for model in AVAILABLE_MODELS {
                    eprintln!("  - {}", model);
                }
                eprintln!("\nTo use an unsupported model, add --force flag or set PARA_FORCE=true");
                std::process::exit(1);
            }
            
            if !is_model_supported(model) && self.force {
                log::warn!("Using unsupported model '{}' with --force flag", model);
            }
        }
    }

    pub fn transcription_replacements(&self) -> &HashMap<String, Option<String>> {
        &self.transcription_replace_text
    }

    pub fn model_name(&self) -> &str {
        self.model.as_ref().unwrap()
    }
}

impl ShortcutConfigProvider for Config {
    fn start_keys(&self) -> &[String] {
        &self.start_keys
    }

    fn stop_keys(&self) -> &[String] {
        &self.stop_keys
    }

    fn cancel_keys(&self) -> &[String] {
        &self.cancel_keys
    }

    fn pause_keys(&self) -> &[String] {
        &self.pause_keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_shortcut_keys() {
        let config = Config::new_for_test(vec![], vec![], vec![], vec![]);
        
        assert!(config.start_keys.is_empty());
        assert!(config.stop_keys.is_empty());
        assert!(config.cancel_keys.is_empty());
    }
    
    #[test]
    fn test_default_values_applied() {
        let mut config = Config {
            debug: false,
            paste: false,
            realtime: false,
            spotify_recording_volume: None,
            spotify_reduce_by: None,
            start_keys: vec![],
            stop_keys: vec![],
            cancel_keys: vec![],
            pause_keys: vec![],
            transcribe_on_pause: false,
            shortcut_resolution_delay_ms: None,
            memory_monitor: false,
            sample_rate: 48000,
            initial_buffer_seconds: 15,
            transcription_replace_text: HashMap::new(),
            model: Some(get_default_model()),
            force: false,
        };
        
        if config.start_keys.is_empty() {
            config.start_keys = vec!["double(ControlLeft, 300)".to_string()];
        }
        if config.stop_keys.is_empty() {
            config.stop_keys = vec!["ControlLeft".to_string()];
        }
        if config.cancel_keys.is_empty() {
            config.cancel_keys = vec!["double(Escape, 300)".to_string()];
        }
        
        assert_eq!(config.start_keys, vec!["double(ControlLeft, 300)"]);
        assert_eq!(config.stop_keys, vec!["ControlLeft"]);
        assert_eq!(config.cancel_keys, vec!["double(Escape, 300)"]);
    }
}
