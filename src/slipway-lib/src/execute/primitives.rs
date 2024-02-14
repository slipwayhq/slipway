use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Hash {
    value: [u8; 32],
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.value {
            // Write each byte as a two-digit hexadecimal number.
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl Hash {
    pub fn from_value(value: &Value) -> Self {
        // Convert the JSON value to a sorted string to ensure consistency
        let serialized = serde_json::to_string(&value).expect("serde_json::Value should serialize");

        // Create a Sha256 object
        let mut hasher = Sha256::new();

        // Write input message
        hasher.update(serialized.as_bytes());

        // Read hash digest and consume hasher
        let value = hasher.finalize();

        Hash {
            value: value.into(),
        }
    }
}
