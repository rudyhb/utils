use std::hash::{Hash, Hasher};

pub trait GetHash: Hash {
    fn get_hash(&self) -> u64;
}

impl<T: Hash> GetHash for T {
    fn get_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}
