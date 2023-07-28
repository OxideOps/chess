use web_time::Duration;

use crate::board_state::BoardState; 
use crate::game::Game; 
use crate::history::History; 
use crate::pieces::Color; 
use crate::timer::Timer;

pub struct GameBuilder {
    duration: Duration,
    state: BoardState,
}

impl Default for GameBuilder {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            state: BoardState::default(),
        }
    }
}

impl GameBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Game {
        let mut game = Game {
            history: History::with_state(self.state),
            timer: Timer::with_duration(self.duration),
            ..Default::default()
        };
        game.add_moves();
        game
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn state(mut self, state: BoardState) -> Self {
        self.state = state;
        self
    }

    pub fn player(mut self, player: Color) -> Self {
        self.state.player = player;
        self
    }
}
