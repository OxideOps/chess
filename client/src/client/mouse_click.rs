use dioxus::{
    html::{geometry::ClientPoint, input_data::MouseButtonSet},
    prelude::*,
};

pub(super) struct MouseClick {
    pub(super) point: ClientPoint,
    pub(super) kind: MouseButtonSet,
}

impl From<Event<MouseData>> for MouseClick {
    fn from(event: Event<MouseData>) -> Self {
        Self {
            point: event.client_coordinates(),
            kind: event.held_buttons(),
        }
    }
}
