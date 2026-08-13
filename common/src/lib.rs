use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bug {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub bounty: u32,
    pub resolved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LwwEntry {
    pub bug: Bug,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BugMap {
    pub bugs: HashMap<String, LwwEntry>,
}

impl BugMap {
    pub fn new() -> Self {
        Self { bugs: HashMap::new() }
    }

    pub fn insert(&mut self, bug: Bug, timestamp: i64) {
        let id = bug.id.clone();
        let entry = LwwEntry { bug, timestamp };
        
        if let Some(existing) = self.bugs.get(&id) {
            if existing.timestamp > timestamp {
                return;
            }
            if existing.timestamp == timestamp && existing.bug.id > entry.bug.id {
                return;
            }
        }
        
        self.bugs.insert(id, entry);
    }

    pub fn merge(&mut self, other: &BugMap) {
        for (_, other_entry) in &other.bugs {
            self.insert(other_entry.bug.clone(), other_entry.timestamp);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WsMessage {
    Sync(BugMap),
    Update(BugMap),
}
