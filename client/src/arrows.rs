use chess::moves::Move;

pub(super) const ALPHA: f64 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ArrowData {
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
    pub(super) fn new(mv: Move, alpha: f64) -> Self {
        Self { mv, alpha }
    }
    pub(super) fn with_move(mv: Move) -> Self {
        Self {
            mv,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub(super) struct Arrows {
    arrows: Vec<ArrowData>,
    showing: usize,
}

impl Arrows {
    pub(super) fn new(moves: Vec<Move>) -> Self {
        Self {
            showing: moves.len(),
            arrows: moves.into_iter().map(ArrowData::with_move).collect(),
        }
    }

    pub(super) fn with_size(n: usize) -> Self {
        Self::new(vec![Move::default(); n])
    }

    pub(super) fn push(&mut self, arrow_data: ArrowData) {
        self.arrows.drain(self.showing..self.arrows.len());
        self.arrows.push(arrow_data);
        self.showing += 1;
    }

    pub(super) fn undo(&mut self) {
        if self.showing > 0 {
            self.showing -= 1;
        }
    }

    pub(super) fn redo(&mut self) {
        if self.showing < self.arrows.len() {
            self.showing += 1;
        }
    }

    pub(super) fn get(&self) -> Vec<ArrowData> {
        self.arrows.iter().copied().take(self.showing).collect()
    }

    pub(super) fn clear(&mut self) {
        self.arrows.clear();
        self.showing = 0;
    }

    pub(super) fn set(&mut self, i: usize, arrow_data: ArrowData) {
        self.arrows[i] = arrow_data;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.arrows.is_empty()
    }
}
