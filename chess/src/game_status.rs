#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
    Replay,
    Draw(DrawKind),
}

impl GameStatus {
    pub fn update(&mut self, status: GameStatus) {
        if *self != status {
            log::info!("GameStatus changing from {:?} to {:?}", *self, status);
            *self = status
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DrawKind {
    Stalemate,
    FiftyMoveRule,
}
