#![allow(dead_code)]
use std::{collections::HashMap, time::SystemTime};

use crate::data_item::DataItem;

pub struct InMemoryStorage {
    storage: HashMap<String, DataItem<String>>,
}

impl InMemoryStorage {
    fn new() -> Self {
        InMemoryStorage {
            storage: HashMap::new(),
        }
    }

    fn set(&mut self, key: String, value: String, ttl: Option<SystemTime>) {
        self.storage.insert(key, DataItem::new(value, ttl));
    }

    fn get(&self, key: &str) -> Option<String> {
        if let Some(data_item) = self.storage.get(key) {
            return data_item.expired_or_return();
        }
        None
    }
}
