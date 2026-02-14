//! TRNG wrapper — same pattern as Decision Engine.

extern crate alloc;
use alloc::vec::Vec;

pub struct Rng {
    trng: trng::Trng,
}

impl Rng {
    pub fn new(xns: &xous_names::XousNames) -> Self {
        Self {
            trng: trng::Trng::new(xns).expect("can't connect to TRNG"),
        }
    }

    pub fn u32(&self) -> u32 {
        self.trng.get_u32().unwrap_or(0)
    }

    /// Random number in range [0, max) with rejection sampling.
    pub fn range(&self, max: u32) -> u32 {
        if max <= 1 {
            return 0;
        }
        let threshold = u32::MAX - (u32::MAX % max);
        loop {
            let val = self.u32();
            if val < threshold {
                return val % max;
            }
        }
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&self, items: &mut [T]) {
        let n = items.len();
        for i in (1..n).rev() {
            let j = self.range((i + 1) as u32) as usize;
            items.swap(i, j);
        }
    }
}
