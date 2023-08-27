use crate::arrows::ArrowData;
use crate::board::get_center;
use dioxus::html::geometry::ClientPoint;
use dioxus::prelude::*;
use std::f64::consts::PI;

// the following are measured relative to the board size
const HEAD: f64 = 1.0 / 30.0; // size of arrow head
const WIDTH: f64 = 1.0 / 80.0; // width of arrow body
const OFFSET: f64 = 1.0 / 20.0; // how far away from the middle of the starting square

#[derive(Props, PartialEq)]
pub struct ArrowProps {
    show: bool,
    data: ArrowData,
    board_size: u32,
}

fn get_color(alpha: f64) -> String {
    format!("rgba(27, 135, 185, {})", alpha)
}

fn get_angle_from_vertical(from: &ClientPoint, to: &ClientPoint) -> f64 {
    (to.y - from.y).atan2(to.x - from.x) + PI / 2.0
}

pub fn Arrow(cx: Scope<ArrowProps>) -> Element {
    if !cx.props.show {
        return None;
    }

    let from = get_center(&cx.props.data.mv.from, cx.props.board_size);
    let to = get_center(&cx.props.data.mv.to, cx.props.board_size);

    let h = HEAD * board_size;
    let w = WIDTH * board_size;
    let o = OFFSET * board_size;

    let angle = get_angle_from_vertical(from, to);
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
                fill: "{get_color(cx.props.data.alpha)}"
            }
        }
    })
}
