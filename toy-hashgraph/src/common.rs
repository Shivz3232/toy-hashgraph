use std::fmt::Write;

pub const KEY_SIZE: usize = 32;
pub type Key = [u8; KEY_SIZE];

pub const HASH_SIZE: usize = 32;
pub type Hash = [u8; HASH_SIZE];

pub const SIGNATURE_SIZE: usize = 64;
pub type Signature = [u8; SIGNATURE_SIZE];

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(s, "{:02x}", byte).unwrap();
    }
    s
}
