use crate::color::Color;

use web_time::{Duration, Instant};

#[derive(Clone)]
pub struct Timer {
    white_time: Duration,
    black_time: Duration,
    start_time: Option<Instant>,
    current_player: Color,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            white_time: Duration::from_secs(60),
            black_time: Duration::from_secs(60),
            start_time: None,
            current_player: Color::White,
        }
    }
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

    pub fn stop(&mut self) {
        self.pause_active_time();
        self.start_time = None;
    }

    fn pause_active_time(&mut self) {
        let elapsed = self
            .start_time
            .take()
            .expect("call `timer.start()` first")
            .elapsed();

        match self.current_player {
            Color::White => {
                self.white_time = self
                    .white_time
                    .checked_sub(elapsed)
                    .unwrap_or(Duration::from_secs(0));
            }
            Color::Black => {
                self.black_time = self
                    .black_time
                    .checked_sub(elapsed)
                    .unwrap_or(Duration::from_secs(0));
            }
        };
    }

    pub fn next_player(&mut self) {
        self.pause_active_time();
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
                .unwrap_or(Duration::from_secs(0))
        } else {
            time
        }
    }

    pub fn get_active_time(&self) -> Duration {
        self.get_time(self.current_player)
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
