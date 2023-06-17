use chess::board;

// #![windows_subsystem = "windows"]

use druid::piet::{ImageFormat, InterpolationMode};
use druid::widget::prelude::*;
use druid::{AppLauncher, Color, LocalizedString, Rect, WindowDesc};

struct CustomWidget;

const WINDOW_SIZE: f64 = 800.0;
const BOARD_SIZE: usize = 816;

impl Widget<String> for CustomWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut String, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &String,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &String, _data: &String, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &String,
        _env: &Env,
    ) -> Size {
        if bc.is_width_bounded() | bc.is_height_bounded() {
            let size = Size::new(WINDOW_SIZE, WINDOW_SIZE);
            bc.constrain(size)
        } else {
            bc.max()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &String, _env: &Env) {
        let size = ctx.size();
        let rect = size.to_rect();
        ctx.fill(rect, &Color::WHITE);

        let image_data = get_image();
        let image = ctx
            .make_image(BOARD_SIZE, BOARD_SIZE, &image_data, ImageFormat::Rgb)
            .unwrap();
        // The image is automatically scaled to fit the rect you pass to draw_image
        ctx.draw_image(
            &image,
            Rect::new(0.0, 0.0, WINDOW_SIZE, WINDOW_SIZE),
            InterpolationMode::Bilinear,
        );
    }
}

pub fn main() {
    let window = WindowDesc::new(|| CustomWidget {})
        .title(LocalizedString::new("Chess"))
        .window_size((WINDOW_SIZE, WINDOW_SIZE));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch("Druid + Piet".to_string())
        .expect("launch failed");
}

fn get_image() -> Vec<u8> {
    let bytes = include_bytes!("board.png");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap();
    img.into_bytes()
}
