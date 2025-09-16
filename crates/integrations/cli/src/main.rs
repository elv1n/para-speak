use config::Config;
use para_speak_cli::{panic_handler, ParaSpeakApp};
use ml_utils::{verify_python_requirements, verify_model_files};

fn main() {
    let config = Config::initialize();

    panic_handler::setup_full_backtrace_for_dev();
    panic_handler::install_panic_handler();

    log::init(config.debug);

    log::info!("Configuration: {:?}", config);

    if let Err(e) = verify_python_requirements(config.model_name()) {
        eprintln!("\n❌ Python requirements verification failed\n");
        eprintln!("{}", e);
        eprintln!("\nPlease run the following command to fix this:");
        eprintln!("  cargo run -p verify-cli\n");
        eprintln!("This will update your Python environment to match the selected model.");
        std::process::exit(1);
    }

    if let Err(e) = verify_model_files(config.model_name()) {
        eprintln!("\n❌ Model verification failed\n");
        eprintln!("{}", e);
        eprintln!("\nPlease run the following command to download/fix the model:");
        eprintln!("  cargo run -p verify-cli\n");
        eprintln!("This will download or repair the required model files.");
        std::process::exit(1);
    }

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
