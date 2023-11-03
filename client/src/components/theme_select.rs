use common::theme::ThemeType;
use dioxus::prelude::*;

#[component]
pub(crate) fn ThemeSelect(
    cx: Scope,
    board_theme: UseState<String>,
    piece_theme: UseState<String>,
) -> Element {
    let board_theme_list = get_theme_future(cx, ThemeType::Board);
    let piece_theme_list = get_theme_future(cx, ThemeType::Piece);

    cx.render(rsx! {
        table {
            tr {
                td {
                    label { "Board theme: " }
                    select {
                        class: "select",
                        onchange: |event| board_theme.set(event.value.clone()),
                        board_theme_list.value().into_iter().flatten().map(|theme|
                            rsx! { option { value: "{theme}", "{theme}" } }
                        )
                    }
                }
            }
            tr {
                td {
                    label { "Piece theme: " }
                    select {
                        class: "select",
                        onchange: |event| piece_theme.set(event.value.clone()),
                        piece_theme_list.value().into_iter().flatten().map(|theme|
                            rsx! { option { value: "{theme}", "{theme}" } }
                        )
                    }
                }
            }
        }
    })
}

fn get_theme_future(cx: &ScopeState, theme_type: ThemeType) -> &UseFuture<Vec<String>> {
    #[cfg(not(target_arch = "wasm32"))]
    use common::theme::get_themes;
    #[cfg(target_arch = "wasm32")]
    use server_functions::get_themes;

    use_future(cx, (), |_| async {
        match get_themes(theme_type).await {
            Ok(themes) => themes,
            Err(e) => {
                log::error!("Failed to get themes: {:?}", e);
                Vec::new()
            }
        }
    })
}
