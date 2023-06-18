use chess::chess_widget::{ChessWidget, WINDOW_SIZE};
use druid::{AppLauncher, LocalizedString, WindowDesc};

pub fn main() {
    let window = WindowDesc::new(|| ChessWidget::new())
        .title(LocalizedString::new("Chess"))
        .window_size((WINDOW_SIZE, WINDOW_SIZE));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch("Druid + Piet".to_string())
        .expect("launch failed");
}
