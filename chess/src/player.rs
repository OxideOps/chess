use crate::color::Color;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum PlayerKind {
    #[default]
    Local,
    Remote,
    Bot,
}

impl PlayerKind {
    pub fn is_local_game(white_player_kind: Self, black_player_kind: Self) -> bool {
        white_player_kind == Self::Local && black_player_kind == Self::Local
    }
}

#[derive(Default, PartialEq, Clone)]
pub struct Player {
    pub kind: PlayerKind,
    pub color: Color,
    pub name: String,
}

impl Player {
    pub fn with_color(color: Color) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }
}
