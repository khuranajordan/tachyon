use sha1::{Digest, Sha1};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Oid([u8; 20]);

impl Oid {
    pub const ZERO: Oid = Oid([0u8; 20]);

    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Oid(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let trimmed = hex_str.trim();
        if trimmed.len() != 40 {
            return Err(format!("Invalid OID hex length: expected 40, got {}", trimmed.len()));
        }
        let mut bytes = [0u8; 20];
        for i in 0..20 {
            let byte_str = &trimmed[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|e| format!("Invalid hex byte '{}': {}", byte_str, e))?;
        }
        Ok(Oid(bytes))
    }

    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(40);
        for byte in &self.0 {
            s.push_str(&format!("{:02x}", byte));
        }
        s
    }

    pub fn short(&self) -> String {
        let full = self.to_hex();
        full[..7.min(full.len())].to_string()
    }

    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&result[..20]);
        Oid(bytes)
    }

    pub fn hash_object(obj_type: &str, content: &[u8]) -> Self {
        let header = format!("{} {}\0", obj_type, content.len());
        let mut hasher = Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(content);
        let result = hasher.finalize();
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&result[..20]);
        Oid(bytes)
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({})", self.to_hex())
    }
}
