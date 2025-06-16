use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

/// An enum wrapper to allow storing different hashers with similar operations in the same iterator
pub enum HasherWrapper {
    Xxh64(Xxh64),
    Xxh3(Xxh3),
}

/// function to keep hasher dependency for the Xxh64 hasher in this module
pub fn new_xxh64() -> Xxh64 {
    Xxh64::new(0)
}
/// function to keep hasher dependency for the Xxh3 hasher in this module
pub fn new_xxh3() -> Xxh3 {
    Xxh3::new()
}
impl HasherWrapper {
    pub fn update(&mut self, data: &[u8]) {
        match self {
            HasherWrapper::Xxh64(hasher) => hasher.update(data),
            HasherWrapper::Xxh3(hasher) => hasher.update(data),
        }
    }

    pub fn finish(&self) -> String {
        match self {
            HasherWrapper::Xxh64(hasher) => format!("{:016X}", hasher.digest()),
            HasherWrapper::Xxh3(hasher) => format!("{:032X}", hasher.digest128()),
        }
    }
}
