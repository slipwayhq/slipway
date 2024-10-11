use sha2::{Digest, Sha256};
use url::Url;

const HASH_PREFIX_LENGTH: usize = 8;
const MAX_FILENAME_BASE_LENGTH: usize = 100;
const COMPONENT_FILE_EXTENSION: &str = ".tar";

// Creates a valid filename from a URL by hashing the URL and combining it with the host and path
pub(super) fn filename_from_url(url: &Url) -> String {
    let url_str = url.as_str();

    // Compute the SHA256 hash of the URL
    let mut hasher = Sha256::new();
    hasher.update(url_str.as_bytes());
    let hash_result = hasher.finalize();
    let hash_hex = format!("{:x}", hash_result);
    let hash_prefix = &hash_hex[..HASH_PREFIX_LENGTH];

    // Build the base of the filename from the host and path
    let host = url.host_str().unwrap_or("");
    let path = url.path();
    let filename_base = format!("{}{}", host, path);

    // Sanitize the filename by replacing invalid characters with underscores
    let sanitized_filename_base = sanitize_filename(&filename_base);

    // Truncate the filename base if it's too long
    let truncated_filename_base = if sanitized_filename_base.len() > MAX_FILENAME_BASE_LENGTH {
        &sanitized_filename_base[..MAX_FILENAME_BASE_LENGTH]
    } else {
        &sanitized_filename_base
    };

    // Combine the sanitized filename base, hash, and extension to form the final filename
    format!(
        "{}-{}{}",
        truncated_filename_base, hash_prefix, COMPONENT_FILE_EXTENSION
    )
}

fn sanitize_filename(filename: &str) -> String {
    let is_char_allowed = |c: char| c.is_alphanumeric() || c == '.' || c == '-' || c == '_';

    let mut sanitized = String::with_capacity(filename.len());

    for c in filename.chars() {
        if is_char_allowed(c) {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }

    // Ensure the filename doesn't start with a dot
    if sanitized.starts_with('.') {
        sanitized.replace_range(0..1, "_");
    }

    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn assert_filename(filename: &str, expected_prefix: &str) {
        // Check we are sufficient length to include the hash and extension
        assert!(filename.len() > HASH_PREFIX_LENGTH + COMPONENT_FILE_EXTENSION.len() + 1);

        // Check the filename ends with the correct extension
        assert!(filename.ends_with(COMPONENT_FILE_EXTENSION));

        // Check the hash only contains base64 characters
        let extension_start_index = filename.len() - COMPONENT_FILE_EXTENSION.len();
        let hash_start_index = extension_start_index - HASH_PREFIX_LENGTH;
        let hash = &filename[hash_start_index..extension_start_index];
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Check the hash is preceded by a dash
        assert_eq!(filename.as_bytes()[hash_start_index - 1], b'-');

        // Check the filename starts with the expected prefix
        assert_eq!(&filename[..hash_start_index - 1], expected_prefix);
    }

    #[test]
    fn filename_from_url_simple() {
        let url = Url::parse("http://example.com/path/to/resource").unwrap();
        let filename = filename_from_url(&url);
        assert_filename(&filename, "example.com_path_to_resource");
    }

    #[test]
    fn filename_from_url_special_chars() {
        let url = Url::parse("https://example.com/path/with%20space/and#fragment").unwrap();
        let filename = filename_from_url(&url);
        assert_filename(&filename, "example.com_path_with_20space_and");
    }

    #[test]
    fn filename_from_url_long_path() {
        // Create a URL with a very long path to test truncation
        let long_path = &"a".repeat(MAX_FILENAME_BASE_LENGTH * 2);
        let url_str = format!("http://example.com/{}", long_path);
        let url = Url::parse(&url_str).unwrap();
        let filename = filename_from_url(&url);

        let truncated_path =
            long_path[..MAX_FILENAME_BASE_LENGTH - "example.com/".len()].to_string();

        let expected_prefix = "example.com_".to_string() + &truncated_path;

        assert_filename(&filename, &expected_prefix);
    }

    #[test]
    fn filename_from_url_no_host() {
        let url_str = "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3D";
        let url = Url::parse(url_str).unwrap();
        let filename = filename_from_url(&url);

        assert_filename(&filename, "text_plain_base64_SGVsbG8sIFdvcmxkIQ_3D_3D");
    }

    #[test]
    fn sanitize_filename_should_remove_initial_period() {
        let filename = ".hidden";
        let sanitized = sanitize_filename(filename);
        assert_eq!(sanitized, "_hidden");
    }
}
