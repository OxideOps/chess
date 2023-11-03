use dioxus::prelude::*;

#[component]
pub(crate) fn ThemeSelect(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            div {
                label { "Board theme: " }
                select {
                    class: "select",
                    onchange: |event| board_theme.set(event.value.clone()),
                    board_theme_list.value().into_iter().flat_map(|themes| themes.iter()).map(|theme|
                        rsx! { option { value: "{theme}", "{theme}" } }
                    )
                }
            }
            div {
                label { "Piece theme: " }
                select {
                    class: "select",
                    onchange: |event| piece_theme.set(event.value.clone()),
                    piece_theme_list.value().into_iter().flat_map(|themes| themes.iter()).map(|theme|
                        rsx! { option { value: "{theme}", "{theme}" } }
                    )
                }
            }
        }
    })
}
