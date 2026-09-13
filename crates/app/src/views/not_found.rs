use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        section { class: "not-found",
            h1 { "Page not found" }
            p { "There is nothing at /{segments.join(\"/\")}." }
        }
    }
}
