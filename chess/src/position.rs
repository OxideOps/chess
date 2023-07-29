use serde::{Deserialize, Serialize};
use std::{
    fmt,
    ops::{Add, AddAssign, Sub},
};

use crate::displacement::Displacement;

const FILES: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
const RANKS: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", FILES[self.x], RANKS[self.y])
    }
}

impl Add<Displacement> for Position {
    type Output = Self;

    fn add(self, m: Displacement) -> Self::Output {
        Self {
            x: self.x.wrapping_add(m.dx as usize),
            y: self.y.wrapping_add(m.dy as usize),
        }
    }
}

impl Sub<Displacement> for Position {
    type Output = Self;

    fn sub(self, m: Displacement) -> Self::Output {
        Self {
            x: self.x.wrapping_sub(m.dx as usize),
            y: self.y.wrapping_sub(m.dy as usize),
        }
    }
}

impl AddAssign<Displacement> for Position {
    fn add_assign(&mut self, m: Displacement) {
        *self = Self {
            x: self.x.wrapping_add(m.dx as usize),
            y: self.y.wrapping_add(m.dy as usize),
        };
    }
}

// Add unit tests at the bottom of each file. Each tests module should only have access to super (non integration)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_position() {
        let mut p = Position { x: 0, y: 0 };
        let m_up = Displacement { dx: 0, dy: 1 };
        let m_right = Displacement { dx: 1, dy: 0 };

        for _ in 0..10 {
            p = p + m_right + m_up
        }
        assert_eq!(p, Position { x: 10, y: 10 })
    }
}
