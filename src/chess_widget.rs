use crate::game::Game;
use crate::pieces::{Piece, Player, Position};
use druid::piet::{ImageFormat, InterpolationMode};
use druid::{
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Rect, RenderContext, Size, UpdateCtx, Widget,
};
use image::io::Reader as ImageReader;
use std::fs::read;

pub const WINDOW_SIZE: f64 = 800.0;
const BOARD_SIZE: usize = 816;
const PIECE_SIZE: usize = 102;

fn get_image(file_name: String) -> Vec<u8> {
    let bytes = read(file_name).unwrap();
    let img = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    img.into_bytes()
}

pub struct ChessWidget {
    game: Game,
}

impl ChessWidget {
    pub fn new() -> Self {
        Self { game: Game::new() }
    }

    fn get_image_file(&self, piece: Piece) -> String {
        let name = match piece {
            Piece::Rook(..) => "Rook",
            Piece::Bishop(..) => "Bishop",
            Piece::Pawn(..) => "Pawn",
            Piece::Knight(..) => "Knight",
            Piece::King(..) => "King",
            Piece::Queen(..) => "Queen",
        };
        let player = match piece.get_player() {
            Player::White => "white",
            Player::Black => "black",
        };
        "images/".to_owned() + player + name + ".png"
    }

    fn draw_background(&self, ctx: &mut PaintCtx) {
        let image_data = get_image("images/board.png".to_string());
        let image = ctx
            .make_image(BOARD_SIZE, BOARD_SIZE, &image_data, ImageFormat::Rgb)
            .unwrap();
        ctx.draw_image(
            &image,
            Rect::new(0.0, 0.0, WINDOW_SIZE, WINDOW_SIZE),
            InterpolationMode::Bilinear,
        );
    }

    fn draw_square(&self, ctx: &mut PaintCtx, position: Position) {
        if let Some(piece) = self.game.get_piece(position) {
            let image_data = get_image(self.get_image_file(piece));
            let image = ctx
                .make_image(
                    PIECE_SIZE,
                    PIECE_SIZE,
                    &image_data,
                    ImageFormat::RgbaSeparate,
                )
                .unwrap();
            let x0 = WINDOW_SIZE * position.x as f64 / 8.0;
            let y1 = WINDOW_SIZE * (1.0 - (position.y as f64) / 8.0);
            ctx.draw_image(
                &image,
                Rect::new(x0, y1 - WINDOW_SIZE / 8.0, x0 + WINDOW_SIZE / 8.0, y1),
                InterpolationMode::Bilinear,
            );
        }
    }
}

impl Widget<String> for ChessWidget {
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

        self.draw_background(ctx);

        for x in 0..8 {
            for y in 0..8 {
                self.draw_square(ctx, Position { x, y });
            }
        }
    }
}
