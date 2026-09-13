use dioxus::prelude::*;

use views::{NotFound, Play};

mod components;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Play {},
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
        }
        main { Outlet::<Route> {} }
    }
}
