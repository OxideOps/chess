use crate::game::Game;
use crate::pieces::{Piece, Player, Position};
use druid::piet::{ImageFormat, InterpolationMode};
use druid::{
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, RenderContext, Size, UpdateCtx, Widget,
};
use image::io::Reader as ImageReader;
use std::fs::read;

pub const WINDOW_SIZE: f64 = 800.0;
const BOARD_SIZE: usize = 816;
const PIECE_SIZE: usize = 102;

fn get_image(file_name: &str) -> Vec<u8> {
    let bytes = read(file_name).unwrap();
    let img = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    img.into_bytes()
}

// convert from druid's 'Point' to our 'Position'
impl From<Point> for Position {
    fn from(point: Point) -> Position {
        Position {
            x: (8.0 * point.x / WINDOW_SIZE).floor() as usize,
            y: (8.0 * (1.0 - point.y / WINDOW_SIZE)).floor() as usize,
        }
    }
}

// convert from our 'Position' to druid's 'Point'
impl From<Position> for Point {
    fn from(position: Position) -> Point {
        Point {
            x: WINDOW_SIZE * position.x as f64 / 8.0,
            y: WINDOW_SIZE * (7.0 - position.y as f64) / 8.0,
        }
    }
}

pub struct ChessWidget {
    game: Game,
    board_image: Vec<u8>,
    piece_images: [Vec<u8>; 12],
    mouse_down: Option<Point>,
    current_point: Point,
}

impl ChessWidget {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            board_image: get_image("images/board.png"),
            piece_images: Self::get_image_files(),
            mouse_down: None,
            current_point: Point { x: 0.0, y: 0.0 },
        }
    }

    fn get_image_files() -> [Vec<u8>; 12] {
        [
            get_image("images/whiteRook.png"),
            get_image("images/whiteBishop.png"),
            get_image("images/whitePawn.png"),
            get_image("images/whiteKnight.png"),
            get_image("images/whiteKing.png"),
            get_image("images/whiteQueen.png"),
            get_image("images/blackRook.png"),
            get_image("images/blackBishop.png"),
            get_image("images/blackPawn.png"),
            get_image("images/blackKnight.png"),
            get_image("images/blackKing.png"),
            get_image("images/blackQueen.png"),
        ]
    }

    fn get_image_file(&self, piece: Piece) -> &Vec<u8> {
        match piece {
            Piece::Rook(Player::White) => &self.piece_images[0],
            Piece::Bishop(Player::White) => &self.piece_images[1],
            Piece::Pawn(Player::White) => &self.piece_images[2],
            Piece::Knight(Player::White) => &self.piece_images[3],
            Piece::King(Player::White) => &self.piece_images[4],
            Piece::Queen(Player::White) => &self.piece_images[5],
            Piece::Rook(Player::Black) => &self.piece_images[6],
            Piece::Bishop(Player::Black) => &self.piece_images[7],
            Piece::Pawn(Player::Black) => &self.piece_images[8],
            Piece::Knight(Player::Black) => &self.piece_images[9],
            Piece::King(Player::Black) => &self.piece_images[10],
            Piece::Queen(Player::Black) => &self.piece_images[11],
        }
    }

    fn draw_background(&self, ctx: &mut PaintCtx) {
        let image = ctx
            .make_image(BOARD_SIZE, BOARD_SIZE, &self.board_image, ImageFormat::Rgb)
            .unwrap();
        ctx.draw_image(
            &image,
            Rect::new(0.0, 0.0, WINDOW_SIZE, WINDOW_SIZE),
            InterpolationMode::Bilinear,
        );
    }

    fn draw_square(&self, ctx: &mut PaintCtx, position: Position) {
        if let Some(piece) = self.game.get_piece(position) {
            let image = ctx
                .make_image(
                    PIECE_SIZE,
                    PIECE_SIZE,
                    self.get_image_file(piece),
                    ImageFormat::RgbaSeparate,
                )
                .unwrap();
            let mut p0 = Point::from(position);
            // if we are holding a piece, offset it's position by how far it's been dragged
            if let Some(mouse_down) = self.mouse_down {
                if position == Position::from(mouse_down) {
                    p0.x += self.current_point.x - mouse_down.x;
                    p0.y += self.current_point.y - mouse_down.y;
                }
            }
            ctx.draw_image(
                &image,
                Rect::new(
                    p0.x,
                    p0.y,
                    p0.x + WINDOW_SIZE / 8.0,
                    p0.y + WINDOW_SIZE / 8.0,
                ),
                InterpolationMode::Bilinear,
            );
        }
    }

    // We want the square a dragged piece is considered to be on to be based on the center of
    // the piece, not the location of the mouse. This requires offsetting based on the original
    // mouse down location
    fn get_dragged_piece_position(&self, mouse_up: Point) -> Position {
        let mouse_down = self.mouse_down.unwrap();
        let top_left = Point::from(Position::from(mouse_down));
        let middle = Point {
            x: top_left.x + WINDOW_SIZE / 16.0,
            y: top_left.y + WINDOW_SIZE / 16.0,
        };
        let x_offset = mouse_down.x - middle.x;
        let y_offset = mouse_down.y - middle.y;
        Position::from(Point {
            x: mouse_up.x - x_offset,
            y: mouse_up.y - y_offset,
        })
    }
}

impl Widget<String> for ChessWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut String, _env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                self.mouse_down = Some(mouse_event.pos);
            }
            Event::MouseUp(mouse_event) => {
                if let Some(mouse_down) = self.mouse_down {
                    let from = Position::from(mouse_down);
                    let to = self.get_dragged_piece_position(mouse_event.pos);
                    self.game
                        .move_piece(from, to)
                        .map_err(|chess_error| {
                            println!("{:?}", chess_error);
                        })
                        .ok();
                }
                self.mouse_down = None;
            }
            Event::MouseMove(mouse_event) => {
                self.current_point = mouse_event.pos;
                // if we are currently holding onto a piece, request a redraw
                if let Some(mouse_down) = self.mouse_down {
                    if self.game.get_piece(Position::from(mouse_down)).is_some() {
                        ctx.request_paint();
                    }
                }
            }
            _ => {}
        }
    }

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
