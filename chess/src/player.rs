use crate::pieces::Color;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum PlayerKind {
    #[default]
    Local,
    Remote,
    Bot,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Player<'cx> {
    pub kind: PlayerKind,
    pub time: Option<u32>,
    pub color: Color,
    pub name: &'cx str,
}

impl<'cx> Player<'cx> {
    pub fn with_color(color: Color) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }
}
