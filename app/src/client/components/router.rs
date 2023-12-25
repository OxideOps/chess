use dioxus::prelude::*;
use dioxus_router::prelude::*;

use super::{nav_bar::NavBar, Widget};

#[component]
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Home Page" }
    }
}

#[component]
fn About(cx: Scope) -> Element {
    render! {
        h1 { "About Page" }
    }
}

#[component]
fn PageNotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        h1 { "Page not found" }
        p { "The page you requested doesn't exist: {route:?}." }
    }
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub(crate) enum Route {
    // All routes under the NavBar layout will be rendered inside of the NavBar Outlet
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[route("/game")]
        Widget{},
        #[route("/about")]
        About{},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}
