use std::time::SystemTime;

pub mod in_memory_storage;

pub trait KVStorage {
    fn set(&self, key: String, value: String, ttl: Option<SystemTime>);

    fn get(&self, key: &str) -> Option<String>;
}
