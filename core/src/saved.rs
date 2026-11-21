//! Best times and the last difficulty, as one versioned JSON record.

use crate::field::Difficulty;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub difficulty: Difficulty,
    /// Best time in seconds per difficulty (beginner, intermediate, expert).
    pub best: [Option<u32>; 3],
}

impl Default for Saved {
    fn default() -> Self { Self { version: VERSION, difficulty: Difficulty::Beginner, best: [None; 3] } }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    Parse,
    /// Written by a newer app version: don't overwrite it.
    Newer(u32),
}

impl Saved {
    pub fn best(&self, d: Difficulty) -> Option<u32> { self.best[d.index()] }

    /// Record a win. `true` if it is a new best.
    pub fn record(&mut self, d: Difficulty, secs: u32) -> bool {
        let slot = &mut self.best[d.index()];
        if slot.is_none_or(|b| secs < b) {
            *slot = Some(secs);
            true
        } else {
            false
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> { serde_json::to_vec(self).unwrap_or_default() }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        let s: Saved = serde_json::from_slice(bytes).map_err(|_| LoadError::Parse)?;
        if s.version > VERSION {
            return Err(LoadError::Newer(s.version));
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_only_improvements() {
        let mut s = Saved::default();
        assert!(s.record(Difficulty::Expert, 300));
        assert!(!s.record(Difficulty::Expert, 300), "equal time isn't a new best");
        assert!(!s.record(Difficulty::Expert, 301));
        assert!(s.record(Difficulty::Expert, 299));
        assert_eq!(s.best(Difficulty::Expert), Some(299));
        assert_eq!(s.best(Difficulty::Beginner), None);
    }

    #[test]
    fn round_trip_and_shorter_rewrites() {
        // Regression: a shorter value written over a longer one left a trailing byte and every
        // best time read as missing. Records are replaced whole; a stale tail is a parse error.
        let mut s = Saved::default();
        s.record(Difficulty::Beginner, 100);
        let long = s.to_bytes();
        assert_eq!(Saved::from_bytes(&long).unwrap(), s);
        s.record(Difficulty::Beginner, 99);
        let short = s.to_bytes();
        assert!(short.len() < long.len());
        let mut stale = short.clone();
        stale.extend_from_slice(&long[short.len()..]);
        assert_eq!(Saved::from_bytes(&stale), Err(LoadError::Parse));
        assert_eq!(Saved::from_bytes(&short).unwrap().best(Difficulty::Beginner), Some(99));
    }

    #[test]
    fn newer_version_and_garbage_are_errors() {
        let mut s = Saved::default();
        s.version = 9;
        assert_eq!(Saved::from_bytes(&s.to_bytes()), Err(LoadError::Newer(9)));
        assert_eq!(Saved::from_bytes(b"[]"), Err(LoadError::Parse));
    }
}
