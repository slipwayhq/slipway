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
    pub fn new(value: [u8; 32]) -> Self {
        Hash { value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonMetadata {
    pub hash: Hash,
    pub serialized: String,
}

impl JsonMetadata {
    pub fn from_value(value: &Value) -> Self {
        let serialized = serde_json::to_string(&value).expect("serde_json::Value should serialize");
        let serialized_bytes = serialized.as_bytes();

        let mut hasher = Sha256::new();
        hasher.update(serialized_bytes);

        // Read hash digest and consume hasher
        let value = hasher.finalize();
        let hash = Hash::new(value.into());

        JsonMetadata { hash, serialized }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn it_should_create_json_value_metadata() {
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

        let metadata = JsonMetadata::from_value(&json);
        let metadata_of_clone = JsonMetadata::from_value(&json_clone);
        let metadata_of_other = JsonMetadata::from_value(&json_other);

        assert_eq!(metadata, metadata_of_clone);
        assert_ne!(metadata, metadata_of_other);

        assert_eq!(metadata.hash.value.len(), 32);

        assert_eq!(metadata.serialized, "{\"a\":1,\"b\":2,\"c\":3}");
    }
}
