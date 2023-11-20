use auto_deref::AutoDeref;
use chess::Color;

#[derive(AutoDeref)]
pub(super) struct BoardSize(pub(super) u32);

#[derive(AutoDeref)]
pub(super) struct GameId(pub(super) Option<u32>);

#[derive(AutoDeref)]
pub(super) struct Perspective(pub(super) Color);
