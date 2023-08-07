use dioxus::html::geometry::ClientPoint;
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq)]
pub struct Ray {
    pub from: ClientPoint,
    pub to: ClientPoint,
}

impl Ray {
    pub fn get_angle_from_vertical(&self) -> f64 {
        (self.to.y - self.from.y).atan2(self.to.x - self.from.x) + PI / 2.0
    }

    pub fn len(&self) -> f64 {
        ((self.to.y - self.from.y).powf(2.0) + (self.to.x - self.from.x).powf(2.0)).sqrt()
    }
}
