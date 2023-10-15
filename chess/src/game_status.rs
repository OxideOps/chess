use crate::color::Color;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(super) enum GameStatus {
    #[default]
    NotStarted,
    Ongoing,
    Check(Color),
    Checkmate(Color),
    Timeout(Color),
    Draw(DrawKind),
}

impl GameStatus {
    pub(super) fn update(&mut self, status: GameStatus) {
        if *self != status {
            log::info!("GameStatus changing from {:?} to {:?}", *self, status);
            *self = status
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum DrawKind {
    FiftyMoveRule,
    Repetition,
    Stalemate,
}
