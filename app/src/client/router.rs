use dioxus::prelude::*;
use dioxus_router::prelude::*;

use super::components::{nav_bar::*, Widget};

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub(crate) enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[route("/game")]
        Widget {},
        #[route("/puzzles")]
        Puzzles {},
        #[route("/about")]
        About {},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}
