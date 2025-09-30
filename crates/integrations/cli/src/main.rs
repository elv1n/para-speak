use config::Config;
use ort::environment::get_environment;
use ort::logging::LogLevel;
use para_speak_cli::{model_verification::ModelVerification, panic_handler, ParaSpeakApp};

fn main() {
    let _ = ort::init()
        .commit()
        .and_then(|_| get_environment().map(|env| env.set_log_level(LogLevel::Fatal)));

    let config = Config::initialize();

    panic_handler::setup_full_backtrace_for_dev();
    panic_handler::install_panic_handler();

    log::init(config.debug);

    log::info!("Configuration: {:?}", config);

    let verifier = match ModelVerification::new(config.model_name().to_string()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("\n❌ Failed to initialize model verification: {}\n", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = verifier.verify_or_download() {
        eprintln!("\n❌ Model verification/download failed: {}\n", e);
        std::process::exit(1);
    }

    let app = ParaSpeakApp::new();
    if let Err(e) = app.run() {
        let error_chain = e
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(": ");

        eprintln!("\n❌ Error: {}\n", error_chain);
        if config.debug {
            eprintln!("Debug backtrace:\n{:?}", e);
        }
        std::process::exit(1);
    }
}
