use chess::moves::Move;

#[derive(Default)]
pub struct Arrows {
    moves: Vec<Move>,
    showing: usize,
}

impl Arrows {
    pub fn push(&mut self, mv: Move) {
        self.moves.drain(self.showing..self.moves.len());
        self.moves.push(mv);
        self.showing += 1;
    }

    pub fn undo(&mut self) {
        if self.showing > 0 {
            self.showing -= 1;
        }
    }

    pub fn redo(&mut self) {
        if self.showing < self.moves.len() {
            self.showing += 1;
        }
    }

    pub fn get(&self) -> Vec<Move> {
        self.moves.iter().copied().take(self.showing).collect()
    }

    pub fn clear(&mut self) {
        self.moves.clear();
        self.showing = 0;
    }
}
