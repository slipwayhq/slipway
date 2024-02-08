use serde_json::Value;
use sha2::{Digest, Sha256};

fn hash_json_value(value: &Value) -> Result<String, serde_json::Error> {
    // Convert the JSON value to a sorted string to ensure consistency
    let serialized = serde_json::to_string(&value)?;

    // Create a Sha256 object
    let mut hasher = Sha256::new();

    // Write input message
    hasher.update(serialized.as_bytes());

    // Read hash digest and consume hasher
    let result = hasher.finalize();

    // Convert the hash to a hexadecimal string
    Ok(format!("{:x}", result))
}
