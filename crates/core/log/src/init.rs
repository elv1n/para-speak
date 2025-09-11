use std::sync::Once;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static LOGGER_INIT: Once = Once::new();

pub fn init(debug: bool) {
    LOGGER_INIT.call_once(|| {
        let base_level = if debug { "debug" } else { "info" };
        let filter_directive = format!("{},enigo=warn", base_level);

        let _ = tracing_log::LogTracer::init();

        let subscriber = tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .without_time()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false)
                    .with_level(true)
                    .compact(),
            )
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&filter_directive)),
            );

        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}
