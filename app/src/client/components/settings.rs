use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::client::storage;
use crate::{client::shared_states, common::theme::ThemeType};

#[cfg(feature = "desktop")]
const APP_NAME: &str = "oxide-chess";
#[cfg(feature = "desktop")]
const CONFIG_NAME: &str = "themes";

#[component]
pub(crate) fn Settings(cx: Scope) -> Element {
    let settings = use_shared_state::<shared_states::Settings>(cx)?;
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
                                settings.write().board_theme = event.value.clone();
                                #[cfg(feature = "desktop")]
                                save_theme_to_config(ThemeType::Board, &event.value);
                                #[cfg(feature = "web")]
                                storage::set_item(&ThemeType::Board.to_string(), &event.value);
                            },
                            for theme in board_theme_list.value().into_iter().flatten() {
                                option {
                                    value: "{theme}",
                                    selected: settings.read().board_theme == *theme,
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
                                settings.write().piece_theme = event.value.clone();
                                #[cfg(feature = "desktop")]
                                save_theme_to_config(ThemeType::Piece, &event.value);
                                #[cfg(feature = "web")]
                                storage::set_item(&ThemeType::Piece.to_string(), &event.value);
                            },
                            for theme in piece_theme_list.value().into_iter().flatten() {
                                option {
                                    value: "{theme}",
                                    selected: settings.read().piece_theme == *theme,
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

#[cfg(feature = "desktop")]
pub fn load_theme(theme_type: ThemeType) -> String {
    let cfg: ThemeConfig = confy::load(APP_NAME, CONFIG_NAME).unwrap_or_default();

    match theme_type {
        ThemeType::Board => cfg.board_theme,
        ThemeType::Piece => cfg.piece_theme,
    }
}

#[cfg(feature = "web")]
pub fn load_theme(theme_type: ThemeType) -> String {
    storage::get_item(&theme_type.to_string()).unwrap_or_else(|| theme_type.default_theme())
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
