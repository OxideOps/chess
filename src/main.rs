use chess::app::App;

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(App);
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(App);
}
