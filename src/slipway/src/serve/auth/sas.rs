use actix_web::http::StatusCode;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::super::responses::ServeError;

// Create alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

const DATE_TIME_FORMAT: &str = "%Y-%m-%dT%H-%M-%S";

fn create_hmac_string(key: &str, input: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(input.as_bytes());

    let result = mac.finalize();

    bytes_to_string(&result.into_bytes())
}

pub(super) fn verify_sas_token(
    key: &str,
    expiry: &str,
    expected_signature: &str,
) -> Result<(), actix_web::Error> {
    let input = create_sas_input(expiry);
    verify_hmac_string(key, &input, expected_signature)?;

    let expiry_parsed_naive = chrono::NaiveDateTime::parse_from_str(expiry, DATE_TIME_FORMAT)
        .map_err(|e| {
            ServeError::UserFacing(
                StatusCode::BAD_REQUEST,
                format!("Invalid expiry format: {e}"),
            )
        })?;
    let expiry_parsed = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        expiry_parsed_naive,
        chrono::Utc,
    );

    let now = chrono::Utc::now();

    if now > expiry_parsed {
        return Err(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            "SAS token has expired".to_string(),
        )
        .into());
    }

    Ok(())
}

pub(crate) fn compute_signature_parts(
    key: &str,
    duration: chrono::Duration,
) -> Vec<(String, String)> {
    let now = chrono::Utc::now();
    let expiry = now + duration;

    let expiry = expiry.format(DATE_TIME_FORMAT).to_string();

    let signature = create_signature(key, &expiry);

    vec![
        (super::SHARED_ACCESS_SIGNATURE_KEY.to_string(), signature),
        (super::EXPIRY_KEY.to_string(), expiry),
    ]
}

fn create_signature(key: &str, expiry: &str) -> String {
    let input = create_sas_input(expiry);
    create_hmac_string(key, &input)
}

fn create_sas_input(expiry: &str) -> String {
    let mut input = String::new();
    input.push_str(super::EXPIRY_KEY);
    input.push('=');
    input.push_str(expiry);
    input
}

fn verify_hmac_string(
    key: &str,
    input: &str,
    expected_signature: &str,
) -> Result<(), actix_web::Error> {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(input.as_bytes());

    let expected_bytes = string_to_bytes(expected_signature).unwrap();

    // `verify_slice` will return `Ok(())` if code is correct, `Err(MacError)` otherwise
    mac.verify_slice(expected_bytes.as_slice()).map_err(|_| {
        ServeError::UserFacing(StatusCode::UNAUTHORIZED, "Invalid signature".to_string())
    })?;

    Ok(())
}

fn string_to_bytes(hex_str: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
        .collect()
}

fn bytes_to_string(input_bytes: &[u8]) -> String {
    let mut s = String::with_capacity(input_bytes.len() * 2);
    for b in input_bytes {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_able_to_convert_bytes_to_string_and_back() {
        let input = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x15, 0x25];
        let hex_string = bytes_to_string(&input);
        let output = string_to_bytes(&hex_string).unwrap();
        assert_eq!(input.to_vec(), output);
    }

    #[test]
    fn it_should_allow_valid_signatures() {
        let key = "test_key";
        let expiry = "2023-10-01T01-02-03";

        let signature = create_signature(key, expiry);
        let input = create_sas_input(expiry);

        assert!(verify_hmac_string(key, &input, &signature).is_ok());
    }

    #[test]
    fn it_should_fail_modified_expiry() {
        let key = "test_key";
        let expiry = "2023-10-01T01-02-03";
        let modified_expiry = "2023-10-02T01-02-03";

        let signature = create_signature(key, expiry);
        let input = create_sas_input(modified_expiry);

        assert!(verify_hmac_string(key, &input, &signature).is_err());
    }

    #[test]
    fn it_should_fail_modified_signature() {
        let key = "test_key";
        let expiry = "2023-10-01T01-02-03";

        let mut signature = create_signature(key, expiry);
        signature.push_str("88");

        let input = create_sas_input(expiry);

        let result = verify_hmac_string(key, &input, &signature);

        match result {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => {
                assert!(e.to_string().contains("Invalid signature"));
            }
        }
    }

    #[test]
    fn it_should_return_signature_parts_and_verify() {
        let key = "test_key";
        let duration = chrono::Duration::seconds(3600);

        let signature_parts = compute_signature_parts(key, duration);

        assert_eq!(signature_parts.len(), 2);

        let expiry = &signature_parts
            .iter()
            .find(|v| v.0 == super::super::EXPIRY_KEY)
            .unwrap()
            .1;
        let signature = &signature_parts
            .iter()
            .find(|v| v.0 == super::super::SHARED_ACCESS_SIGNATURE_KEY)
            .unwrap()
            .1;

        println!("Signature: {}", signature);
        println!("Expiry: {}", expiry);

        let result = verify_sas_token(key, expiry, signature);

        match result {
            Ok(_) => {}
            Err(e) => panic!("Expected Ok, but got error: {}", e),
        }
    }

    #[test]
    fn it_should_fail_to_verify_expired_signature() {
        let key = "test_key";
        let duration = chrono::Duration::seconds(-1);

        let signature_parts = compute_signature_parts(key, duration);

        let expiry = &signature_parts
            .iter()
            .find(|v| v.0 == super::super::EXPIRY_KEY)
            .unwrap()
            .1;
        let signature = &signature_parts
            .iter()
            .find(|v| v.0 == super::super::SHARED_ACCESS_SIGNATURE_KEY)
            .unwrap()
            .1;

        let result = verify_sas_token(key, expiry, signature);

        match result {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => {
                assert!(e.to_string().contains("SAS token has expired"));
            }
        }
    }
}
