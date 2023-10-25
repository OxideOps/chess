use auto_deref::AutoDeref;
use chess::color::Color;

#[derive(AutoDeref)]
pub(super) struct GameId(pub(super) Option<u32>);
#[derive(AutoDeref, Copy, Clone)]
pub(super) struct Perspective(pub(super) Color);
#[derive(AutoDeref, Copy, Clone)]
pub(super) struct Analyze(pub(super) bool);
