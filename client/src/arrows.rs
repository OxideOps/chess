use chess::moves::Move;

pub const ALPHA: f64 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowData {
    pub(crate) mv: Move,
    pub(crate) alpha: f64,
}

impl Default for ArrowData {
    fn default() -> Self {
        Self {
            mv: Move::default(),
            alpha: ALPHA,
        }
    }
}

impl ArrowData {
    pub fn new(mv: Move, alpha: f64) -> Self {
        Self { mv, alpha }
    }
    pub fn with_move(mv: Move) -> Self {
        Self {
            mv,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct Arrows {
    arrows: Vec<ArrowData>,
    showing: usize,
}

impl Arrows {
    pub fn new(moves: Vec<Move>) -> Self {
        Self {
            showing: moves.len(),
            arrows: moves.into_iter().map(ArrowData::with_move).collect(),
        }
    }

    pub fn push(&mut self, arrow_data: ArrowData) {
        self.arrows.drain(self.showing..self.arrows.len());
        self.arrows.push(arrow_data);
        self.showing += 1;
    }

    pub fn undo(&mut self) {
        if self.showing > 0 {
            self.showing -= 1;
        }
    }

    pub fn redo(&mut self) {
        if self.showing < self.arrows.len() {
            self.showing += 1;
        }
    }

    pub fn get(&self) -> Vec<ArrowData> {
        self.arrows.iter().copied().take(self.showing).collect()
    }

    pub fn clear(&mut self) {
        self.arrows.clear();
        self.showing = 0;
    }

    pub fn set(&mut self, i: usize, arrow_data: ArrowData) {
        self.arrows[i] = arrow_data;
    }
}
