use auto_deref::AutoDeref;
use chess::Color;

use crate::{client::components::settings, common::theme::ThemeType};

#[derive(AutoDeref)]
pub(super) struct Analyze(pub(super) bool);

#[derive(AutoDeref)]
pub(super) struct BoardSize(pub(super) u32);

#[derive(AutoDeref)]
pub(super) struct GameId(pub(super) Option<u32>);

#[derive(AutoDeref)]
pub(super) struct Perspective(pub(super) Color);

pub(super) struct Settings {
    pub(super) board_theme: String,
    pub(super) piece_theme: String,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            board_theme: settings::load_theme(ThemeType::Board),
            piece_theme: settings::load_theme(ThemeType::Piece),
        }
    }
}
