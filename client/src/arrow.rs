use crate::ray::Ray;
use dioxus::prelude::*;

const COLOR: &str = "rgba(12, 60, 82, 0.9)";
// the following are measured relative to the board size
const HEAD: f64 = 1.0 / 30.0; // size of arrow head
const WIDTH: f64 = 1.0 / 80.0; // width of arrow body
const OFFSET: f64 = 1.0 / 20.0; // how far away from the middle of the starting square

#[derive(Props, PartialEq)]
pub struct ArrowProps {
    ray: Ray,
    board_size: u32,
}

pub fn Arrow(cx: Scope<ArrowProps>) -> Element {
    let h = HEAD * cx.props.board_size as f64;
    let w = WIDTH * cx.props.board_size as f64;
    let o = OFFSET * cx.props.board_size as f64;

    let to = cx.props.ray.to;
    let from = cx.props.ray.from;

    let angle = cx.props.ray.get_angle_from_vertical();
    let sin = angle.sin();
    let cos = angle.cos();

    let x0 = to.x as u32;
    let y0 = to.y as u32;

    let x1 = (to.x + h * cos - h * sin) as u32;
    let y1 = (to.y + h * sin + h * cos) as u32;

    let x2 = (to.x + w * cos - h * sin) as u32;
    let y2 = (to.y + w * sin + h * cos) as u32;

    let x3 = (from.x + w * cos + o * sin) as u32;
    let y3 = (from.y + w * sin - o * cos) as u32;

    let x4 = (from.x - w * cos + o * sin) as u32;
    let y4 = (from.y - w * sin - o * cos) as u32;

    let x5 = (to.x - w * cos - h * sin) as u32;
    let y5 = (to.y - w * sin + h * cos) as u32;

    let x6 = (to.x - h * cos - h * sin) as u32;
    let y6 = (to.y - h * sin + h * cos) as u32;

    cx.render(rsx! {
        svg {
            class: "absolute pointer-events-none",
            style: "z-index: 3",
            height: "{cx.props.board_size}",
            width: "{cx.props.board_size}",
            polygon {
                class: "absolute pointer-events-none",
                points: "{x0},{y0}, {x1},{y1} {x2},{y2} {x3},{y3} {x4},{y4} {x5},{y5} {x6},{y6}",
                fill: "{COLOR}"
            }
        }
    })
}
