use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple Hashable. A simple way for a type to hash itself
trait SimpleHashable {
    fn hashed(&self) -> u64;
}

impl<'a> SimpleHashable for &'a str {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl SimpleHashable for String {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl SimpleHashable for &String {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl SimpleHashable for usize {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// A newtype to define `From` implementations
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AnyHashable(u64);

impl AnyHashable {
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<&str> for AnyHashable {
    fn from(value: &str) -> Self {
        AnyHashable(value.hashed())
    }
}

impl From<String> for AnyHashable {
    fn from(value: String) -> Self {
        AnyHashable(value.hashed())
    }
}

impl From<&String> for AnyHashable {
    fn from(value: &String) -> Self {
        AnyHashable(value.hashed())
    }
}

impl From<usize> for AnyHashable {
    fn from(value: usize) -> Self {
        AnyHashable(value.hashed())
    }
}
