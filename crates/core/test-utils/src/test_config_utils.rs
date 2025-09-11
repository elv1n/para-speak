use config::Config;
use std::sync::Arc;

pub fn initialize_for_test(
    start_keys: Vec<String>,
    stop_keys: Vec<String>,
    cancel_keys: Vec<String>,
    pause_keys: Vec<String>,
) -> Arc<Config> {
    let config = Arc::new(Config::new_for_test(
        start_keys,
        stop_keys,
        cancel_keys,
        pause_keys,
    ));

    let _ = Config::set_global_for_test(config.clone());

    config
}
