use crate::pieces::Color;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[derive(Clone)]
pub struct Timer {
    white_time: Duration,
    black_time: Duration,
    start_time: Option<Instant>,
    current_player: Color,
}

impl Timer {
    pub fn with_duration(duration: Duration) -> Self {
        Self {
            white_time: duration,
            black_time: duration,
            start_time: None,
            current_player: Color::White,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn next_player(&mut self) {
        let elapsed = self
            .start_time
            .take()
            .expect("call `timer.start()` first")
            .elapsed();

        match self.current_player {
            Color::White => {
                self.white_time = self.white_time.checked_sub(elapsed).expect("time ran out")
            }
            Color::Black => {
                self.black_time = self.black_time.checked_sub(elapsed).expect("time ran out")
            }
        };

        self.current_player = !self.current_player;
        self.start();
    }

    pub fn get_time(&self, player: Color) -> Duration {
        let time = match player {
            Color::White => self.white_time,
            Color::Black => self.black_time,
        };

        if self.start_time.is_some() && player == self.current_player {
            time.checked_sub(self.start_time.unwrap().elapsed())
                .expect("time ran out")
        } else {
            time
        }
    }

    pub fn print(&self) {
        for &player in &[Color::White, Color::Black] {
            let total_seconds = self.get_time(player).as_secs();
            let minutes = total_seconds / 60;
            let seconds = total_seconds % 60;

            log::info!("{:?}: {:02}:{:02}", player, minutes, seconds);
        }
    }

    pub fn is_active(&self) -> bool {
        self.start_time.is_some()
    }
}
