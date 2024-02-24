use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hash {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn it_should_hash_json_value() {
        let json = json!({
            "a": 1,
            "b": 2,
            "c": 3,
        });

        let json_clone = json.clone();

        let json_other = json!({
            "a": 2,
            "b": 2,
            "c": 3,
        });

        let hash = Hash::from_value(&json);
        let hash_of_clone = Hash::from_value(&json_clone);
        let hash_of_other = Hash::from_value(&json_other);

        assert_eq!(hash.to_string(), hash_of_clone.to_string());
        assert_ne!(hash.to_string(), hash_of_other.to_string());

        assert_eq!(hash.value.len(), 32);
    }
}
