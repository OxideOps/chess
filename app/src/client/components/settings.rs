use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::theme::ThemeType;

const APP_NAME: &str = "oxide-chess";
const CONFIG_NAME: &str = "themes";

#[component]
pub(crate) fn Settings(
    cx: Scope,
    board_theme: UseState<String>,
    piece_theme: UseState<String>,
) -> Element {
    let board_theme_list = get_theme_future(cx, ThemeType::Board);
    let piece_theme_list = get_theme_future(cx, ThemeType::Piece);
    cx.render(rsx! {
        div {
            table {
                tr {
                    td { "Board theme: " }
                    td {
                        select {
                            class: "select",
                            onchange: |event| {
                                board_theme.set(event.value.clone());
                                #[cfg(feature = "desktop")]
                                save_theme_to_config(ThemeType::Board, &event.value);
                            },
                            for theme in board_theme_list.value().into_iter().flatten() {
                                option {
                                    value: "{theme}",
                                    selected: **board_theme == *theme,
                                    "{theme}"
                                }
                            }
                        }
                    }
                }
                tr {
                    td { "Piece theme: " }
                    td {
                        select {
                            class: "select",
                            onchange: |event| {
                                piece_theme.set(event.value.clone());
                                #[cfg(feature = "desktop")]
                                save_theme_to_config(ThemeType::Piece, &event.value);
                            },
                            for theme in piece_theme_list.value().into_iter().flatten() {
                                option {
                                    value: "{theme}",
                                    selected: **piece_theme == *theme,
                                    "{theme}"
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
#[derive(Serialize, Deserialize)]
struct ThemeConfig {
    board_theme: String,
    piece_theme: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            board_theme: "qootee".into(),
            piece_theme: "merida".into(),
        }
    }
}

pub fn load_theme_from_config(theme_type: ThemeType) -> String {
    let cfg: ThemeConfig = confy::load(APP_NAME, CONFIG_NAME).unwrap_or_default();

    match theme_type {
        ThemeType::Board => cfg.board_theme,
        ThemeType::Piece => cfg.piece_theme,
    }
}

#[cfg(feature = "desktop")]
fn save_theme_to_config(theme_type: ThemeType, theme_value: &str) {
    let mut cfg: ThemeConfig = confy::load(APP_NAME, CONFIG_NAME).unwrap_or_default();

    match theme_type {
        ThemeType::Board => cfg.board_theme = theme_value.to_string(),
        ThemeType::Piece => cfg.piece_theme = theme_value.to_string(),
    }

    if let Err(e) = confy::store(APP_NAME, CONFIG_NAME, cfg) {
        log::error!("could not store theme: {e}")
    }
}

fn get_theme_future(cx: &ScopeState, theme_type: ThemeType) -> &UseFuture<Vec<String>> {
    #[cfg(feature = "desktop")]
    use crate::common::theme::get_themes;
    #[cfg(feature = "web")]
    use crate::server::server_functions::get_themes;

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
