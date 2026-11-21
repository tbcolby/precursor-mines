//! Game time that runs only while the game is in front.
//!
//! The shell passes ticktimer milliseconds in. Time spent in the background is not
//! counted (0.1 used wall time from the first click, so switching apps mid-game
//! inflated the result), and `stop` freezes it at the exact moment of the winning
//! move (0.1 recorded the time of the previous key press).

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    acc_ms: u64,
    since: Option<u64>,
    started: bool,
}

impl Clock {
    /// Begin counting at `now`.
    pub fn start(&mut self, now: u64) {
        self.acc_ms = 0;
        self.since = Some(now);
        self.started = true;
    }

    pub fn is_started(&self) -> bool { self.started }

    pub fn is_running(&self) -> bool { self.since.is_some() }

    /// Stop counting (app went to the background, or the game ended).
    pub fn pause(&mut self, now: u64) {
        if let Some(s) = self.since.take() {
            self.acc_ms += now.saturating_sub(s);
        }
    }

    /// Continue after a pause; does nothing before `start`.
    pub fn resume(&mut self, now: u64) {
        if self.started && self.since.is_none() {
            self.since = Some(now);
        }
    }

    pub fn reset(&mut self) { *self = Clock::default(); }

    pub fn elapsed_ms(&self, now: u64) -> u64 { self.acc_ms + self.since.map_or(0, |s| now.saturating_sub(s)) }

    /// Whole seconds, shown as 000-999.
    pub fn secs(&self, now: u64) -> u32 { (self.elapsed_ms(now) / 1000).min(999) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_foreground_time() {
        let mut c = Clock::default();
        assert_eq!(c.secs(50_000), 0);
        c.resume(1_000); // before start: nothing
        assert!(!c.is_started());
        c.start(10_000);
        assert_eq!(c.secs(12_500), 2);
        c.pause(13_000); // 3 s played
        assert_eq!(c.secs(99_000), 3, "background time doesn't count");
        c.resume(100_000);
        assert_eq!(c.secs(101_999), 4);
        c.pause(102_000);
        assert_eq!(c.elapsed_ms(500_000), 5_000);
    }

    #[test]
    fn stop_at_the_winning_move_uses_that_moments_time() {
        // Regression: best time was computed before the timer was updated for the final key.
        let mut c = Clock::default();
        c.start(0);
        c.pause(37_400);
        assert_eq!(c.secs(37_400), 37);
        assert!(!c.is_running());
    }

    #[test]
    fn caps_at_999_and_survives_clock_going_backwards() {
        let mut c = Clock::default();
        c.start(1_000);
        assert_eq!(c.secs(5_000_000), 999);
        assert_eq!(c.secs(10), 0, "saturating");
        c.reset();
        assert!(!c.is_started() && c.secs(7) == 0);
    }
}
