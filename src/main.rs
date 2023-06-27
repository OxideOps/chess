use chess::chess_widget::{ChessWidget, WINDOW_SIZE};
use druid::{AppLauncher, LocalizedString, WindowDesc};

pub fn main() {
    AppLauncher::with_window(
        WindowDesc::new(ChessWidget::default())
            .title(LocalizedString::new("Chess"))
            .window_size((WINDOW_SIZE, WINDOW_SIZE)),
    )
    .launch(())
    .expect("launch failed");

    println!("---Game ended---")
}
