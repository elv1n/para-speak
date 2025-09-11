use config::Config;
use para_speak_cli::{panic_handler, ParaSpeakApp};

fn main() {
    let config = Config::initialize();

    panic_handler::setup_full_backtrace_for_dev();
    panic_handler::install_panic_handler();

    log::init(config.debug);

    log::info!("Configuration: {:?}", config);

    let app = ParaSpeakApp::new();
    if let Err(e) = app.run() {
        let error_chain = e
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(": ");

        if error_chain.contains("Model not found") {
            eprintln!("\n❌ Model not found\n");
            eprintln!("Please download the model first by running:");
            eprintln!("  cargo run -p verify-cli\n");
            std::process::exit(1);
        } else {
            eprintln!("\n❌ Error: {}\n", error_chain);
            if config.debug {
                eprintln!("Debug backtrace:\n{:?}", e);
            }
            std::process::exit(1);
        }
    }
}
