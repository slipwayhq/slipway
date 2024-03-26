use crate::errors::AppError;
use core::fmt;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

const MAXIMUM_PUBLISHER_LENGTH: usize = 64;
const MAXIMUM_NAME_LENGTH: usize = 64;
const MAXIMUM_DESCRIPTION_LENGTH: usize = 256;
const MAXIMUM_COMPONENT_HANDLE_LENGTH: usize = 256;
const MAXIMUM_SESSION_HANDLE_LENGTH: usize = 65536;

crate::utils::create_validated_string_struct!(pub Publisher, Some(r"^\w+$"), Some(1), MAXIMUM_PUBLISHER_LENGTH);
crate::utils::create_validated_string_struct!(pub Name, Some(r"^\w+$"), Some(1), MAXIMUM_NAME_LENGTH);
crate::utils::create_validated_string_struct!(pub Description, None, None, MAXIMUM_DESCRIPTION_LENGTH);
crate::utils::create_validated_string_struct!(
    pub ComponentHandle,
    Some(r"^\w+$"),
    Some(1),
    MAXIMUM_COMPONENT_HANDLE_LENGTH
);

#[cfg(test)]
mod tests {
    use super::*;

    trait SlipwayErrorTrait {
        fn as_slipway_error(&self) -> Option<&AppError>;
    }

    impl SlipwayErrorTrait for AppError {
        fn as_slipway_error(&self) -> Option<&AppError> {
            Some(self)
        }
    }

    fn test_invalid_primitive<T: FromStr>(input: &str, expected_type: &str, expected_error: &str)
    where
        T: FromStr,
        T::Err: SlipwayErrorTrait,
    {
        match T::from_str(input) {
            Ok(_) => panic!("Should not have parsed"),
            Err(e) => {
                if let Some(AppError::InvalidSlipwayPrimitive(t, m)) = e.as_slipway_error() {
                    assert_eq!(t, expected_type);
                    assert!(
                        m.starts_with(expected_error),
                        "Expected error to start with \"{}\" but it was \"{}\"",
                        expected_error,
                        m
                    );
                } else {
                    panic!("Expected a InvalidSlipwayPrimitive error");
                }
            }
        }
    }

    mod publisher_tests {
        use crate::test_utils::quote;

        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_publisher() {
            let s = r"test_publisher";
            let json = quote(s);
            let id: Publisher = serde_json::from_str(&json).unwrap();
            let json_out = serde_json::to_string(&id).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_publisher_from_string() {
            let s = r"test_publisher";
            let id = Publisher::from_str(s).unwrap();
            assert_eq!(id.0, s);
        }

        #[test]
        fn it_should_fail_to_parse_publisher_with_hyphens() {
            test_invalid_primitive::<Publisher>(
                "test-publisher",
                "Publisher",
                "Publisher does not match the required format",
            );
        }

        #[test]
        fn it_should_fail_to_parse_too_short_publisher() {
            test_invalid_primitive::<Publisher>(
                "",
                "Publisher",
                "Publisher is shorter than the minimum length",
            );
        }

        #[test]
        fn it_should_fail_to_parse_too_long_publisher() {
            test_invalid_primitive::<Publisher>(
                'a'.to_string()
                    .repeat(MAXIMUM_PUBLISHER_LENGTH + 1)
                    .as_str(),
                "Publisher",
                "Publisher is longer than the maximum length",
            );
        }
    }

    mod name_tests {
        use crate::test_utils::quote;

        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_name() {
            let s = r"test_name";
            let json = quote(s);
            let id: Name = serde_json::from_str(&json).unwrap();
            let json_out = serde_json::to_string(&id).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_name_from_string() {
            let s = r"test_name";
            let id = Name::from_str(s).unwrap();
            assert_eq!(id.0, s);
        }

        #[test]
        fn it_should_fail_to_parse_name_with_hyphens() {
            test_invalid_primitive::<Name>(
                "test-name",
                "Name",
                "Name does not match the required format",
            );
        }

        #[test]
        fn it_should_fail_to_parse_too_short_name() {
            test_invalid_primitive::<Name>("", "Name", "Name is shorter than the minimum length");
        }

        #[test]
        fn it_should_fail_to_parse_too_long_name() {
            test_invalid_primitive::<Name>(
                'a'.to_string().repeat(MAXIMUM_NAME_LENGTH + 1).as_str(),
                "Name",
                "Name is longer than the maximum length",
            );
        }
    }

    mod description_tests {
        use crate::test_utils::quote;

        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_description() {
            let s = r"the quick brown fox jumps over the lazy-dog.";
            let json = quote(s);
            let id: Description = serde_json::from_str(&json).unwrap();
            let json_out = serde_json::to_string(&id).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_description_from_string() {
            let s = r"the quick! brown fox jumps over the lazy-dog.";
            let id = Description::from_str(s).unwrap();
            assert_eq!(id.0, s);
        }

        #[test]
        fn it_should_parse_empty_description() {
            let s = r"";
            let id = Description::from_str(s).unwrap();
            assert_eq!(id.0, s);
        }

        #[test]
        fn it_should_fail_to_parse_too_long_description() {
            test_invalid_primitive::<Description>(
                'a'.to_string()
                    .repeat(MAXIMUM_DESCRIPTION_LENGTH + 1)
                    .as_str(),
                "Description",
                "Description is longer than the maximum length",
            );
        }
    }

    mod component_handle_tests {
        use crate::test_utils::quote;

        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_component_handle() {
            let s = r"test_component_handle";
            let json = quote(s);
            let id: ComponentHandle = serde_json::from_str(&json).unwrap();
            let json_out = serde_json::to_string(&id).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_component_handle_from_string() {
            let s = r"test_component_handle";
            let id = ComponentHandle::from_str(s).unwrap();
            assert_eq!(id.0, s);
        }

        #[test]
        fn it_should_fail_to_parse_component_handle_with_hyphens() {
            test_invalid_primitive::<ComponentHandle>(
                "test-component_handle",
                "ComponentHandle",
                "ComponentHandle does not match the required format",
            );
        }

        #[test]
        fn it_should_fail_to_parse_too_short_component_handle() {
            test_invalid_primitive::<ComponentHandle>(
                "",
                "ComponentHandle",
                "ComponentHandle is shorter than the minimum length",
            );
        }

        #[test]
        fn it_should_fail_to_parse_too_long_component_handle() {
            test_invalid_primitive::<ComponentHandle>(
                'a'.to_string()
                    .repeat(MAXIMUM_COMPONENT_HANDLE_LENGTH + 1)
                    .as_str(),
                "ComponentHandle",
                "ComponentHandle is longer than the maximum length",
            );
        }
    }
}
