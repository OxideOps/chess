use auto_deref::AutoDeref;

#[derive(AutoDeref)]
pub struct GameId(pub Option<u32>);
