//! Rate limiting for the account endpoints.
//!
//! Fixed windows in memory, keyed by whatever the caller chooses (a username,
//! a client address). Good enough to blunt password guessing and signup
//! spam on one server; a shared store would be needed for several.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

/// How many hits a key gets per window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit {
    pub hits: u32,
    pub window: Duration,
}

impl Limit {
    pub const fn per_minute(hits: u32) -> Limit {
        Limit {
            hits,
            window: Duration::from_secs(60),
        }
    }
}

#[derive(Debug)]
struct Window {
    started: Instant,
    hits: u32,
}

#[derive(Debug, Default)]
pub struct Limiter {
    windows: Mutex<HashMap<String, Window>>,
}

/// Keep the map from growing without bound: past this many keys, expired
/// windows are dropped on the next hit.
const PRUNE_AT: usize = 10_000;

impl Limiter {
    /// Count one hit for `key`. `Err(wait)` when the key is over its limit,
    /// with how long until the window resets.
    pub fn hit(&self, key: &str, limit: Limit, now: Instant) -> Result<(), Duration> {
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if windows.len() >= PRUNE_AT {
            windows.retain(|_, w| now.duration_since(w.started) < limit.window);
        }
        let window = windows.entry(key.to_string()).or_insert(Window {
            started: now,
            hits: 0,
        });
        let elapsed = now.duration_since(window.started);
        if elapsed >= limit.window {
            window.started = now;
            window.hits = 0;
        }
        if window.hits >= limit.hits {
            return Err(limit.window - elapsed);
        }
        window.hits += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_waits_for_the_window() {
        let limiter = Limiter::default();
        let limit = Limit::per_minute(3);
        let t0 = Instant::now();
        for _ in 0..3 {
            assert_eq!(limiter.hit("a", limit, t0), Ok(()));
        }
        let wait = limiter
            .hit("a", limit, t0 + Duration::from_secs(10))
            .unwrap_err();
        assert_eq!(wait, Duration::from_secs(50));
        // Other keys are independent.
        assert_eq!(
            limiter.hit("b", limit, t0 + Duration::from_secs(10)),
            Ok(())
        );
        // The window resets.
        assert_eq!(
            limiter.hit("a", limit, t0 + Duration::from_secs(60)),
            Ok(())
        );
    }

    #[test]
    fn prunes_expired_windows_when_large() {
        let limiter = Limiter::default();
        let limit = Limit::per_minute(1);
        let t0 = Instant::now();
        for i in 0..PRUNE_AT {
            limiter.hit(&format!("k{i}"), limit, t0).unwrap();
        }
        assert_eq!(limiter.windows.lock().unwrap().len(), PRUNE_AT);
        limiter
            .hit("fresh", limit, t0 + Duration::from_secs(61))
            .unwrap();
        assert_eq!(limiter.windows.lock().unwrap().len(), 1);
    }
}
