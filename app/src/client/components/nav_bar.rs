use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::client::router::Route;

#[component]
pub(crate) fn NavBar(cx: Scope) -> Element {
    render! {
        nav {
            div {
                class: "nav-bar",
                ul {
                    class: "flex",
                    li {
                        Link {
                            class: "nav-link",
                            to: Route::Home {},
                            "Home"
                        }
                    }
                    li {
                        Link {
                            class: "nav-link",
                            to: Route::Widget {},
                            "Game"
                        }
                    }
                    li {
                        Link {
                            class: "nav-link",
                            to: Route::Puzzles {},
                            "Puzzles"
                        }
                    }
                    li {
                        Link {
                            class: "nav-link",
                            to: Route::About {},
                            "About"
                        }
                    }
                }
            }
        }
        Outlet::<Route> {}
    }
}

#[component]
pub(crate) fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Home Page" }
    }
}

#[component]
pub(crate) fn About(cx: Scope) -> Element {
    render! {
        h1 { "About Page" }
    }
}

#[component]
pub(crate) fn Puzzles(cx: Scope) -> Element {
    render! {
        h1 { "Puzzles Page" }
    }
}

#[component]
pub(crate) fn PageNotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        h1 { "Page not found" }
        p { "The page you requested doesn't exist: {route:?}." }
    }
}
