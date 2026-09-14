//! Glicko-2 ratings (Mark Glickman, "Example of the Glicko-2 system",
//! 2013: <http://www.glicko.net/glicko/glicko2.pdf>).
//!
//! Pure functions; the database layer decides when to call them. Like
//! Lichess, each game is its own rating period, and a player's deviation
//! grows back while they don't play (one period per day). Ratings are kept
//! as `f64` and rounded only for display.

use std::f64::consts::PI;

/// Converts between the Glicko scale and Glicko-2's internal one.
const SCALE: f64 = 173.7178;
/// The system constant: how much volatility may change per period.
pub const TAU: f64 = 0.5;
/// A deviation above this means the rating is still a guess ("1500?").
pub const PROVISIONAL_DEVIATION: f64 = 110.0;
/// Where deviations start, and the most they grow back to.
pub const MAX_DEVIATION: f64 = 350.0;
/// Keeps established ratings responsive (Lichess uses the same floor).
pub const MIN_DEVIATION: f64 = 45.0;
const CONVERGENCE: f64 = 0.000_001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

impl Default for Rating {
    /// A player with no games yet.
    fn default() -> Self {
        Rating {
            rating: 1500.0,
            deviation: MAX_DEVIATION,
            volatility: 0.06,
        }
    }
}

impl Rating {
    pub fn provisional(&self) -> bool {
        self.deviation > PROVISIONAL_DEVIATION
    }

    /// The rating as shown: rounded to a whole number.
    pub fn shown(&self) -> i32 {
        self.rating.round() as i32
    }

    /// `periods` rating periods without games: the deviation grows (step 6
    /// of the paper, repeated), up to [`MAX_DEVIATION`].
    pub fn aged(self, periods: f64) -> Rating {
        let phi = self.deviation / SCALE;
        let grown = (phi * phi + periods.max(0.0) * self.volatility * self.volatility).sqrt();
        Rating {
            deviation: (grown * SCALE).min(MAX_DEVIATION),
            ..self
        }
    }

    /// The rating after one period with `games`: each an opponent's rating
    /// (as of before the period) and the score, 1 for a win, ½ for a draw, 0
    /// for a loss. No games is plain ageing by one period.
    pub fn update(self, games: &[(Rating, f64)]) -> Rating {
        if games.is_empty() {
            return self.aged(1.0);
        }
        // Step 2: to the Glicko-2 scale.
        let mu = (self.rating - 1500.0) / SCALE;
        let phi = self.deviation / SCALE;
        let sigma = self.volatility;

        // Steps 3 and 4: estimated variance and improvement.
        let mut inverse_v = 0.0;
        let mut sum = 0.0;
        for (opponent, score) in games {
            let mu_j = (opponent.rating - 1500.0) / SCALE;
            let g_j = g(opponent.deviation / SCALE);
            let e_j = expected(mu, mu_j, g_j);
            inverse_v += g_j * g_j * e_j * (1.0 - e_j);
            sum += g_j * (score - e_j);
        }
        let v = 1.0 / inverse_v;
        let delta = v * sum;

        // Step 5: the new volatility, by the Illinois algorithm.
        let a = (sigma * sigma).ln();
        let f = |x: f64| {
            let ex = x.exp();
            ex * (delta * delta - phi * phi - v - ex) / (2.0 * (phi * phi + v + ex).powi(2))
                - (x - a) / (TAU * TAU)
        };
        let mut big_a = a;
        let mut big_b = if delta * delta > phi * phi + v {
            (delta * delta - phi * phi - v).ln()
        } else {
            let mut k = 1.0;
            while f(a - k * TAU) < 0.0 {
                k += 1.0;
            }
            a - k * TAU
        };
        let mut f_a = f(big_a);
        let mut f_b = f(big_b);
        while (big_b - big_a).abs() > CONVERGENCE {
            let big_c = big_a + (big_a - big_b) * f_a / (f_b - f_a);
            let f_c = f(big_c);
            if f_c * f_b <= 0.0 {
                big_a = big_b;
                f_a = f_b;
            } else {
                f_a /= 2.0;
            }
            big_b = big_c;
            f_b = f_c;
        }
        let new_sigma = (big_a / 2.0).exp();

        // Steps 6 to 8: the new deviation and rating, back on the Glicko scale.
        let phi_star = (phi * phi + new_sigma * new_sigma).sqrt();
        let new_phi = 1.0 / (1.0 / (phi_star * phi_star) + 1.0 / v).sqrt();
        let new_mu = mu + new_phi * new_phi * sum;
        Rating {
            rating: new_mu * SCALE + 1500.0,
            deviation: (new_phi * SCALE).clamp(MIN_DEVIATION, MAX_DEVIATION),
            volatility: new_sigma,
        }
    }
}

fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (PI * PI)).sqrt()
}

fn expected(mu: f64, mu_j: f64, g_j: f64) -> f64 {
    1.0 / (1.0 + (-g_j * (mu - mu_j)).exp())
}

/// One rated game between `white` and `black` (each as of before the game,
/// already aged): both new ratings. `white_score` is 1, ½ or 0.
pub fn rate_game(white: Rating, black: Rating, white_score: f64) -> (Rating, Rating) {
    (
        white.update(&[(black, white_score)]),
        black.update(&[(white, 1.0 - white_score)]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(rating: f64, deviation: f64) -> Rating {
        Rating {
            rating,
            deviation,
            volatility: 0.06,
        }
    }

    #[test]
    fn matches_glickmans_worked_example() {
        // Section "Example calculation" of the paper.
        let player = r(1500.0, 200.0);
        let after = player.update(&[
            (r(1400.0, 30.0), 1.0),
            (r(1550.0, 100.0), 0.0),
            (r(1700.0, 300.0), 0.0),
        ]);
        assert!((after.rating - 1464.06).abs() < 0.01, "{after:?}");
        assert!((after.deviation - 151.52).abs() < 0.01, "{after:?}");
        assert!((after.volatility - 0.05999).abs() < 0.00001, "{after:?}");
    }

    #[test]
    fn one_game_moves_ratings_the_expected_way() {
        let (w, b) = rate_game(Rating::default(), Rating::default(), 1.0);
        assert!(w.rating > 1500.0 && b.rating < 1500.0, "{w:?} {b:?}");
        // Equal players, equal and opposite changes.
        assert!((w.rating - 1500.0 + (b.rating - 1500.0)).abs() < 1e-9);
        assert!(w.deviation < MAX_DEVIATION && w.provisional());

        let (w, b) = rate_game(Rating::default(), Rating::default(), 0.5);
        assert!((w.rating - 1500.0).abs() < 1e-9 && (b.rating - 1500.0).abs() < 1e-9);

        // Beating a much stronger, established player is worth more than
        // beating an equal one; losing to them costs little.
        let strong = r(1900.0, 60.0);
        let upset = r(1500.0, 60.0).update(&[(strong, 1.0)]).rating - 1500.0;
        let par = r(1500.0, 60.0).update(&[(r(1500.0, 60.0), 1.0)]).rating - 1500.0;
        let expected_loss = r(1500.0, 60.0).update(&[(strong, 0.0)]).rating - 1500.0;
        assert!(upset > par && par > 0.0, "{upset} {par}");
        assert!(
            expected_loss < 0.0 && expected_loss.abs() < par,
            "{expected_loss}"
        );
    }

    #[test]
    fn deviation_shrinks_with_games_and_grows_with_time_within_bounds() {
        let mut player = Rating::default();
        let opponent = r(1500.0, 50.0);
        for i in 0..200 {
            player = player.update(&[(opponent, if i % 2 == 0 { 1.0 } else { 0.0 })]);
        }
        // Settles well inside the bounds (volatility keeps it off the floor).
        assert!(!player.provisional());
        assert!(
            (MIN_DEVIATION..PROVISIONAL_DEVIATION).contains(&player.deviation),
            "{player:?}"
        );
        // The floor applies when the maths would go below it.
        let certain = r(1500.0, 20.0).update(&[(r(1500.0, 20.0), 1.0)]);
        assert_eq!(certain.deviation, MIN_DEVIATION);

        let rested = player.aged(30.0);
        assert!(rested.deviation > player.deviation);
        assert_eq!(rested.rating, player.rating);
        assert_eq!(player.aged(1_000_000.0).deviation, MAX_DEVIATION);
        assert_eq!(player.aged(0.0), player);
        assert_eq!(player.aged(-3.0), player);
    }

    #[test]
    fn shown_and_provisional() {
        assert_eq!(r(1499.5, 80.0).shown(), 1500);
        assert!(!r(1500.0, 110.0).provisional());
        assert!(r(1500.0, 110.1).provisional());
    }
}
