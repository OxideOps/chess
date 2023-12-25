use dioxus::prelude::*;
use dioxus_router::prelude::*;

use super::router::Route;

#[component]
pub(super) fn NavBar(cx: Scope) -> Element {
    render! {
        nav {
            ul {
                li { Link { to: Route::Home {}, "Home" } }
                li { Link { to: Route::Widget {}, "Game" } }
            }
        }
        Outlet::<Route> {}
    }
}
