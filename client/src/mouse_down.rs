use dioxus::html::geometry::ClientPoint;
use dioxus::html::input_data::MouseButtonSet;
use dioxus::prelude::*;

pub struct MouseDown {
    pub point: ClientPoint,
    pub kind: MouseButtonSet,
}

impl From<Event<MouseData>> for MouseDown {
    fn from(event: Event<MouseData>) -> Self {
        Self {
            point: event.client_coordinates(),
            kind: event.held_buttons(),
        }
    }
}
