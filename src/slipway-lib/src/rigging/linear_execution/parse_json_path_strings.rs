use std::str::FromStr;

use jsonpath_rust::JsonPathInst;

use crate::errors::SlipwayError;

use super::find_json_path_strings::FoundJsonPathString;

const JSON_PATH_SOURCE_GENERATED: &str = "generated";
const JSON_PATH_SOURCE_EXTERNAL_PREFIX: &str = "at location \"";
const JSON_PATH_SOURCE_EXTERNAL_SUFFIX: &str = "\"";

pub(crate) struct FoundJsonPath {
    pub path_to: JsonPathInst,
    pub path: JsonPathInst,
}

impl FoundJsonPathString<'_> {
    pub(crate) fn parse(&self) -> Result<FoundJsonPath, SlipwayError> {
        let path_to = JsonPathInst::from_str(&self.path_to).map_err(|e| {
            SlipwayError::InvalidJsonPathExpression(JSON_PATH_SOURCE_GENERATED.to_string(), e)
        })?;
        let path = JsonPathInst::from_str(&self.path).map_err(|e| {
            SlipwayError::InvalidJsonPathExpression(
                format!(
                    "{JSON_PATH_SOURCE_EXTERNAL_PREFIX}{0}{JSON_PATH_SOURCE_EXTERNAL_SUFFIX}",
                    self.path_to
                ),
                e,
            )
        })?;
        Ok(FoundJsonPath { path_to, path })
    }
}

pub(crate) trait Parse {
    fn parse(&self) -> Result<Vec<FoundJsonPath>, SlipwayError>;
}

impl Parse for Vec<FoundJsonPathString<'_>> {
    fn parse(&self) -> Result<Vec<FoundJsonPath>, SlipwayError> {
        self.iter()
            .map(|found_json_path_strings| found_json_path_strings.parse())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn it_should_parse_valid_json_path_strings() {
        let found_json_path_strings = vec![
            FoundJsonPathString {
                path_to: "$.key1".to_string(),
                path: Cow::Borrowed("$[0]"),
            },
            FoundJsonPathString {
                path_to: "$..book[?(@.author ~= '(?i)foo')]".to_string(),
                path: Cow::Borrowed("$..book[?(@.price <= $.expensive)]"),
            },
        ];

        let found_json_paths = found_json_path_strings.parse().unwrap();

        assert_eq!(found_json_paths.len(), found_json_path_strings.len());
    }

    #[test]
    fn should_fail_to_parse_invalid_path_to() {
        let found_json_path_strings = vec![FoundJsonPathString {
            path_to: "foo".to_string(),
            path: Cow::Borrowed("$.value1"),
        }];

        let found_json_paths = found_json_path_strings.parse();

        match found_json_paths {
            Err(SlipwayError::InvalidJsonPathExpression(s, e)) => {
                assert_eq!(s, JSON_PATH_SOURCE_GENERATED);
                assert!(e.contains("foo"))
            }
            _ => panic!("Expected InvalidJsonPathExpression error"),
        }
    }

    #[test]
    fn should_fail_to_parse_invalid_path_found() {
        let found_json_path_strings = vec![FoundJsonPathString {
            path_to: "$.key1".to_string(),
            path: Cow::Borrowed("$.foo.blah[0[.bar"),
        }];

        let found_json_paths = found_json_path_strings.parse();

        match found_json_paths {
            Err(SlipwayError::InvalidJsonPathExpression(s, e)) => {
                assert!(s.contains("\"$.key1\""));
                assert!(e.contains("$.foo.blah[0[.bar"));
            }
            _ => panic!("Expected InvalidJsonPathExpression error"),
        }
    }
}
