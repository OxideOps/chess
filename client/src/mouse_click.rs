use dioxus::{
    html::{geometry::ElementPoint, input_data::MouseButtonSet},
    prelude::*,
};

pub(super) struct MouseClick {
    pub(super) point: ElementPoint,
    pub(super) kind: MouseButtonSet,
}

impl From<Event<MouseData>> for MouseClick {
    fn from(event: Event<MouseData>) -> Self {
        Self {
            point: event.element_coordinates(),
            kind: event.held_buttons(),
        }
    }
}
