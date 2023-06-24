use crate::game::Game;
use crate::pieces::{Piece, Player, Position};
use druid::{
    keyboard_types::Key,
    piet::{ImageFormat, InterpolationMode, PietImage},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point,
    Rect, RenderContext, Size, UpdateCtx, Widget,
};
use image::io::Reader as ImageReader;
use std::fs::read;
use std::sync::Mutex;

pub const WINDOW_SIZE: f64 = 800.0;
const BOARD_SIZE: usize = 816;
const PIECE_SIZE: usize = 102;
const BOARD_FILE: &str = "images/board.png";
const IMAGE_FILES: [&str; 12] = [
    "images/whiteRook.png",
    "images/whiteBishop.png",
    "images/whitePawn.png",
    "images/whiteKnight.png",
    "images/whiteKing.png",
    "images/whiteQueen.png",
    "images/blackRook.png",
    "images/blackBishop.png",
    "images/blackPawn.png",
    "images/blackKnight.png",
    "images/blackKing.png",
    "images/blackQueen.png",
];

static WIDTH: Mutex<f64> = Mutex::new(WINDOW_SIZE);
static HEIGHT: Mutex<f64> = Mutex::new(WINDOW_SIZE);

fn get_window_width() -> f64 {
    *WIDTH.lock().unwrap()
}

fn get_window_height() -> f64 {
    *HEIGHT.lock().unwrap()
}

fn set_window_width(width: f64) {
    *WIDTH.lock().unwrap() = width;
}

fn set_window_height(height: f64) {
    *HEIGHT.lock().unwrap() = height;
}

fn create_image(file_name: &str, ctx: &mut PaintCtx, size: usize, fmt: ImageFormat) -> PietImage {
    let bytes = read(file_name).unwrap();
    let img = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .into_bytes();
    ctx.make_image(size, size, &img, fmt).unwrap()
}

// convert from druid's 'Point' to our 'Position'
impl From<Point> for Position {
    fn from(point: Point) -> Position {
        Position {
            x: (8.0 * point.x / get_window_width()).floor() as usize,
            y: (8.0 * (1.0 - point.y / get_window_height())).floor() as usize,
        }
    }
}

// convert from our 'Position' to druid's 'Point'
impl From<Position> for Point {
    fn from(position: Position) -> Point {
        Point {
            x: get_window_width() * position.x as f64 / 8.0,
            y: get_window_height() * (7.0 - position.y as f64) / 8.0,
        }
    }
}

pub struct ChessWidget {
    game: Game,
    board_image: Option<PietImage>,
    piece_images: Option<[PietImage; 12]>,
    mouse_down: Option<Point>,
    current_point: Point,
}

impl ChessWidget {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            board_image: None,
            piece_images: None,
            mouse_down: None,
            current_point: Default::default(),
        }
    }

    fn get_image_files(ctx: &mut PaintCtx) -> [PietImage; 12] {
        IMAGE_FILES
            .map(|file_name| create_image(file_name, ctx, PIECE_SIZE, ImageFormat::RgbaSeparate))
    }

    fn get_image_file(&self, piece: Piece) -> &PietImage {
        let index = match piece {
            Piece::Rook(Player::White) => 0,
            Piece::Bishop(Player::White) => 1,
            Piece::Pawn(Player::White) => 2,
            Piece::Knight(Player::White) => 3,
            Piece::King(Player::White) => 4,
            Piece::Queen(Player::White) => 5,
            Piece::Rook(Player::Black) => 6,
            Piece::Bishop(Player::Black) => 7,
            Piece::Pawn(Player::Black) => 8,
            Piece::Knight(Player::Black) => 9,
            Piece::King(Player::Black) => 10,
            Piece::Queen(Player::Black) => 11,
        };
        &self.piece_images.as_ref().unwrap()[index]
    }

    fn draw_background(&self, ctx: &mut PaintCtx) {
        ctx.draw_image(
            &self.board_image.as_ref().unwrap(),
            Rect::new(0.0, 0.0, get_window_width(), get_window_height()),
            InterpolationMode::Bilinear,
        );
    }

    fn draw_square(&self, ctx: &mut PaintCtx, position: Position) {
        if let Some(piece) = self.game.get_piece(&position) {
            let mut p0 = Point::from(position);
            // if we are holding a piece, offset it's position by how far it's been dragged
            if let Some(mouse_down) = self.mouse_down {
                if position == Position::from(mouse_down) {
                    p0.x += self.current_point.x - mouse_down.x;
                    p0.y += self.current_point.y - mouse_down.y;
                }
            }
            ctx.draw_image(
                &self.get_image_file(piece),
                Rect::new(
                    p0.x,
                    p0.y,
                    p0.x + get_window_width() / 8.0,
                    p0.y + get_window_height() / 8.0,
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
            x: top_left.x + get_window_width() / 16.0,
            y: top_left.y + get_window_height() / 16.0,
        };
        let x_offset = mouse_down.x - middle.x;
        let y_offset = mouse_down.y - middle.y;
        Position::from(Point {
            x: mouse_up.x - x_offset,
            y: mouse_up.y - y_offset,
        })
    }

    fn get_dragged_piece_start_position(&self) -> Option<Position> {
        Some(Position::from(self.mouse_down?))
    }

    fn create_images(&mut self, ctx: &mut PaintCtx) {
        self.board_image = Some(create_image(BOARD_FILE, ctx, BOARD_SIZE, ImageFormat::Rgb));
        self.piece_images = Some(Self::get_image_files(ctx));
    }
}

impl Widget<String> for ChessWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut String, _env: &Env) {
        match event {
            //Let this widget receive keyboard Events
            Event::WindowConnected => {
                #[cfg(windows)]
                {
                    let insets = ctx.window().content_insets();
                    set_window_width(WINDOW_SIZE - insets.x0 - insets.x1);
                    set_window_height(WINDOW_SIZE - insets.y0 - insets.y1);
                    ctx.request_paint();
                }
                ctx.request_focus();
            }
            Event::MouseDown(mouse_event) => {
                self.mouse_down = Some(mouse_event.pos);
            }
            Event::MouseUp(mouse_event) => {
                if let Some(mouse_down) = self.mouse_down {
                    let from = Position::from(mouse_down);
                    let to = self.get_dragged_piece_position(mouse_event.pos);
                    self.game
                        .move_piece(from, to)
                        .ok();
                    self.mouse_down = None;
                    ctx.request_paint();
                }
            }
            Event::MouseMove(mouse_event) => {
                self.current_point = mouse_event.pos;
                // if we are currently holding onto a piece, request a redraw
                if let Some(mouse_down) = self.mouse_down {
                    if self.game.get_piece(&Position::from(mouse_down)).is_some() {
                        ctx.request_paint();
                    }
                }
            }
            Event::KeyDown(key_event) => match key_event.key {
                Key::ArrowLeft => {
                    println!("left arrow pressed down")
                }
                Key::ArrowRight => {
                    println!("right arrow pressed down")
                }
                Key::Character(ref c) => {
                    println!("'{}' pressed down", c.to_uppercase())
                }
                _ => {}
            },
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
            let size = Size::new(get_window_width(), get_window_height());
            bc.constrain(size)
        } else {
            bc.max()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &String, _env: &Env) {
        if self.board_image.is_none() || self.piece_images.is_none() {
            self.create_images(ctx)
        }

        self.draw_background(ctx);

        let dpsp = self.get_dragged_piece_start_position();
        for x in 0..8 {
            for y in 0..8 {
                if Some(Position { x, y }) != dpsp {
                    self.draw_square(ctx, Position { x, y });
                }
            }
        }
        // If we are dragging a piece, we need to draw it last in case it overlaps other pieces
        if let Some(position) = dpsp {
            self.draw_square(ctx, position);
        }
    }
}
