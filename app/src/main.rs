pub fn main() {
    dioxus_logger::init(app::args::get_log_level()).expect("Failed to initialize dioxus logger");
    app::launch();
}
