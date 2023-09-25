use auto_deref::AutoDeref;

#[derive(AutoDeref)]
pub(super) struct GameId(pub(super) Option<u32>);

