//! PDDB storage for Minesweeper.
//!
//! Dictionary: mines.state
//! Keys: config, best_times

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::minefield::Difficulty;

const DICT: &str = "mines.state";
const KEY_CONFIG: &str = "config";
const KEY_BEST: &str = "best_times";

pub struct Storage {
    pddb: pddb::Pddb,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    fn read_key(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut handle = self
            .pddb
            .get(DICT, key, None, false, false, None, None::<fn()>)
            .ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        handle.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    fn write_key(&mut self, key: &str, data: &[u8]) {
        if let Ok(mut handle) = self.pddb.get(
            DICT, key, None, true, true, Some(data.len()), None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            handle.seek(std::io::SeekFrom::Start(0)).ok();
            handle.write_all(data).ok();
            handle.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    pub fn load_difficulty(&mut self) -> Option<Difficulty> {
        let buf = self.read_key(KEY_CONFIG)?;
        let json: serde_json::Value = serde_json::from_slice(&buf).ok()?;
        match json.get("difficulty").and_then(|v| v.as_str()) {
            Some("beginner") => Some(Difficulty::Beginner),
            Some("intermediate") => Some(Difficulty::Intermediate),
            Some("expert") => Some(Difficulty::Expert),
            _ => None,
        }
    }

    pub fn save_difficulty(&mut self, diff: &Difficulty) {
        let json = serde_json::json!({ "difficulty": diff.key() });
        let data = serde_json::to_vec(&json).unwrap_or_default();
        self.write_key(KEY_CONFIG, &data);
    }

    pub fn load_best_time(&mut self, diff: &Difficulty) -> Option<u32> {
        let buf = self.read_key(KEY_BEST)?;
        let json: serde_json::Value = serde_json::from_slice(&buf).ok()?;
        json.get(diff.key()).and_then(|v| v.as_u64()).map(|v| v as u32)
    }

    pub fn save_best_time(&mut self, diff: &Difficulty, time: u32) {
        // Load existing, merge, save
        let mut json: serde_json::Value = self.read_key(KEY_BEST)
            .and_then(|buf| serde_json::from_slice(&buf).ok())
            .unwrap_or(serde_json::json!({}));

        json[diff.key()] = serde_json::json!(time);
        let data = serde_json::to_vec(&json).unwrap_or_default();
        self.write_key(KEY_BEST, &data);
    }
}
