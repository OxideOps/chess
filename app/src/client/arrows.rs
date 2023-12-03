use chess::Move;
use palette::{LinSrgb, LinSrgba, WithAlpha};

pub(super) const ALPHA: f64 = 0.75;
pub(super) const ANALYSIS_COLOR: LinSrgb<f64> = LinSrgb::new(0.11, 0.53, 0.73);
pub(super) const USER_COLOR: LinSrgb<f64> = LinSrgb::new(0.99, 0.62, 0.01);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ArrowData {
    pub(crate) mv: Move,
    pub(crate) color: LinSrgba<f64>,
}

impl ArrowData {
    pub(super) fn new(mv: Move, color: LinSrgba<f64>) -> Self {
        Self { mv, color }
    }

    pub(super) fn analysis_arrow(mv: Move) -> Self {
        Self {
            mv,
            color: ANALYSIS_COLOR.with_alpha(ALPHA),
        }
    }

    pub(super) fn user_arrow(mv: Move) -> Self {
        Self {
            mv,
            color: USER_COLOR.with_alpha(ALPHA),
        }
    }

    pub(super) fn has_length(&self) -> bool {
        self.mv.from != self.mv.to
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
            arrows: moves.into_iter().map(ArrowData::analysis_arrow).collect(),
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
