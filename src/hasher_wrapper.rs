use strum::VariantNames;
use strum_macros::EnumString;
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

/// An enum wrapper to allow storing different hashers with similar operations in the same iterator
#[derive(EnumString, strum_macros::VariantNames)]
#[strum(serialize_all = "kebab-case")]
pub enum HasherWrapper {
    #[strum(to_string = "xxh64")]
    Xxh64(Xxh64),
    #[strum(to_string = "xxh3")]
    Xxh3(Xxh3),
}

pub fn default_hash() -> String {
    // TODO: Make this more elegant and less prone to errors
    "xxh64".to_string()
}

pub fn variants() -> Vec<String> {
    HasherWrapper::VARIANTS
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

// /// Hardcoded default. Using XXH64 for speed
// impl Default for HasherWrapper {
//     fn default() -> Self {
//         HasherWrapper::Xxh64(new_xxh64())
//     }
// }

/// function to keep hasher dependency for the Xxh64 hasher in this module
pub fn new_xxh64() -> Xxh64 {
    Xxh64::new(0)
}

/// function to keep hasher dependency for the Xxh3 hasher in this module
pub fn new_xxh3() -> Xxh3 {
    Xxh3::new()
}

/// implementing a minimum set of functions to unify different hasher stream operations
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

/// function to check if a given hash is implemented in
/// this module
///
/// # Arguments
/// * `hash_list` - A comma-separated list of hash names to check against implemented hashers
///
/// # Returns
/// A tuple of two vectors:
/// * First vector contains valid/implemented hash names
/// * Second vector contains invalid/unimplemented hash names
pub fn check_hash(hash_list: &String) -> (Vec<String>, Vec<String>) {
    let implemented_hashes = variants();

    let hash_cleaned = hash_list.replace(" ", "").to_lowercase();

    hash_cleaned
        .split(",")
        .map(|s| s.to_string())
        .into_iter()
        .partition(|candidate| implemented_hashes.iter().any(|valid| valid.eq(candidate)))
}
