use dioxus::prelude::*;
use dioxus_router::prelude::*;

use super::router::Route;

#[component]
pub(super) fn NavBar(cx: Scope) -> Element {
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
