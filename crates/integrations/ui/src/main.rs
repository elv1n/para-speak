use ui::{create_overlay_options, OverlayApp};

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = create_overlay_options();

    eframe::run_native(
        "Para-Speak Overlay",
        options,
        Box::new(|_cc| Ok(Box::new(OverlayApp::default()))),
    )
}
