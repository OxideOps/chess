use dioxus::prelude::*;

use views::{Analysis, NotFound, Play};

mod components;
mod engine;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Play {},
        #[route("/analysis")]
        Analysis {},
    #[end_layout]
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Navbar() -> Element {
    rsx! {
        nav { id: "navbar",
            span { class: "brand", "Chess" }
            Link { to: Route::Play {}, "Play" }
            Link { to: Route::Analysis {}, "Analysis" }
        }
        main { Outlet::<Route> {} }
    }
}
