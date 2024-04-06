use std::time::SystemTime;

pub struct DataItem<T: Clone> {
    value: T,
    ttl: Option<SystemTime>,
}

impl<T: Clone> DataItem<T> {
    pub fn new(value: T, ttl: Option<SystemTime>) -> Self {
        DataItem { value, ttl }
    }
    pub fn expired_or_return(&self) -> Option<T> {
        if let Some(ttl) = self.ttl {
            if ttl < SystemTime::now() {
                return None;
            }
        }
        Some(self.value.clone())
    }
}
